//! `argo work {add, serve}` — minimal worker runtime (#716).
//!
//! Bridges the existing per-turn machinery into a persistent,
//! poll-and-process loop. Uses only existing primitives:
//! sessions, vertical_id, vertical-namespaced memory,
//! `argo-store::SessionStore::append_message`, and
//! `crate::handlers::chat::run_turn` for live providers. Mock
//! provider short-circuits to a synthetic deterministic
//! response — no API key needed.
//!
//! ## Job state in memory
//!
//!   pending_jobs    { schema_key, vertical_id, prompt, status,
//!                     created_at, attempts }
//!   completed_jobs  { schema_key, pending_job_id, vertical_id,
//!                     session_id, user_message_id, assistant_id,
//!                     assistant_text, completed_at }
//!
//! De-duplication: on each tick the worker rebuilds the set of
//! `completed_jobs.pending_job_id` values from disk and processes
//! only `pending_jobs` whose own UUIDs aren't in the set.
//! Restart-safe — the set is rebuilt on each tick.

use crate::cli::WorkCmd;
use crate::handlers::vertical::resolve_verticals_dir;
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_memory::{MemoryEntry, MemoryFile, MemoryStore};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId, WorkspaceId};
use chrono::Utc;
use serde_json::{json, Value};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

pub async fn run(cmd: WorkCmd) -> ArgoResult<()> {
    match cmd {
        WorkCmd::Add {
            vertical,
            prompt,
            json,
        } => add(&vertical, &prompt, json).await,
        WorkCmd::Serve {
            vertical,
            interval_seconds,
            provider,
            model,
            once,
            max_steps,
            max_tool_rounds,
            json,
        } => {
            serve(
                &vertical,
                interval_seconds,
                &provider,
                &model,
                once,
                max_steps,
                max_tool_rounds,
                json,
            )
            .await
        }
        WorkCmd::Jobs {
            state,
            vertical,
            limit,
            json,
        } => super::work_jobs::list(&state, vertical.as_deref(), limit, json).await,
        WorkCmd::Logs { job_id, kind } => super::work_jobs::logs(&job_id, &kind).await,
    }
}

// ────────────────────────────────────────────────────────── add
async fn add(vertical_id: &str, prompt: &str, json_out: bool) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;

    validate_vertical(vertical_id)?;

    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    let name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (_tenant, workspace) = store.ensure_workspace(name, &root).await?;
    let store_arc = Arc::new(store);
    let memory = MemoryStore::new(store_arc.clone());

    // The memory entry's UUID is the canonical job_id.
    let now = Utc::now().to_rfc3339();
    let body = json!({
        "schema_key": "pending_jobs",
        "vertical_id": vertical_id,
        "prompt": prompt,
        "status": "pending",
        "created_at": now,
        "attempts": 0,
    });
    let entry = memory
        .append_for_vertical(
            workspace,
            MemoryFile::Memory,
            body.to_string(),
            None,
            vertical_id,
        )
        .await?;

    if json_out {
        let out = json!({
            "ok": true,
            "job_id": entry.id.to_string(),
            "vertical_id": vertical_id,
            "status": "pending",
            "created_at": now,
        });
        println!("{}", serde_json::to_string(&out).unwrap_or_default());
    } else {
        println!("queued job {} for vertical {vertical_id}", entry.id);
    }
    Ok(())
}

