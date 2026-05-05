//! Multi-step worker step drivers (#722).
//!
//! Extracted from `handlers/work.rs::process_job` to keep the
//! main handler under the per-file budget. Three pieces:
//!
//! * `run_mock_step` — synthetic deterministic response; no LLM
//!   dispatch. Always emits the four sectioned deliverables +
//!   `[STOP: TASK_COMPLETE]` so downstream artifact splitters
//!   and the loop terminator both see what they expect.
//! * `run_live_loop` — runs `chat::run_turn` per step inside the
//!   `work_loop::run_loop` orchestrator. Each step is one full
//!   agent turn (with internal tool dispatch up to
//!   `MAX_TOOL_ROUNDS`).
//! * `research_tools_available_for` + `record_research_unavailable`
//!   — pre-flight gate. Multi-step runs presume research, so we
//!   refuse to start when neither `web_search` nor `web_extract`
//!   is available to the bound vertical and write a clear
//!   `research_tools_unavailable` artifact bundle in its place.

use crate::handlers::vertical::resolve_verticals_dir;
use argo_memory::{MemoryEntry, MemoryFile, MemoryStore};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, MessageRole, SessionId, TenantId, WorkspaceId};
use chrono::Utc;
use serde_json::json;
use std::collections::HashSet;
use std::sync::Arc;
use uuid::Uuid;

use super::work_artifacts;
use super::work_loop;

/// What a step driver returns to `process_job`. The two-tuple
/// signature stayed inside work.rs in v1; v2 splits the
/// outcome out so the caller can persist `steps_completed` and
/// `stop_reason` alongside the message ids.
pub(crate) struct StepRunOutput {
    pub assistant_text: String,
    pub assistant_id: Uuid,
    pub user_message_id: Uuid,
    pub outcome: work_loop::LoopOutcome,
}

/// Synthetic single-step mock. Worker v2 still wants four
/// labelled sections in the output so the artifact splitter
/// has something to split, plus the `[STOP: TASK_COMPLETE]`
/// marker so the loop terminates cleanly.
pub(crate) async fn run_mock_step(
    store: &Arc<SessionStore>,
    session_id: SessionId,
    vertical_id: &str,
    job_id_str: &str,
    prompt: &str,
) -> ArgoResult<StepRunOutput> {
    let user_content = json!([{"type":"text","text":prompt}]).to_string();
    let user_id = store
        .append_message(session_id, MessageRole::User, &user_content)
        .await
        .map_err(|e| ArgoError::Provider(format!("append user msg: {e}")))?;
    let text = format!(
        "[mock] vertical={vertical_id} job={job_id_str} processed at {}\n\n\
         prompt:\n{prompt}\n\n\
         (deterministic synthetic response — no LLM dispatched)\n\n\
         ## research\n_(mock: research stub — real backends arrive Phase 4 / §12)_\n\n\
         ## pricing\n_(mock: pricing stub)_\n\n\
         ## listings\n_(mock: listings stub)_\n\n\
         ## approval_checklist\n- [ ] human review (mock placeholder)\n\n\
         [STOP: TASK_COMPLETE]\n",
        Utc::now().to_rfc3339(),
    );
    let asst_content = json!([{"type":"text","text":text}]).to_string();
    let aid = store
        .append_message(session_id, MessageRole::Assistant, &asst_content)
        .await
        .map_err(|e| ArgoError::Provider(format!("append assistant msg: {e}")))?;
    Ok(StepRunOutput {
        assistant_text: text.clone(),
        assistant_id: aid,
        user_message_id: user_id,
        outcome: work_loop::LoopOutcome {
            steps_completed: 1,
            stop_reason: work_loop::StopReason::TaskComplete,
            final_text: text,
        },
    })
}

/// Live multi-step loop. Each iteration calls `chat::run_turn`
/// (which itself runs the agent loop with tool dispatch up to
/// MAX_TOOL_ROUNDS) and parses the assistant text for a stop
/// marker. Continues until a marker is found or `max_steps` is
/// reached.
pub(crate) async fn run_live_loop(
    store: &Arc<SessionStore>,
    session_id: SessionId,
    initial_prompt: String,
    max_steps: u32,
) -> ArgoResult<StepRunOutput> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;
    // Tracked outside the closure because async closures can't
    // easily mutate captured state; we hold the latest assistant
    // id in a Cell so the loop helper stays parametric.
    let last_assistant_id = std::cell::Cell::new(Uuid::nil());

    let outcome = work_loop::run_loop(initial_prompt, max_steps, |_step, prompt| {
        let store = store.clone();
        let cwd = cwd.clone();
        let last_assistant_id = &last_assistant_id;
        async move {
            match crate::handlers::chat::run_turn(
                &store,
                &cwd,
                session_id,
                Some(&prompt),
                false,
                crate::handlers::chat::MAX_TOOL_ROUNDS,
            )
            .await
            {
                Ok(r) => {
                    last_assistant_id.set(r.assistant_id);
                    work_loop::StepResult { text: r.text }
                }
                Err(e) => {
                    // Surface as a [STOP: NEED_INFO] so the loop
                    // terminates cleanly — the actual error text
                    // is preserved in the assistant message body
                    // and shows up in the transcript.
                    work_loop::StepResult {
                        text: format!(
                            "worker step failed: {e}\n[STOP: NEED_INFO transport error — \
                             retry once the upstream issue clears]"
                        ),
                    }
                }
            }
        }
    })
    .await;

    Ok(StepRunOutput {
        assistant_text: outcome.final_text.clone(),
        assistant_id: last_assistant_id.get(),
        user_message_id: Uuid::nil(),
        outcome,
    })
}

