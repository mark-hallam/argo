//! `argo forge optimize` — score-driven candidate sweep. Iter 0
//! benches the baseline active skill; iters 1..N mutate the body,
//! supersede the active row with the candidate, run the bench
//! against the same examples, and record the score. The
//! highest-scoring version is promoted to active so the next
//! agent turn picks it up. Leaf and stage helpers live in
//! `forge_optimize_helpers.rs`.
use super::forge_mutator::mutate_via_llm;
use super::forge_optimize_helpers::{
    build_skill_md, bump_version, check_iter_budget, emit_event, evaluate_promote_decision,
    mechanical_refinement, report_summary,
};
use super::forge_optimize_judge::PromoteDecision;
use super::forge_subject::{open_store, BenchSubject};
use crate::home::resolve_home;
use argo_audit::{AuditKind, AuditSink};
use argo_eval::{load_jsonl, run_eval, EvalRecord};
use argo_judge::Judge;
use argo_skill::{PrincipalOverride, SkillStore};
use argo_types::{ArgoError, ArgoResult};
use std::path::PathBuf;
use std::sync::Arc;

pub(crate) use super::forge_optimize_helpers::spawn_optimize;

pub(super) async fn run(
    skill_name: &str,
    examples_path: &str,
    iterations: u32,
    budget_usd: f64,
    stream: bool,
) -> ArgoResult<()> {
    run_with_events(skill_name, examples_path, iterations, budget_usd, stream, None).await
}

/// Like [`run`] but every progress event is forwarded to the
/// supplied channel instead of stdout. Used by the gateway's
/// `POST /forge/optimize/stream` so HTTP clients can tail the
/// run live. When `events_tx` is `None` the function is
/// equivalent to `run(...)`.
pub(crate) async fn run_with_events(
    skill_name: &str,
    examples_path: &str,
    iterations: u32,
    budget_usd: f64,
    stream: bool,
    events_tx: Option<tokio::sync::mpsc::UnboundedSender<serde_json::Value>>,
) -> ArgoResult<()> {
    run_with_events_and_run_id(
        skill_name,
        examples_path,
        iterations,
        budget_usd,
        stream,
        events_tx,
        None,
        None,
    )
    .await
}

