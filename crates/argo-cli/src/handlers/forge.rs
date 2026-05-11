//! `argo forge` — runs list / show / abort + promote / bench /
//! optimize. Optimize lives in `forge_optimize.rs` so this file
//! stays under 500 LOC; the shared `BenchSubject` +
//! `open_store` helpers live in `forge_subject.rs`. Runs
//! count/list rendering lives in `forge_runs_list.rs`.
use super::forge_optimize;
use super::forge_runs_list;
use super::forge_subject::{open_store, BenchSubject};
use crate::cli::{ForgeCmd, ForgeRunsCmd};
use argo_audit::{AuditKind, AuditSink};
use argo_eval::{load_jsonl, run_eval};
use argo_skill::SkillStore;
use argo_types::{ArgoError, ArgoResult};
use std::path::PathBuf;
use std::sync::Arc;

pub async fn run(cmd: ForgeCmd) -> ArgoResult<()> {
    match cmd {
        ForgeCmd::Runs { cmd } => match cmd {
            ForgeRunsCmd::Count {
                status,
                since_minutes,
                json,
            } => forge_runs_list::count_runs(status, since_minutes, json).await,
            ForgeRunsCmd::List {
                limit,
                json,
                format,
                since_minutes,
                status,
                score_min,
                score_max,
                skill,
                has_candidate,
                iterations_min,
                iterations_max,
                sort_by,
            } => {
                forge_runs_list::list_runs(
                    limit,
                    json,
                    format,
                    since_minutes,
                    status,
                    score_min,
                    score_max,
                    skill,
                    has_candidate,
                    iterations_min,
                    iterations_max,
                    sort_by,
                )
                .await
            }
            ForgeRunsCmd::Show { id } => super::forge_admin::show_run(&id).await,
            ForgeRunsCmd::Abort { id } => super::forge_admin::abort_run(&id).await,
            ForgeRunsCmd::AbortAll => super::forge_admin::abort_all_runs().await,
            ForgeRunsCmd::Sweep { older_than_minutes } => {
                super::forge_admin::sweep_stalled(older_than_minutes).await
            }
            ForgeRunsCmd::ByDay { days, json } => {
                super::forge_by_day::run(days, json).await
            }
            ForgeRunsCmd::ByStatus { since_minutes, json } => {
                super::forge_by_status::run(since_minutes, json).await
            }
            ForgeRunsCmd::BySkill { since_minutes, json } => {
                super::forge_by_skill::run(since_minutes, json).await
            }
            ForgeRunsCmd::Requeue { id, json } => {
                super::forge_admin::requeue_run(&id, json).await
            }
            ForgeRunsCmd::Payload { id, json } => {
                super::forge_admin::show_payload(&id, json).await
            }
            ForgeRunsCmd::Audit { id, json } => {
                super::forge_admin::show_audit(&id, json).await
            }
        },
        ForgeCmd::Promote { skill, version } => promote(&skill, &version).await,
        ForgeCmd::Bench {
            skill,
            examples,
            provider,
            model,
        } => bench(&skill, &examples, &provider, &model).await,
        ForgeCmd::Optimize {
            skill,
            examples,
            iterations,
            budget,
            stream,
        } => forge_optimize::run(&skill, &examples, iterations, budget, stream).await,
        ForgeCmd::Daemon {
            interval_secs,
            older_than_minutes,
            max_ticks,
        } => super::forge_daemon::run(interval_secs, older_than_minutes, max_ticks).await,
        ForgeCmd::Enqueue {
            skill,
            examples,
            iterations,
            budget,
            json,
        } => {
            super::forge_enqueue::run(&skill, &examples, iterations, budget, json).await
        }
        ForgeCmd::CostReport {
            skill,
            since,
            json,
        } => super::forge_cost_report::run(skill, since, json).await,
    }
}

