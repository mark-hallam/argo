//! Iteration loop helpers for `argo forge optimize`. Split out of
//! `forge_optimize.rs` so each piece (setup, per-iter step,
//! finalize) fits the function-LOC budget. The orchestrator in
//! `forge_optimize::run_with_events_and_run_id` is the only
//! caller.
use super::forge_mutator::mutate_via_llm;
use super::forge_optimize_helpers::{
    build_skill_md, bump_version, check_iter_budget, emit_event, evaluate_promote_decision,
    mechanical_refinement, report_summary,
};
use super::forge_optimize_judge::PromoteDecision;
use super::forge_subject::{open_store, BenchSubject};
use crate::home::resolve_home;
use argo_audit::{AuditKind, AuditSink};
use argo_eval::{load_jsonl, run_eval, EvalRecord, EvalTask};
use argo_judge::Judge;
use argo_skill::{PrincipalOverride, Skill, SkillStore};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId, WorkspaceId};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Mutable counters threaded through the iter loop. The struct
/// stays small (5 fields) deliberately — keeps the helper
/// signatures from sprawling without paying for an indirection
/// every iteration.
pub(super) struct OptimizeState {
    pub(super) best_score: f64,
    pub(super) best_version: String,
    pub(super) completed: i64,
    pub(super) last_status: &'static str,
    pub(super) prev_records: Vec<EvalRecord>,
}

