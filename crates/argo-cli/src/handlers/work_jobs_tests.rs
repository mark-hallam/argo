//! Unit tests for `work_jobs::collect_jobs` (#742). Drives a
//! real `:memory:` SQLite, seeds pending + completed rows,
//! asserts the merged view comes back correctly.

use super::*;
use argo_memory::{MemoryFile, MemoryStore};
use argo_store::SessionStore;
use argo_types::WorkspaceId;
use std::path::PathBuf;
use std::sync::Arc;

fn set_verticals_dir() {
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.to_path_buf())
        .expect("workspace root");
    std::env::set_var("ARGO_VERTICALS_DIR", workspace_root.join("verticals"));
}

async fn open_test_memory() -> (MemoryStore, WorkspaceId) {
    set_verticals_dir();
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    let store = Arc::new(store);
    let (_t, ws) = store.ensure_workspace("ws", "/tmp/ws").await.unwrap();
    (MemoryStore::new(store), ws)
}

async fn seed_pending(memory: &MemoryStore, ws: WorkspaceId, vertical: &str) -> String {
    let body = serde_json::json!({
        "schema_key": "pending_jobs",
        "vertical_id": vertical,
        "prompt": "test prompt",
        "status": "pending",
        "created_at": "2026-05-05T10:00:00Z",
        "attempts": 0,
    });
    let entry = memory
        .append_for_vertical(ws, MemoryFile::Memory, body.to_string(), None, vertical)
        .await
        .unwrap();
    entry.id.to_string()
}

async fn seed_completed(
    memory: &MemoryStore,
    ws: WorkspaceId,
    vertical: &str,
    pending_job_id: &str,
    stop_reason: &str,
    actions_emitted: u32,
) {
    let body = serde_json::json!({
        "schema_key": "completed_jobs",
        "pending_job_id": pending_job_id,
        "vertical_id": vertical,
        "session_id": "deadbeef-...",
        "user_message_id": "u-1",
        "assistant_id": "a-1",
        "assistant_text": "...",
        "completed_at": "2026-05-05T10:01:00Z",
        "steps_completed": 3,
        "stop_reason": stop_reason,
        "stop_detail": "",
        "actions_emitted": actions_emitted,
        "output_dir": "/tmp/.skycot/outputs/abc",
    });
    memory
        .append_for_vertical(ws, MemoryFile::Memory, body.to_string(), None, vertical)
        .await
        .unwrap();
}

#[tokio::test]
async fn collect_jobs_lists_pending_with_no_completed_match() {
    let (memory, ws) = open_test_memory().await;
    let id = seed_pending(&memory, ws, "gtm-engineer").await;
    let rows = collect_jobs(&memory, ws, Some("gtm-engineer"))
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].job_id, id);
    assert_eq!(rows[0].state, "pending");
    assert!(rows[0].stop_reason.is_none());
    assert!(rows[0].output_dir.is_none());
}

#[tokio::test]
async fn collect_jobs_promotes_pending_to_completed_when_completion_row_matches() {
    let (memory, ws) = open_test_memory().await;
    let id = seed_pending(&memory, ws, "gtm-engineer").await;
    seed_completed(&memory, ws, "gtm-engineer", &id, "task_complete", 2).await;

    let rows = collect_jobs(&memory, ws, Some("gtm-engineer"))
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].state, "completed");
    assert_eq!(rows[0].stop_reason.as_deref(), Some("task_complete"));
    assert_eq!(rows[0].steps_completed, Some(3));
    assert_eq!(rows[0].actions_emitted, Some(2));
    assert_eq!(rows[0].created_at, "2026-05-05T10:01:00Z");
    assert_eq!(
        rows[0].output_dir.as_deref(),
        Some("/tmp/.skycot/outputs/abc")
    );
}

#[tokio::test]
async fn collect_jobs_filters_to_named_vertical() {
    let (memory, ws) = open_test_memory().await;
    let _gtm = seed_pending(&memory, ws, "gtm-engineer").await;

    // Filter to gtm-engineer — finds the row.
    let rows = collect_jobs(&memory, ws, Some("gtm-engineer"))
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);

    // Filter to a vertical that doesn't exist in registry → 0
    // rows (because list_for_vertical is scoped + the registry
    // walk skips unknown ids).
    let rows_other = collect_jobs(&memory, ws, Some("non-existent")).await;
    // load_dir should still succeed (ignores unknowns); the
    // filter will return 0.
    assert!(rows_other.is_ok());
    assert_eq!(rows_other.unwrap().len(), 0);
}
