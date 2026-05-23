//! Per-job phase helpers extracted from
//! [`super::work_dispatch::process_job`]. Each helper owns one
//! linear phase (parse → session+bundle → step run → artifact
//! write → action emission → completed_jobs append → stdout
//! emission) so the orchestrator stays a thin caller and every
//! helper stays well under the 80-LOC ceiling.

use crate::handlers::vertical::resolve_verticals_dir;
use crate::handlers::work_artifacts::ArtifactOutput;
use crate::handlers::work_steps::StepRunOutput;
use argo_memory::{MemoryEntry, MemoryFile, MemoryStore};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, SessionId, TenantId, WorkspaceId};
use serde_json::{json, Value};
use std::sync::Arc;

/// Decode `job.body` into a `(prompt, pending_created_at)` pair.
/// Both fields are surfaced to artifacts + the completed_jobs row,
/// so the caller propagates them downstream.
pub(super) fn parse_job_body(job: &MemoryEntry) -> ArgoResult<(String, String)> {
    let body: Value = serde_json::from_str(&job.body)
        .map_err(|_| ArgoError::Provider(format!("malformed pending_jobs entry {}", job.id)))?;
    let prompt = body
        .get("prompt")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ArgoError::Provider(format!("pending_jobs {} missing `prompt`", job.id)))?
        .to_string();
    let pending_created_at = body
        .get("created_at")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    Ok((prompt, pending_created_at))
}

/// Open a fresh vertical-bound session and install the vertical's
/// bundled skills (best-effort) so the worker sees the full catalog
/// the vertical declares.
pub(super) async fn create_session_with_bundle(
    store: &Arc<SessionStore>,
    tenant: TenantId,
    workspace: WorkspaceId,
    vertical_id: &str,
    provider: &str,
    model: &str,
    job: &MemoryEntry,
) -> ArgoResult<SessionId> {
    let title = format!("work job {}", job.id);
    let session_id = store
        .create_session(
            tenant,
            workspace,
            model,
            provider,
            Some(&title),
            Some(vertical_id),
        )
        .await
        .map_err(|e| ArgoError::Provider(format!("create_session: {e}")))?;
    let store_for_attach = Arc::clone(store);
    let dir = resolve_verticals_dir();
    let (registry, _errors) = argo_vertical::load_dir(&dir)?;
    if let Some(v) = registry.get(vertical_id) {
        let _ = crate::handlers::vertical_attach::install_and_attach_bundle(
            &store_for_attach,
            v,
            session_id,
        )
        .await;
    }
    Ok(session_id)
}

/// Drive the per-job step loop — mock provider short-circuits to a
/// synthetic response, every other provider runs the live loop.
/// Both yield the same `StepRunOutput` shape.
#[allow(clippy::too_many_arguments)]
pub(super) async fn run_job_steps(
    store: &Arc<SessionStore>,
    session_id: SessionId,
    vertical_id: &str,
    provider: &str,
    job: &MemoryEntry,
    prompt: &str,
    max_steps: u32,
    max_tool_rounds: usize,
) -> ArgoResult<StepRunOutput> {
    if provider == "mock" {
        super::work_steps::run_mock_step(
            store,
            session_id,
            vertical_id,
            &job.id.to_string(),
            prompt,
        )
        .await
    } else {
        super::work_steps::run_live_loop(
            store,
            session_id,
            prompt.to_string(),
            max_steps,
            max_tool_rounds,
        )
        .await
    }
}

/// Bundle the per-job artifact metadata, write the on-disk artifact
/// directory, and emit any worker_action rows discovered in the
/// assistant text. Returns the directory string (for the
/// completed_jobs `output_dir`) and the count of actions emitted.
#[allow(clippy::too_many_arguments)]
pub(super) async fn write_artifacts_and_emit_actions(
    store: &Arc<SessionStore>,
    memory: &MemoryStore,
    workspace: WorkspaceId,
    vertical_id: &str,
    provider: &str,
    model: &str,
    job: &MemoryEntry,
    session_id: SessionId,
    pending_created_at: &str,
    completed_at: &str,
    step_run: &StepRunOutput,
    preview: &str,
) -> ArgoResult<(Option<String>, usize)> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("cwd: {e}")))?;
    let stop_reason_str = step_run.outcome.stop_reason.as_str().to_string();
    let stop_detail_str = step_run.outcome.stop_reason.detail().to_string();
    let inputs = super::work_artifacts::ArtifactInputs {
        job_id: job.id,
        session_id,
        vertical_id,
        provider,
        model,
        created_at: pending_created_at,
        completed_at,
        assistant_id: step_run.assistant_id,
        assistant_text: &step_run.assistant_text,
        assistant_text_preview: preview,
        steps_completed: step_run.outcome.steps_completed,
        stop_reason: &stop_reason_str,
        stop_detail: &stop_detail_str,
    };
    let artifact = match super::work_artifacts::write_artifacts(store, &cwd, &inputs).await {
        Ok(a) => Some(a),
        Err(e) => {
            eprintln!("argo work: warning — output artifacts not written: {e}");
            None
        }
    };
    let output_dir = artifact
        .as_ref()
        .map(|a| a.output_dir.to_string_lossy().to_string());
    let actions_emitted = emit_actions(memory, workspace, vertical_id, job, artifact.as_ref()).await;
    Ok((output_dir, actions_emitted))
}

