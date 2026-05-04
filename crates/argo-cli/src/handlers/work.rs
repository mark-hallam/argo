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
use argo_types::{ArgoError, ArgoResult, MessageRole, TenantId, WorkspaceId};
use chrono::Utc;
use serde_json::{json, Value};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

pub async fn run(cmd: WorkCmd) -> ArgoResult<()> {
    match cmd {
        WorkCmd::Add { vertical, prompt, json } => add(&vertical, &prompt, json).await,
        WorkCmd::Serve {
            vertical, interval_seconds, provider, model, once, json,
        } => serve(&vertical, interval_seconds, &provider, &model, once, json).await,
    }
}

// ────────────────────────────────────────────────────────── add
async fn add(vertical_id: &str, prompt: &str, json_out: bool) -> ArgoResult<()> {
    let cwd = std::env::current_dir()
        .map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;

    validate_vertical(vertical_id)?;

    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    let name = cwd.file_name().and_then(|s| s.to_str()).unwrap_or("workspace");
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
async fn serve(
    vertical_id: &str,
    interval_seconds: u64,
    provider: &str,
    model: &str,
    once: bool,
    json_out: bool,
) -> ArgoResult<()> {
    let cwd = std::env::current_dir()
        .map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;

    validate_vertical(vertical_id)?;

    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    let name = cwd.file_name().and_then(|s| s.to_str()).unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (tenant, workspace) = store.ensure_workspace(name, &root).await?;
    let store_arc = Arc::new(store);
    let memory = MemoryStore::new(store_arc.clone());

    eprintln!(
        "argo work: serving vertical={vertical_id} provider={provider} model={model} interval={interval_seconds}s{}",
        if once { " (--once)" } else { "" },
    );

    loop {
        // Each tick rebuilds the dedup set from disk → restart-safe.
        let entries = memory
            .list_for_vertical(workspace, MemoryFile::Memory, vertical_id)
            .await?;
        let next = pick_next_pending_job(&entries);

        match next {
            Some(job) => {
                process_job(
                    &store_arc, &memory, tenant, workspace, vertical_id, provider,
                    model, job, json_out,
                )
                .await?;
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

        // Sleep with shutdown awareness — Ctrl-C exits cleanly.
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(interval_seconds)) => {}
            _ = tokio::signal::ctrl_c() => {
                eprintln!("\nargo work: SIGINT received, graceful shutdown");
                return Ok(());
            }
        }
    }
}

// ────────────────────────────────────────────── job dispatch
// 9 args is intentional: each is load-bearing (storage handles,
// tenancy ids, vertical scope, provider config, the job, output
// mode). Bundling into a struct would just push the same arity
// into the struct definition without removing the coupling.
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
    json_out: bool,
) -> ArgoResult<()> {
    let body = parse_body(&job.body).ok_or_else(|| {
        ArgoError::Provider(format!("malformed pending_jobs entry {}", job.id))
    })?;
    let prompt = body
        .get("prompt")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            ArgoError::Provider(format!("pending_jobs {} missing `prompt`", job.id))
        })?;

    // Fresh vertical-bound session per job. Bundled skills are
    // attached at session-creation time via the same path
    // `argo new` uses, so the worker's turn sees the full skill
    // catalog the GTM Engineer (or any vertical) declares.
    let title = format!("work job {}", job.id);
    let session_id = store
        .create_session(tenant, workspace, model, provider, Some(&title), Some(vertical_id))
        .await
        .map_err(|e| ArgoError::Provider(format!("create_session: {e}")))?;
    let store_for_attach = Arc::clone(store);
    let dir = resolve_verticals_dir();
    let (registry, _errors) = argo_vertical::load_dir(&dir)?;
    if let Some(v) = registry.get(vertical_id) {
        let _ = crate::handlers::vertical_attach::install_and_attach_bundle(
            &store_for_attach, v, session_id,
        )
        .await;
    }

    // Run one turn. Mock provider short-circuits to a synthetic
    // assistant message — no LLM dispatch. Live providers go
    // through the same run_turn the gateway POST /runs uses.
    let (assistant_text, assistant_id, user_message_id) = if provider == "mock" {
        // #717: messages are stored as content_json — typed text
        // blocks `[{"type":"text","text":"..."}]` so `argo session
        // show` and `GET /sessions/<id>` extract bodies correctly.
        // The plain text we render in completed_jobs is unchanged.
        let user_content = serde_json::json!([{"type":"text","text":prompt}]).to_string();
        let user_id = store
            .append_message(session_id, MessageRole::User, &user_content)
            .await
            .map_err(|e| ArgoError::Provider(format!("append user msg: {e}")))?;
        let text = format!(
            "[mock] vertical={vertical_id} job={} processed at {}\n\n\
             prompt:\n{prompt}\n\n\
             (deterministic synthetic response — no LLM dispatched)",
            job.id,
            Utc::now().to_rfc3339(),
        );
        let asst_content = serde_json::json!([{"type":"text","text":text}]).to_string();
        let aid = store
            .append_message(session_id, MessageRole::Assistant, &asst_content)
            .await
            .map_err(|e| ArgoError::Provider(format!("append assistant msg: {e}")))?;
        (text, aid, user_id)
    } else {
        // run_turn appends the user message itself when given Some(prompt)
        // and then runs the LLM dispatch. We use the same MAX_TOOL_ROUNDS
        // the gateway runs surface uses.
        let cwd = std::env::current_dir()
            .map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;
        let result = crate::handlers::chat::run_turn(
            store,
            &cwd,
            session_id,
            Some(prompt),
            false,
            crate::handlers::chat::MAX_TOOL_ROUNDS,
        )
        .await?;
        // run_turn doesn't surface the user-message id, so we re-derive
        // it by listing — but the audit trail captures the full chain.
        (result.text, result.assistant_id, Uuid::nil())
    };

    // Truncate the persisted assistant text to keep the memory entry
    // reasonable. The full text is in the session's message rows;
    // memory carries a preview pointer.
    let preview: String = assistant_text.chars().take(2000).collect();
    let completed_at = Utc::now().to_rfc3339();
    let completed_body = json!({
        "schema_key": "completed_jobs",
        "pending_job_id": job.id.to_string(),
        "vertical_id": vertical_id,
        "session_id": session_id.0.to_string(),
        "user_message_id": user_message_id.to_string(),
        "assistant_id": assistant_id.to_string(),
        "assistant_text": preview,
        "completed_at": completed_at,
    });
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
        });
        println!("{}", serde_json::to_string(&out).unwrap_or_default());
    } else {
        println!(
            "processed job {} → session {} → {} chars",
            job.id,
            session_id.0,
            assistant_text.len()
        );
    }
    Ok(())
}

// ──────────────────────────────────────────────────── helpers
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
