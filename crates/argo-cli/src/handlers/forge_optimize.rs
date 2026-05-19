//! `argo forge optimize` — score-driven candidate sweep. Iter 0
//! benches the baseline active skill; iters 1..N mutate the body,
//! supersede the active row with the candidate, run the bench
//! against the same examples, and record the score. The
//! highest-scoring version is promoted to active so the next
//! agent turn picks it up. Setup / per-iter / finalize helpers
//! live in `forge_optimize_loop.rs`; leaf math + dispatch
//! helpers live in `forge_optimize_helpers.rs`.
use super::forge_optimize_loop::{
    finalize_optimize_run, prepare_optimize_run, run_one_iter, OptimizeState,
};
use super::forge_subject::BenchSubject;
use argo_judge::Judge;
use argo_types::ArgoResult;
use std::sync::Arc;

pub(crate) use super::forge_optimize_helpers::spawn_optimize;

pub(super) async fn run(
    skill_name: &str,
    examples_path: &str,
    iterations: u32,
    budget_usd: f64,
    stream: bool,
) -> ArgoResult<()> {
    run_with_events(
        skill_name,
        examples_path,
        iterations,
        budget_usd,
        stream,
        None,
    )
    .await
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
    let (store, skills, baseline, tasks, cwd, tenant, workspace, run_id, interactive_text) =
        prepare_optimize_run(
            skill_name,
            examples_path,
            iterations,
            budget_usd,
            stream,
            events_tx.as_ref(),
            existing_run_id,
        )
        .await?;
    let subject = BenchSubject {
        store: Arc::clone(&store),
        cwd: cwd.clone(),
        tenant,
        workspace,
        provider: "anthropic".to_string(),
        model: "claude-3-5-sonnet-20241022".to_string(),
        skill_name: skill_name.to_string(),
    };
    let budget_micros = (budget_usd * 1_000_000.0).round() as i64;
    let mut state = OptimizeState::new(baseline.frontmatter.version.clone());
    if interactive_text {
        println!("\n  iter  version    mean_score  pass_rate  errors  best_so_far");
    }
    for iter in 0..iterations {
        if let Some(stop) = run_one_iter(
            iter,
            skill_name,
            &cwd,
            &baseline,
            &tasks,
            &subject,
            &skills,
            &store,
            run_id,
            interactive_text,
            events_tx.as_ref(),
            stream,
            budget_micros,
            budget_usd,
            &mut state,
        )
        .await?
        {
            state.last_status = stop;
            break;
        }
    }
    finalize_optimize_run(
        skill_name,
        &skills,
        &store,
        &baseline,
        run_id,
        iterations,
        tenant,
        judge,
        interactive_text,
        events_tx.as_ref(),
        stream,
        state,
    )
    .await;
    Ok(())
}