/// `#729` — emit worker_action rows from the assistant text. Failure
/// to persist a single action is logged but doesn't tank the job —
/// the bundle + completed_jobs row remain authoritative either way.
async fn emit_actions(
    memory: &MemoryStore,
    workspace: WorkspaceId,
    vertical_id: &str,
    job: &MemoryEntry,
    artifact: Option<&ArtifactOutput>,
) -> usize {
    if let Some(art) = artifact {
        super::work_emission::emit_and_persist_actions(
            memory,
            workspace,
            vertical_id,
            &job.id.to_string(),
            art,
        )
        .await
    } else {
        0
    }
}

/// Write the `completed_jobs` memory row that the dedup set keys
/// off. The persisted row pairs the pending-job id, session, and
/// stop-reason summary so downstream rollups can attribute spend.
#[allow(clippy::too_many_arguments)]
pub(super) async fn record_completed_job(
    memory: &MemoryStore,
    workspace: WorkspaceId,
    vertical_id: &str,
    job: &MemoryEntry,
    session_id: SessionId,
    step_run: &StepRunOutput,
    preview: &str,
    completed_at: &str,
    output_dir: &Option<String>,
    actions_emitted: usize,
) -> ArgoResult<()> {
    let mut completed_body = json!({
        "schema_key": "completed_jobs",
        "pending_job_id": job.id.to_string(),
        "vertical_id": vertical_id,
        "session_id": session_id.0.to_string(),
        "user_message_id": step_run.user_message_id.to_string(),
        "assistant_id": step_run.assistant_id.to_string(),
        "assistant_text": preview,
        "completed_at": completed_at,
        "steps_completed": step_run.outcome.steps_completed,
        "stop_reason": step_run.outcome.stop_reason.as_str().to_string(),
        "stop_detail": step_run.outcome.stop_reason.detail().to_string(),
    });
    if let Some(dir) = output_dir {
        completed_body["output_dir"] = json!(dir);
    }
    completed_body["actions_emitted"] = json!(actions_emitted);
    memory
        .append_for_vertical(
            workspace,
            MemoryFile::Memory,
            completed_body.to_string(),
            Some(session_id),
            vertical_id,
        )
        .await?;
    Ok(())
}

/// Emit the job-completion line to stdout — JSON when `json_out`,
/// the legacy human-readable summary otherwise.
pub(super) fn print_completion(
    json_out: bool,
    job: &MemoryEntry,
    session_id: SessionId,
    step_run: &StepRunOutput,
    completed_at: &str,
    output_dir: &Option<String>,
    actions_emitted: usize,
) {
    let stop_reason_str = step_run.outcome.stop_reason.as_str().to_string();
    let stop_detail_str = step_run.outcome.stop_reason.detail().to_string();
    if json_out {
        let out = json!({
            "ok": true,
            "job_id": job.id.to_string(),
            "session_id": session_id.0.to_string(),
            "assistant_id": step_run.assistant_id.to_string(),
            "completed_at": completed_at,
            "output_dir": output_dir,
            "steps_completed": step_run.outcome.steps_completed,
            "stop_reason": stop_reason_str,
            "stop_detail": stop_detail_str,
            "actions_emitted": actions_emitted,
        });
        println!("{}", serde_json::to_string(&out).unwrap_or_default());
    } else {
        println!(
            "processed job {} → session {} → {} chars (steps={}, stop_reason={}, actions_emitted={})",
            job.id,
            session_id.0,
            step_run.assistant_text.len(),
            step_run.outcome.steps_completed,
            stop_reason_str,
            actions_emitted,
        );
    }
}