impl OptimizeState {
    pub(super) fn new(baseline_version: String) -> Self {
        Self {
            best_score: f64::NEG_INFINITY,
            best_version: baseline_version,
            completed: 0,
            last_status: "succeeded",
            prev_records: Vec::new(),
        }
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(super) async fn prepare_optimize_run(
    skill_name: &str,
    examples_path: &str,
    iterations: u32,
    budget_usd: f64,
    stream: bool,
    events_tx: Option<&tokio::sync::mpsc::UnboundedSender<serde_json::Value>>,
    existing_run_id: Option<uuid::Uuid>,
) -> ArgoResult<(
    Arc<SessionStore>,
    SkillStore,
    Skill,
    Vec<EvalTask>,
    PathBuf,
    TenantId,
    WorkspaceId,
    uuid::Uuid,
    bool,
)> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let _home = resolve_home(&cwd)?;
    let store = Arc::new(open_store().await?);
    let skills = SkillStore::new(Arc::clone(&store));
    let baseline = skills
        .find_active_by_name(skill_name)
        .await?
        .ok_or_else(|| ArgoError::Provider(format!("no active skill named '{skill_name}'")))?;
    let tasks = load_jsonl(&PathBuf::from(examples_path)).await?;
    if tasks.is_empty() {
        return Err(ArgoError::Provider(format!(
            "no eval tasks parsed from {examples_path}"
        )));
    }
    let interactive_text = !stream && events_tx.is_none();
    if interactive_text {
        println!(
            "argo forge optimize: {skill_name} v{} → {} iters × {} task(s); budget ${budget_usd:.2}",
            baseline.frontmatter.version,
            iterations,
            tasks.len()
        );
    }
    let (tenant, workspace, run_id) = open_forge_run_record(
        &store,
        skill_name,
        examples_path,
        iterations,
        budget_usd,
        events_tx,
        stream,
        existing_run_id,
        &baseline,
        &cwd,
        tasks.len(),
    )
    .await?;
    Ok((
        store,
        skills,
        baseline,
        tasks,
        cwd,
        tenant,
        workspace,
        run_id,
        interactive_text,
    ))
}

#[allow(clippy::too_many_arguments)]
async fn open_forge_run_record(
    store: &Arc<SessionStore>,
    skill_name: &str,
    examples_path: &str,
    iterations: u32,
    budget_usd: f64,
    events_tx: Option<&tokio::sync::mpsc::UnboundedSender<serde_json::Value>>,
    stream: bool,
    existing_run_id: Option<uuid::Uuid>,
    baseline: &Skill,
    cwd: &Path,
    n_tasks: usize,
) -> ArgoResult<(TenantId, WorkspaceId, uuid::Uuid)> {
    let (tenant, workspace) = store
        .ensure_workspace(workspace_name(cwd), &cwd.to_string_lossy())
        .await?;
    let run_id = match existing_run_id {
        Some(id) => id,
        None => {
            store
                .start_forge_run(skill_name, &baseline.frontmatter.version, n_tasks as u32)
                .await?
        }
    };
    dispatch_event(
        events_tx,
        stream,
        serde_json::json!({
            "event": "start",
            "run_id": run_id.to_string(),
            "skill_name": skill_name,
            "base_version": baseline.frontmatter.version,
            "iterations": iterations,
            "examples": examples_path,
            "n_tasks": n_tasks,
        }),
    );
    let _ = AuditSink::new_with_tenant(Arc::clone(store), tenant.to_string())
        .write(
            None,
            AuditKind::ForgeRunStart,
            serde_json::json!({
                "run_id": run_id.to_string(),
                "skill_name": skill_name,
                "examples": examples_path,
                "iterations": iterations,
                "budget_usd": budget_usd,
            }),
            None,
        )
        .await;
    Ok((tenant, workspace, run_id))
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn run_one_iter(
    iter: u32,
    skill_name: &str,
    cwd: &Path,
    baseline: &Skill,
    tasks: &[EvalTask],
    subject: &BenchSubject,
    skills: &SkillStore,
    store: &Arc<SessionStore>,
    run_id: uuid::Uuid,
    interactive_text: bool,
    events_tx: Option<&tokio::sync::mpsc::UnboundedSender<serde_json::Value>>,
    stream: bool,
    budget_micros: i64,
    budget_usd: f64,
    state: &mut OptimizeState,
) -> ArgoResult<Option<&'static str>> {
    let version_for_iter =
        prepare_candidate(iter, skill_name, cwd, baseline, tasks, subject, skills, state).await?;
    let (records, summary) = run_eval(subject, tasks).await;
    state.prev_records = records;
    if summary.errors > 0 {
        state.last_status = if summary.errors == summary.n {
            "failed"
        } else {
            "partial"
        };
    }
    if summary.mean_score > state.best_score {
        state.best_score = summary.mean_score;
        state.best_version = version_for_iter.clone();
    }
    state.completed += 1;
    if interactive_text {
        println!(
            "  {iter:>3}   {ver:<9}  {mean:.3}      {pr:.3}     {errs:<6}  {best:.3}",
            iter = iter,
            ver = version_for_iter,
            mean = summary.mean_score,
            pr = summary.pass_rate,
            errs = summary.errors,
            best = state.best_score,
        );
    } else {
        dispatch_event(
            events_tx,
            stream,
            serde_json::json!({
                "event": "iter",
                "run_id": run_id.to_string(),
                "iter": iter,
                "version": version_for_iter,
                "mean_score": summary.mean_score,
                "pass_rate": summary.pass_rate,
                "errors": summary.errors,
                "best_so_far": state.best_score,
            }),
        );
    }
    let best_so_far = state.best_score.is_finite().then_some(state.best_score);
    let _ = store
        .update_forge_run_progress(run_id, state.completed, best_so_far, Some(&version_for_iter))
        .await;
    Ok(check_iter_budget(
        store,
        run_id,
        iter,
        budget_micros,
        budget_usd,
        interactive_text,
        events_tx,
        stream,
    )
    .await)
}

#[allow(clippy::too_many_arguments)]
async fn prepare_candidate(
    iter: u32,
    skill_name: &str,
    cwd: &Path,
    baseline: &Skill,
    tasks: &[EvalTask],
    subject: &BenchSubject,
    skills: &SkillStore,
    state: &OptimizeState,
) -> ArgoResult<String> {
    if iter == 0 {
        return Ok(baseline.frontmatter.version.clone());
    }
    let candidate_version = bump_version(&baseline.frontmatter.version, iter);
    // Use the LLM mutator if there were failures last iter;
    // otherwise fall back to the mechanical refinement template
    // (so we still try *something* when everything passed but
    // score was sub-1.0).
    let candidate_body = match mutate_via_llm(
        cwd,
        &subject.provider,
        &subject.model,
        &baseline.body,
        tasks,
        &state.prev_records,
    )
    .await
    {
        Ok(Some(body)) => body,
        Ok(None) | Err(_) => mechanical_refinement(&baseline.body, iter),
    };
    let raw = build_skill_md(
        skill_name,
        &candidate_version,
        &baseline.frontmatter.description,
        &candidate_body,
    );
    skills
        .supersede(&raw, None, PrincipalOverride::Force)
        .await?;
    Ok(candidate_version)
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn finalize_optimize_run(
    skill_name: &str,
    skills: &SkillStore,
    store: &Arc<SessionStore>,
    baseline: &Skill,
    run_id: uuid::Uuid,
    iterations: u32,
    tenant: TenantId,
    judge: Option<&dyn Judge>,
    interactive_text: bool,
    events_tx: Option<&tokio::sync::mpsc::UnboundedSender<serde_json::Value>>,
    stream: bool,
    state: OptimizeState,
) {
    let (promote_decision, last_status) = evaluate_promote_decision(
        skills,
        store,
        baseline,
        skill_name,
        &state.best_version,
        state.best_score,
        iterations,
        run_id,
        judge,
        state.last_status,
    )
    .await;
    let promoted = matches!(promote_decision, PromoteDecision::Allow);
    let final_score = state.best_score.is_finite().then_some(state.best_score);
    let _ = store
        .finish_forge_run(run_id, last_status, final_score, state.completed)
        .await;
    let _ = AuditSink::new_with_tenant(Arc::clone(store), tenant.to_string())
        .write(
            None,
            AuditKind::ForgeRunEnd,
            serde_json::json!({
                "run_id": run_id.to_string(),
                "status": last_status,
                "best_score": final_score,
                "best_version": state.best_version,
                "iterations_run": state.completed,
                "promoted": promoted,
            }),
            None,
        )
        .await;
    report_summary(
        interactive_text,
        skill_name,
        last_status,
        state.best_score,
        &state.best_version,
        state.completed,
        iterations,
        final_score,
        promoted,
        run_id,
        events_tx,
        stream,
    );
}

fn workspace_name(cwd: &Path) -> &str {
    cwd.file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace")
}

fn dispatch_event(
    events_tx: Option<&tokio::sync::mpsc::UnboundedSender<serde_json::Value>>,
    stream: bool,
    payload: serde_json::Value,
) {
    if let Some(tx) = events_tx {
        let _ = tx.send(payload);
    } else if stream {
        emit_event(&payload);
    }
}
