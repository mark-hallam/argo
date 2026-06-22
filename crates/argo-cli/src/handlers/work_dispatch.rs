//! Per-job dispatch for `argo work serve`.
//!
//! Extracted from `handlers/work.rs` to keep the file under the
//! 500-LOC ceiling. `serve()` calls `process_job` once per claimed
//! `pending_jobs` row; this file orchestrates the linear phase
//! sequence (parse → session+bundle → step run → artifacts → action
//! emission → completed_jobs append → stdout emission). Phase
//! helpers live in [`super::work_dispatch_phases`] so each phase
//! stays well under the 80-LOC ceiling.

use argo_memory::{MemoryEntry, MemoryStore};
use argo_store::{SessionStore, WorkerId};
use argo_types::{ArgoResult, TenantId, WorkspaceId};
use chrono::Utc;
use std::sync::Arc;

/// Operator pre-approved tools for a single run (the "Approve & run"
/// keystone fix). When present, [`process_job`] mints ONE-SHOT session
/// grants for each named tool on the freshly created session BEFORE the
/// turn executes — so a probationary teammate's turn runs with the
/// operator's explicit per-tool OK instead of being held again on a
/// brand-new grant-less session.
///
/// Safety posture (load-bearing):
/// - The CALLER is responsible for filtering `tools` down to the
///   worker's legitimate declared tool set (see
///   `routes_worker_run_exec::filter_to_worker_tool_set`) — this struct
///   never widens what a worker may hold.
/// - Every grant is minted `one_shot = true`, so the dispatch gate
///   consumes it on first use and the approval never outlives the turn.
/// - The session a run mints grants on is never reused by another run
///   (each "Run now" creates a fresh session), so no permanent
///   elevation is possible.
///
/// Example (crate-internal type, shown rather than doc-tested):
///
/// ```text
/// let pre = PreApprovedTools {
///     tools: vec!["file_read".into()],
///     approved_by: Some("human:mark".into()),
/// };
/// // pre.tools is the filtered allow-list; grants mint one-shot.
/// ```
#[derive(Debug, Clone)]
pub(crate) struct PreApprovedTools {
    /// Tool names to grant (already filtered to the worker's tool set).
    pub tools: Vec<String>,
    /// Audit principal of the approving operator, when known.
    pub approved_by: Option<String>,
}

// 13 args is intentional: each is load-bearing (storage handles,
// tenancy ids, the optional owning worker, vertical scope, provider
// config, the job, max steps, max tool rounds, output mode, optional
// pre-approved tools). Bundling into a struct would just push the same
// arity into the struct definition without removing the coupling.
//
// `worker_id` is the teammate that owns this run, when one exists. The
// `argo work serve` queue path passes `None` (system-driven jobs); the
// gateway "Run now" path passes `Some(worker.id)` so the session — and
// therefore every turn audit row emitted under it — attributes to that
// teammate instead of resolving to "unknown" in the audit feed
// (fix-runnow-process-job-worker-attribution / e2e R2-F1).
//
// `pre_approved` carries the operator's one-action "Approve & run"
// tool list (gateway run path only; `argo work serve` passes `None`).
// Grants are minted on the new session AFTER creation, BEFORE the turn.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn process_job(
    store: &Arc<SessionStore>,
    memory: &MemoryStore,
    tenant: TenantId,
    workspace: WorkspaceId,
    worker_id: Option<WorkerId>,
    vertical_id: &str,
    provider: &str,
    model: &str,
    job: &MemoryEntry,
    max_steps: u32,
    max_tool_rounds: usize,
    json_out: bool,
    pre_approved: Option<&PreApprovedTools>,
) -> ArgoResult<()> {
    use super::work_dispatch_phases as p;
    let (prompt, pending_created_at) = p::parse_job_body(job)?;

    if let Some(early) = preflight_research_gate(
        store,
        memory,
        tenant,
        workspace,
        vertical_id,
        provider,
        model,
        job,
        max_steps,
        json_out,
        &pending_created_at,
    )
    .await?
    {
        return early;
    }

    let session_id = p::create_session_with_bundle(
        store, tenant, workspace, worker_id, vertical_id, provider, model, job,
    )
    .await?;

    // "Approve & run" (keystone fix): mint the operator's ONE-SHOT
    // per-tool grants on THIS run's session before the turn executes,
    // so the probation gate sees the approval instead of holding the
    // same tools on yet another grant-less fresh session.
    if let Some(pre) = pre_approved {
        p::mint_pre_approved_grants(store, tenant, session_id, pre).await?;
    }

    // WAVE-2 ⑥: build the in-turn chat→delivery bridge backend for THIS run,
    // capturing the bound vertical manifest + job/worker context so the agent
    // can call the `enqueue_action` tool mid-turn to submit a drafted action
    // for approval. `None` when the worker has no id (system job) or the
    // manifest can't load — the tool then stays unwired (errors on use), never
    // auto-delivering. The backend submits actions through the approval gate
    // and never executes them.
    let deliver_dispatcher = build_deliver_dispatcher(store, tenant, workspace, worker_id, vertical_id, job);
    // work-serve-wire-clarify-dispatcher: a fresh per-run cell the headless
    // `clarify` backend records into. `run_live_loop` turns a recorded question
    // into a deterministic `[STOP: NEED_INFO]` so the agent's correct instinct
    // to ASK (instead of guess) ends the run cleanly at need_info, rather than
    // failing on the unwired-dispatcher error. Always wired on the live worker
    // path; the cell stays empty unless the agent actually calls `clarify`.
    let clarify_cell = Some(std::sync::Arc::new(std::sync::Mutex::new(None)));

    let step_run = p::run_job_steps(
        store,
        session_id,
        vertical_id,
        provider,
        job,
        &prompt,
        max_steps,
        max_tool_rounds,
        deliver_dispatcher,
        clarify_cell,
    )
    .await?;

    finalize_job(
        store,
        memory,
        tenant,
        workspace,
        vertical_id,
        provider,
        model,
        job,
        session_id,
        &pending_created_at,
        &step_run,
        json_out,
    )
    .await
}

