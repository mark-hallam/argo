//! Per-job dispatch for `argo work serve`.
//!
//! Extracted from `handlers/work.rs` to keep the file under the
//! 500-LOC ceiling. `serve()` calls `process_job` once per claimed
//! `pending_jobs` row; everything from research-tools pre-flight
//! through artifact emission and the `completed_jobs` memory write
//! lives here.

use crate::handlers::vertical::resolve_verticals_dir;
use argo_memory::{MemoryEntry, MemoryFile, MemoryStore};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId, WorkspaceId};
use chrono::Utc;
use serde_json::{json, Value};
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
    let body: Value = serde_json::from_str(&job.body)
        .map_err(|_| ArgoError::Provider(format!("malformed pending_jobs entry {}", job.id)))?;
    let prompt = body
        .get("prompt")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ArgoError::Provider(format!("pending_jobs {} missing `prompt`", job.id)))?;
    let pending_created_at = body
        .get("created_at")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // #722 — pre-flight research-tools gate. Multi-step worker
    // mode (>1 step) presumes research/verification, so refuses
    // to start when neither web_search nor web_extract is
    // available to the bound vertical. Sets the
    // `research_tools_unavailable` stop reason so the artifacts
    // and completed_jobs row reflect WHY it stopped (vs.
    // silently producing an unverified output).
    let research_available = super::work_steps::research_tools_available_for(vertical_id)?;
    if max_steps > 1 && !research_available {
        return super::work_steps::record_research_unavailable(
            store,
            memory,
            tenant,
            workspace,
            vertical_id,
            provider,
            model,
            job,
            json_out,
            &pending_created_at,
        )
        .await;
    }

    // Fresh vertical-bound session per job. Bundled skills are
    // attached at session-creation time via the same path
    // `argo new` uses, so the worker's turn sees the full skill
    // catalog the GTM Engineer (or any vertical) declares.
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

    // #722 — drive the multi-step loop. Mock provider still
    // short-circuits to a synthetic deterministic response (no
    // LLM dispatch); live providers run chat::run_turn per step.
    // Both paths yield the same StepRunOutput shape.
    let step_run = if provider == "mock" {
        super::work_steps::run_mock_step(
            store,
            session_id,
            vertical_id,
            &job.id.to_string(),
            prompt,
        )
        .await?
    } else {
        super::work_steps::run_live_loop(
            store,
            session_id,
            prompt.to_string(),
            max_steps,
            max_tool_rounds,
        )
        .await?
    };
    let assistant_text = step_run.assistant_text;
    let assistant_id = step_run.assistant_id;
    let user_message_id = step_run.user_message_id;
    let outcome = step_run.outcome;

    let stop_reason_str = outcome.stop_reason.as_str().to_string();
    let stop_detail_str = outcome.stop_reason.detail().to_string();

    // Truncate the persisted assistant text to keep the memory entry
    // reasonable. The full text is in the session's message rows;
    // memory carries a preview pointer.
    let preview: String = assistant_text.chars().take(2000).collect();
    let completed_at = Utc::now().to_rfc3339();

    // #721: write per-job artifacts to .skycot/outputs/<job_id>/.
    // Failure here is non-fatal — we still record the completed_jobs
    // memory entry so the dedup set converges; we just lose the
    // disk-side mirror. The session's message rows remain canonical.
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;
    let inputs = super::work_artifacts::ArtifactInputs {
        job_id: job.id,
        session_id,
        vertical_id,
        provider,
        model,
        created_at: &pending_created_at,
        completed_at: &completed_at,
        assistant_id,
        assistant_text: &assistant_text,
        assistant_text_preview: &preview,
        steps_completed: outcome.steps_completed,
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

    // #729 — emit actions from every assistant turn's text and
    // persist each as a `worker_action` memory row + render
    // execution_plan.md alongside the artifact bundle. Failure
    // to persist a single action is logged but doesn't tank the
    // job — the bundle on disk plus the completed_jobs memory
    // entry stay authoritative either way.
    let actions_emitted = if let Some(art) = artifact.as_ref() {
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
    };

    let mut completed_body = json!({
        "schema_key": "completed_jobs",
        "pending_job_id": job.id.to_string(),
        "vertical_id": vertical_id,
        "session_id": session_id.0.to_string(),
        "user_message_id": user_message_id.to_string(),
        "assistant_id": assistant_id.to_string(),
        "assistant_text": preview,
        "completed_at": completed_at,
        "steps_completed": outcome.steps_completed,
        "stop_reason": stop_reason_str,
        "stop_detail": stop_detail_str,
    });
    if let Some(dir) = &output_dir {
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

    if json_out {
        let out = json!({
            "ok": true,
            "job_id": job.id.to_string(),
            "session_id": session_id.0.to_string(),
            "assistant_id": assistant_id.to_string(),
            "completed_at": completed_at,
            "output_dir": output_dir,
            "steps_completed": outcome.steps_completed,
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
            assistant_text.len(),
            outcome.steps_completed,
            stop_reason_str,
            actions_emitted,
        );
    }
    Ok(())
}