/// Lookup the bound vertical's deny set and decide whether
/// research is plausible. "Available" means at least one of
/// `web_search` or `web_extract` is NOT in the deny list.
/// Both are registered in the builtin set, so the only filter
/// in this build is the manifest deny.
pub(crate) fn research_tools_available_for(vertical_id: &str) -> ArgoResult<bool> {
    let dir = resolve_verticals_dir();
    let (registry, _errors) = argo_vertical::load_dir(&dir)?;
    let v = match registry.get(vertical_id) {
        Some(v) => v,
        None => return Ok(true), // unknown vertical → don't second-guess
    };
    let denied: HashSet<&str> = v
        .manifest
        .tools
        .as_ref()
        .map(|t| t.denied.iter().map(String::as_str).collect())
        .unwrap_or_default();
    Ok(!denied.contains("web_search") || !denied.contains("web_extract"))
}

/// Pre-flight failure path. We still want a `completed_jobs`
/// memory entry (so the dedup set converges and the queue
/// drains) and an artifact directory written so operators can
/// see WHY the job stopped — but no LLM is dispatched.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn record_research_unavailable(
    store: &Arc<SessionStore>,
    memory: &MemoryStore,
    tenant: TenantId,
    workspace: WorkspaceId,
    vertical_id: &str,
    provider: &str,
    model: &str,
    job: &MemoryEntry,
    json_out: bool,
    pending_created_at: &str,
) -> ArgoResult<()> {
    let completed_at = Utc::now().to_rfc3339();
    let stop_reason = work_loop::StopReason::ResearchToolsUnavailable;
    let stop_reason_str = stop_reason.as_str().to_string();
    let detail = format!(
        "vertical '{vertical_id}' denies both web_search and web_extract — \
         research-flavored worker run cannot proceed. Wire an MCP web backend \
         (Phase 4 / §12) and remove these tools from `[tools].denied`, or run \
         with `--max-steps 1` to bypass the pre-flight check."
    );
    let assistant_text = format!(
        "Worker stopped before any LLM dispatch.\n\n\
         ## approval_checklist\n- [ ] research tools unavailable\n\n\
         ## research\n_(not produced — research tools unavailable)_\n\n\
         ## pricing\n_(not produced — research tools unavailable)_\n\n\
         ## listings\n_(not produced — research tools unavailable)_\n\n\
         {detail}\n"
    );
    let preview: String = assistant_text.chars().take(2000).collect();
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;

    let session_id = store
        .create_session(
            tenant,
            workspace,
            model,
            provider,
            Some(&format!("work job {} (no research)", job.id)),
            Some(vertical_id),
        )
        .await
        .map_err(|e| ArgoError::Provider(format!("create_session (pre-flight): {e}")))?;

    let inputs = work_artifacts::ArtifactInputs {
        job_id: job.id,
        session_id,
        vertical_id,
        provider,
        model,
        created_at: pending_created_at,
        completed_at: &completed_at,
        assistant_id: Uuid::nil(),
        assistant_text: &assistant_text,
        assistant_text_preview: &preview,
        steps_completed: 0,
        stop_reason: &stop_reason_str,
        stop_detail: &detail,
    };
    let output_dir = match work_artifacts::write_artifacts(store, &cwd, &inputs).await {
        Ok(p) => Some(p.to_string_lossy().to_string()),
        Err(e) => {
            eprintln!("argo work: warning — output artifacts not written: {e}");
            None
        }
    };

    let mut completed_body = json!({
        "schema_key": "completed_jobs",
        "pending_job_id": job.id.to_string(),
        "vertical_id": vertical_id,
        "session_id": session_id.0.to_string(),
        "assistant_text": preview,
        "completed_at": completed_at,
        "steps_completed": 0,
        "stop_reason": stop_reason_str,
        "stop_detail": detail,
    });
    if let Some(dir) = &output_dir {
        completed_body["output_dir"] = json!(dir);
    }
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
            "ok": false,
            "job_id": job.id.to_string(),
            "session_id": session_id.0.to_string(),
            "completed_at": completed_at,
            "output_dir": output_dir,
            "steps_completed": 0,
            "stop_reason": stop_reason_str,
            "stop_detail": detail,
        });
        println!("{}", serde_json::to_string(&out).unwrap_or_default());
    } else {
        eprintln!("argo work: job {} stopped pre-flight: {detail}", job.id);
    }
    Ok(())
}

// Pre-flight gate behaviour is exercised end-to-end in
// `crates/argo-cli/tests/work_multi_step.rs::multi_step_fails_fast_when_research_tools_unavailable`.
// (We don't unit-test `research_tools_available_for` here
// because the verticals dir resolves relative to cwd / env,
// which the integration test sets correctly via
// `ARGO_VERTICALS_DIR` but a `cargo test --lib` run does not.)