/// Post-step orchestration: write per-job artifacts, emit
/// worker_action rows, persist the `completed_jobs` memory row,
/// then print the completion line. Returns the same `()` the
/// caller propagates as the job's exit status.
#[allow(clippy::too_many_arguments)]
async fn finalize_job(
    store: &Arc<SessionStore>,
    memory: &MemoryStore,
    tenant: TenantId,
    workspace: WorkspaceId,
    vertical_id: &str,
    provider: &str,
    model: &str,
    job: &MemoryEntry,
    session_id: argo_types::SessionId,
    pending_created_at: &str,
    step_run: &super::work_steps::StepRunOutput,
    json_out: bool,
) -> ArgoResult<()> {
    use super::work_dispatch_phases as p;
    // Truncate the persisted assistant text to keep the memory entry
    // reasonable. The full text is in the session's message rows;
    // memory carries a preview pointer.
    let preview: String = step_run.assistant_text.chars().take(2000).collect();
    let completed_at = Utc::now().to_rfc3339();
    let (output_dir, actions) = p::write_artifacts_and_emit_actions(
        store,
        memory,
        workspace,
        tenant,
        vertical_id,
        provider,
        model,
        job,
        session_id,
        pending_created_at,
        &completed_at,
        step_run,
        &preview,
    )
    .await?;
    // DEC-050 G2 — inline-dispatch the auto-eligible emitted actions
    // through their declared executors so a "Run now" actually carries
    // the teammate's work out (not just emit + classify, then stop).
    // No-op unless `ARGO_EXECUTION_GRADUATION` is on (shipped default
    // OFF); held / approval-gated / blocked actions stay untouched.
    super::work_dispatch_executor_inline::dispatch_emitted_actions_inline(
        store,
        memory,
        workspace,
        tenant,
        vertical_id,
        &actions,
    )
    .await;
    let actions_emitted = actions.len();
    p::record_completed_job(
        memory,
        workspace,
        vertical_id,
        job,
        session_id,
        step_run,
        &preview,
        &completed_at,
        &output_dir,
        actions_emitted,
    )
    .await?;
    p::print_completion(
        json_out,
        job,
        session_id,
        step_run,
        &completed_at,
        &output_dir,
        actions_emitted,
    );
    Ok(())
}

/// Build the per-run [`argo_tool::DeliverDispatcher`] backend that the
/// in-turn `enqueue_action` tool dispatches into (WAVE-2 ⑥).
///
/// Returns `Some` only when BOTH a `worker_id` is present (an in-turn
/// delivery must attribute to a teammate, not a system job) AND the
/// worker's bound vertical manifest loads — otherwise `None`, which leaves
/// the tool unwired (it errors on use rather than auto-delivering). The
/// backend submits actions through the approval gate; it never executes.
fn build_deliver_dispatcher(
    store: &Arc<SessionStore>,
    tenant: TenantId,
    workspace: WorkspaceId,
    worker_id: Option<WorkerId>,
    vertical_id: &str,
    job: &MemoryEntry,
) -> Option<Arc<dyn argo_tool::DeliverDispatcher>> {
    let worker_id = worker_id?;
    let dir = super::vertical::resolve_verticals_dir();
    let loaded = argo_vertical::load_for_worker(&dir, &tenant.to_string(), vertical_id)?;
    let dispatcher = super::worker_actions_bridge_dispatcher::BridgeDeliverDispatcher::new(
        Arc::clone(store),
        workspace,
        Arc::new(loaded.manifest),
        vertical_id.to_string(),
        job.id.to_string(),
        worker_id,
    );
    Some(Arc::new(dispatcher))
}

/// `#722` — pre-flight research-tools gate. Multi-step worker mode
/// (>1 step) presumes research/verification, so refuses to start
/// when neither `web_search` nor `web_extract` is available to the
/// bound vertical. Returns `Some(Ok(()))` for the recorded-and-
/// terminated path so the caller can early-return; `None` lets the
/// normal pipeline continue.
#[allow(clippy::too_many_arguments)]
async fn preflight_research_gate(
    store: &Arc<SessionStore>,
    memory: &MemoryStore,
    tenant: TenantId,
    workspace: WorkspaceId,
    vertical_id: &str,
    provider: &str,
    model: &str,
    job: &MemoryEntry,
    max_steps: u32,
    json_out: bool,
    pending_created_at: &str,
) -> ArgoResult<Option<ArgoResult<()>>> {
    let research_available = super::work_steps::research_tools_available_for(vertical_id)?;
    if max_steps > 1 && !research_available {
        let outcome = super::work_steps::record_research_unavailable(
            store,
            memory,
            tenant,
            workspace,
            vertical_id,
            provider,
            model,
            job,
            json_out,
            pending_created_at,
        )
        .await;
        return Ok(Some(outcome));
    }
    Ok(None)
}