// ──────────────────────────────────────────────────────── serve
#[allow(clippy::too_many_arguments)]
async fn serve(
    vertical_id: &str,
    interval_seconds: u64,
    provider: &str,
    model: &str,
    once: bool,
    max_steps: u32,
    max_tool_rounds: usize,
    json_out: bool,
) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;

    validate_vertical(vertical_id)?;

    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    let name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (tenant, workspace) = store.ensure_workspace(name, &root).await?;
    let store_arc = Arc::new(store);
    let memory = MemoryStore::new(store_arc.clone());

    eprintln!(
        "argo work: serving vertical={vertical_id} provider={provider} model={model} interval={interval_seconds}s max_steps={max_steps} max_tool_rounds={max_tool_rounds}{}",
        if once { " (--once)" } else { "" },
    );

    // #770 — install the SIGINT/SIGTERM watcher so `systemctl
    // stop` / `kubectl delete pod` get a clean exit instead of
    // SIGKILL ripping mid-step. Installed even in --once so the
    // operator can Ctrl+C a one-shot run cleanly.
    let shutdown = super::work_shutdown::install_shutdown_watcher();

    loop {
        // Top-of-loop check: a signal during job processing flips
        // the flag; we observe it here and exit before starting
        // another job ("finish current step, then exit").
        if shutdown.load(std::sync::atomic::Ordering::SeqCst) {
            eprintln!("argo work: graceful shutdown");
            return Ok(());
        }
        // Each tick rebuilds the dedup set from disk → restart-safe.
        let entries = memory
            .list_for_vertical(workspace, MemoryFile::Memory, vertical_id)
            .await?;
        match pick_next_pending_job(&entries) {
            Some(job) => {
                process_job(&store_arc, &memory, tenant, workspace, vertical_id, provider, model, job, max_steps, max_tool_rounds, json_out).await?;
            }
            None => {
                if once {
                    eprintln!("argo work: queue empty, --once → exit");
                    return Ok(());
                }
                eprintln!("argo work: queue empty, sleeping {interval_seconds}s");
            }
        }
        if once {
            return Ok(());
        }
        // Signal during sleep wakes us up immediately rather
        // than waiting the full interval.
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(interval_seconds)) => {}
            _ = super::work_shutdown::wait_for_shutdown(&shutdown) => {
                eprintln!("argo work: graceful shutdown");
                return Ok(());
            }
        }
    }
}

// ────────────────────────────────────────────── job dispatch
// 11 args is intentional: each is load-bearing (storage handles,
// tenancy ids, vertical scope, provider config, the job, max
// steps, max tool rounds, output mode). Bundling into a struct
// would just push the same arity into the struct definition
// without removing the coupling.
#[allow(clippy::too_many_arguments)]
async fn process_job(
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
    let body = parse_body(&job.body)
        .ok_or_else(|| ArgoError::Provider(format!("malformed pending_jobs entry {}", job.id)))?;
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

// ──────────────────────────────────────────────────── helpers
//
// Multi-step step drivers + the research-tools pre-flight live
// in `super::work_steps` (#722) so this file stays under the
// per-file budget. What remains here is just the queue-side
// helpers (parse_body, pick_next_pending_job, validate_vertical).

fn parse_body(body: &str) -> Option<Value> {
    serde_json::from_str(body).ok()
}

/// Scan memory entries for the oldest `pending_jobs` row whose own
/// UUID is NOT recorded as a `completed_jobs.pending_job_id`. Entries
/// come back from `list_for_vertical` already sorted oldest-first.
fn pick_next_pending_job(entries: &[MemoryEntry]) -> Option<&MemoryEntry> {
    let mut completed: HashSet<Uuid> = HashSet::new();
    for e in entries {
        if let Some(b) = parse_body(&e.body) {
            if b.get("schema_key").and_then(|v| v.as_str()) == Some("completed_jobs") {
                if let Some(s) = b.get("pending_job_id").and_then(|v| v.as_str()) {
                    if let Ok(u) = Uuid::parse_str(s) {
                        completed.insert(u);
                    }
                }
            }
        }
    }
    entries.iter().find(|e| {
        if completed.contains(&e.id) {
            return false;
        }
        match parse_body(&e.body) {
            Some(b) => b.get("schema_key").and_then(|v| v.as_str()) == Some("pending_jobs"),
            None => false,
        }
    })
}

fn validate_vertical(vertical_id: &str) -> ArgoResult<()> {
    let dir = resolve_verticals_dir();
    let (registry, _errors) = argo_vertical::load_dir(&dir)?;
    if registry.get(vertical_id).is_none() {
        return Err(ArgoError::Provider(format!(
            "vertical '{vertical_id}' not found in {} — run `argo vertical list` to see what's loaded",
            dir.display()
        )));
    }
    Ok(())
}