async fn bench(
    skill_name: &str,
    examples_path: &str,
    provider: &str,
    model: &str,
) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let store = Arc::new(open_store().await?);
    let skills = SkillStore::new(Arc::clone(&store));

    // Resolve skill (lookup is informational — body isn't yet
    // injected into the prompt; that lands with the skill-loader
    // PR).
    let active = skills
        .find_active_by_name(skill_name)
        .await?
        .ok_or_else(|| ArgoError::Provider(format!("no active skill named '{skill_name}'")))?;

    let tasks = load_jsonl(&PathBuf::from(examples_path)).await?;
    if tasks.is_empty() {
        return Err(ArgoError::Provider(format!(
            "no eval tasks parsed from {examples_path}"
        )));
    }
    println!(
        "argo forge bench: {skill_name} v{} → {} task(s) from {examples_path}",
        active.frontmatter.version,
        tasks.len()
    );

    // Bootstrap the workspace bookkeeping so each bench session
    // can claim a tenant + workspace row.
    let ws_name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let ws_root = cwd.to_string_lossy();
    let (tenant, workspace) = store.ensure_workspace(ws_name, &ws_root).await?;

    let run_id = store
        .start_forge_run(
            skill_name,
            &active.frontmatter.version,
            tasks.len() as u32,
        )
        .await?;
    let _ = AuditSink::new(Arc::clone(&store))
        .write(
            None,
            AuditKind::ForgeRunStart,
            serde_json::json!({
                "run_id": run_id.to_string(),
                "skill_name": skill_name,
                "examples": examples_path,
                "n": tasks.len(),
            }),
            None,
        )
        .await;

    let subject = BenchSubject {
        store: Arc::clone(&store),
        cwd: cwd.clone(),
        tenant,
        workspace,
        provider: provider.to_string(),
        model: model.to_string(),
        skill_name: skill_name.to_string(),
    };
    let (records, summary) = run_eval(&subject, &tasks).await;

    let status = if summary.errors > 0 && summary.errors == summary.n {
        "failed"
    } else if summary.errors > 0 {
        "partial"
    } else {
        "succeeded"
    };
    let _ = store
        .finish_forge_run(run_id, status, Some(summary.mean_score), summary.n as i64)
        .await;
    let _ = AuditSink::new(Arc::clone(&store))
        .write(
            None,
            AuditKind::ForgeRunEnd,
            serde_json::json!({
                "run_id": run_id.to_string(),
                "status": status,
                "mean_score": summary.mean_score,
                "pass_rate": summary.pass_rate,
                "errors": summary.errors,
            }),
            None,
        )
        .await;

    println!("\n  task                     score  passed  output (truncated)");
    for r in &records {
        let pass_tag = if r.passed { "yes" } else { "no" };
        let preview: String = r.output.chars().take(48).collect();
        println!(
            "  {name:<24} {score:.3}  {pass:<6}  {preview}",
            name = r.task,
            score = r.score,
            pass = pass_tag,
        );
    }
    println!(
        "\nsummary: n={} mean_score={:.3} pass_rate={:.3} errors={} (run_id={})",
        summary.n, summary.mean_score, summary.pass_rate, summary.errors, run_id
    );
    Ok(())
}


/// Async kick-off used by the gateway's `POST /forge/bench`.
/// Spawns a Tokio task that runs the bench; the resulting
/// forge_run row shows up in `GET /forge/runs`. Errors land in
/// the gateway log — the caller already got 202 Accepted.
pub(crate) fn spawn_bench(skill: String, examples: String, provider: String, model: String) {
    tokio::spawn(async move {
        if let Err(e) = bench(&skill, &examples, &provider, &model).await {
            eprintln!("argo gateway: forge bench ({skill}) failed: {e}");
        }
    });
}

async fn promote(skill_name: &str, version: &str) -> ArgoResult<()> {
    let store = Arc::new(open_store().await?);
    let skills = SkillStore::new(Arc::clone(&store));
    let promoted = skills.promote(skill_name, version).await?;
    AuditSink::new(Arc::clone(&store))
        .write(
            None,
            AuditKind::ForgeRunEnd,
            serde_json::json!({
                "kind": "promote",
                "skill_name": skill_name,
                "version": version,
                "skill_id": promoted.id.0.to_string(),
                "by": "cli",
            }),
            None,
        )
        .await?;
    println!(
        "argo forge promote: {} → version {} is now active (skill_id {})",
        skill_name, version, promoted.id.0
    );
    Ok(())
}