/// `argo forge daemon`-side entry point. Uses the existing
/// run_id (already flipped to 'running' by
/// `claim_queued_forge_run`) instead of inserting a fresh
/// row. Same execution path as [`run`] otherwise — re-uses
/// the iteration/budget/audit chain logic.
pub(crate) async fn run_claimed(
    skill_name: &str,
    examples_path: &str,
    iterations: u32,
    budget_usd: f64,
    run_id: uuid::Uuid,
) -> ArgoResult<()> {
    run_with_events_and_run_id(
        skill_name,
        examples_path,
        iterations,
        budget_usd,
        false,
        None,
        Some(run_id),
        None,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn run_with_events_and_run_id(
    skill_name: &str,
    examples_path: &str,
    iterations: u32,
    budget_usd: f64,
    stream: bool,
    events_tx: Option<tokio::sync::mpsc::UnboundedSender<serde_json::Value>>,
    existing_run_id: Option<uuid::Uuid>,
    judge: Option<&dyn Judge>,
) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
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
    // Channel-or-stdout sink; the channel suppresses the
    // human-readable banners + table.
    let dispatch = |payload: serde_json::Value| {
        if let Some(tx) = events_tx.as_ref() {
            let _ = tx.send(payload);
        } else if stream {
            emit_event(&payload);
        }
    };
    let interactive_text = !stream && events_tx.is_none();
    if interactive_text {
        println!(
            "argo forge optimize: {skill_name} v{} → {} iters × {} task(s); budget ${budget_usd:.2}",
            baseline.frontmatter.version,
            iterations,
            tasks.len()
        );
    }

    let ws_name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let ws_root = cwd.to_string_lossy();
    let (tenant, workspace) = store.ensure_workspace(ws_name, &ws_root).await?;
    let run_id = match existing_run_id {
        Some(id) => id,
        None => {
            store
                .start_forge_run(
                    skill_name,
                    &baseline.frontmatter.version,
                    tasks.len() as u32,
                )
                .await?
        }
    };
    dispatch(serde_json::json!({
        "event": "start",
        "run_id": run_id.to_string(),
        "skill_name": skill_name,
        "base_version": baseline.frontmatter.version,
        "iterations": iterations,
        "examples": examples_path,
        "n_tasks": tasks.len(),
    }));
    let _ = AuditSink::new(Arc::clone(&store))
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
        )
        .await;

    let subject = BenchSubject {
        store: Arc::clone(&store),
        cwd: cwd.clone(),
        tenant,
        workspace,
        provider: "anthropic".to_string(),
        model: "claude-3-5-sonnet-20241022".to_string(),
        skill_name: skill_name.to_string(),
    };

    let mut best_score: f64 = f64::NEG_INFINITY;
    let mut best_version = baseline.frontmatter.version.clone();
    let mut completed = 0_i64;
    let mut last_status: &'static str = "succeeded";
    let budget_micros = (budget_usd * 1_000_000.0).round() as i64;
    // Records from the previous iter — used to drive the next
    // mutation by feeding the meta-prompt the failed examples.
    let mut prev_records: Vec<EvalRecord> = Vec::new();

    if interactive_text {
        println!("\n  iter  version    mean_score  pass_rate  errors  best_so_far");
    }
    for iter in 0..iterations {
        let version_for_iter = if iter == 0 {
            baseline.frontmatter.version.clone()
        } else {
            let candidate_version = bump_version(&baseline.frontmatter.version, iter);
            // Use the LLM mutator if there were failures last
            // iter; otherwise fall back to the mechanical
            // refinement template (so we still try *something*
            // when everything passed but score was sub-1.0).
            let candidate_body = match mutate_via_llm(
                &cwd,
                &subject.provider,
                &subject.model,
                &baseline.body,
                &tasks,
                &prev_records,
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
            candidate_version
        };

        let (records, summary) = run_eval(&subject, &tasks).await;
        prev_records = records;
        if summary.errors > 0 {
            last_status = if summary.errors == summary.n {
                "failed"
            } else {
                "partial"
            };
        }
        if summary.mean_score > best_score {
            best_score = summary.mean_score;
            best_version = version_for_iter.clone();
        }
        completed += 1;
        if interactive_text {
            println!(
                "  {iter:>3}   {ver:<9}  {mean:.3}      {pr:.3}     {errs:<6}  {best:.3}",
                iter = iter,
                ver = version_for_iter,
                mean = summary.mean_score,
                pr = summary.pass_rate,
                errs = summary.errors,
                best = best_score,
            );
        } else {
            dispatch(serde_json::json!({
                "event": "iter",
                "run_id": run_id.to_string(),
                "iter": iter,
                "version": version_for_iter,
                "mean_score": summary.mean_score,
                "pass_rate": summary.pass_rate,
                "errors": summary.errors,
                "best_so_far": best_score,
            }));
        }
        // Push mid-run progress into the forge_run row so the
        // TUI's F-overlay auto-refresh shows the iter count
        // climbing + the best-so-far score moving while the
        // loop is still in flight. The WHERE clause guards
        // against clobbering an already-aborted row.
        let best_so_far = best_score.is_finite().then_some(best_score);
        let _ = store
            .update_forge_run_progress(run_id, completed, best_so_far, Some(&version_for_iter))
            .await;

        if let Some(status) = check_iter_budget(
            &store,
            run_id,
            iter,
            budget_micros,
            budget_usd,
            interactive_text,
            events_tx.as_ref(),
            stream,
        )
        .await
        {
            last_status = status;
            break;
        }
    }

    let (promote_decision, last_status) = evaluate_promote_decision(
        &skills,
        &store,
        &baseline,
        skill_name,
        &best_version,
        best_score,
        iterations,
        run_id,
        judge,
        last_status,
    )
    .await;
    let promoted = matches!(promote_decision, PromoteDecision::Allow);

    let final_score = best_score.is_finite().then_some(best_score);
    let _ = store
        .finish_forge_run(run_id, last_status, final_score, completed)
        .await;
    let _ = AuditSink::new(Arc::clone(&store))
        .write(
            None,
            AuditKind::ForgeRunEnd,
            serde_json::json!({
                "run_id": run_id.to_string(),
                "status": last_status,
                "best_score": final_score,
                "best_version": best_version,
                "iterations_run": completed,
                "promoted": promoted,
            }),
        )
        .await;
    report_summary(
        interactive_text,
        skill_name,
        last_status,
        best_score,
        &best_version,
        completed,
        iterations,
        final_score,
        promoted,
        run_id,
        events_tx.as_ref(),
        stream,
    );
    Ok(())
}
