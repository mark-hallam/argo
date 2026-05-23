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
use argo_store::SessionStore;
use argo_types::{ArgoResult, TenantId, WorkspaceId};
use chrono::Utc;
use std::sync::Arc;

// 11 args is intentional: each is load-bearing (storage handles,
// tenancy ids, vertical scope, provider config, the job, max
// steps, max tool rounds, output mode). Bundling into a struct
// would just push the same arity into the struct definition
// without removing the coupling.
#[allow(clippy::too_many_arguments)]
pub(super) async fn process_job(
    store: &Arc<SessionStore>,
    memory: &MemoryStore,
    tenant: TenantId,
    workspace: WorkspaceId,
    vertical_id: &str,
    provider: &str,
    model: &str,
    job: &MemoryEntry,
    max_steps: u32,
    max_tool_rounds: usize,
    json_out: bool,
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
        store, tenant, workspace, vertical_id, provider, model, job,
    )
    .await?;

    let step_run = p::run_job_steps(
        store,
        session_id,
        vertical_id,
        provider,
        job,
        &prompt,
        max_steps,
        max_tool_rounds,
    )
    .await?;

    finalize_job(
        store,
        memory,
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
    let (output_dir, actions_emitted) = p::write_artifacts_and_emit_actions(
        store,
        memory,
        workspace,
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
