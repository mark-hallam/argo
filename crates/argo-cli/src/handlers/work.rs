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
use argo_types::{ArgoError, ArgoResult};
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
                super::work_dispatch::process_job(
                    &store_arc,
                    &memory,
                    tenant,
                    workspace,
                    vertical_id,
                    provider,
                    model,
                    job,
                    max_steps,
                    max_tool_rounds,
                    json_out,
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

// ──────────────────────────────────────────────────── helpers
//
// Per-job dispatch (`process_job`) lives in `super::work_dispatch`.
// Multi-step step drivers + the research-tools pre-flight live in
// `super::work_steps` (#722). What remains here is the public
// command surface (`run`, `add`, `serve`) and the queue-side
// helpers (`parse_body`, `pick_next_pending_job`, `validate_vertical`).

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
