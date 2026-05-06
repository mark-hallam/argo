//! Unit tests for `worker_doctor` (#739). Drives a real
//! `MemoryStore` (`:memory:` SQLite), seeds action rows, and
//! asserts the report's counts + recent-row windows are correct.

use super::*;
use crate::handlers::worker_actions::ActionRecord;
use argo_memory::{MemoryFile, MemoryStore};
use argo_store::SessionStore;
use argo_types::WorkspaceId;
use chrono::Utc;
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

fn rec(action_id: &str, action_type: &str, state: &str, requires_approval: bool) -> ActionRecord {
    ActionRecord {
        schema_key: ActionRecord::SCHEMA_KEY.to_string(),
        action_id: action_id.to_string(),
        job_id: "job-1".to_string(),
        worker_id: argo_identity::WorkerId::nil(),
        item_id: None,
        action_type: action_type.to_string(),
        risk_level: "medium".to_string(),
        requires_approval,
        state: state.to_string(),
        payload: serde_json::json!({}),
        approved_by: None,
        approved_at: None,
        executed_at: None,
        platform: None,
        result: None,
        evidence: Vec::new(),
        blocked_reason: if state == "blocked" {
            Some("research evidence insufficient".to_string())
        } else {
            None
        },
        created_at: Utc::now().to_rfc3339(),
    }
}

async fn seed(memory: &MemoryStore, ws: WorkspaceId, vertical: &str, r: &ActionRecord) {
    memory
        .append_for_vertical(
            ws,
            MemoryFile::Memory,
            serde_json::to_string(r).unwrap(),
            None,
            vertical,
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn collect_latest_dedupes_to_one_row_per_action_id() {
    let (memory, ws) = open_test_memory().await;
    let dir = super::super::vertical::resolve_verticals_dir();
    let (registry, _) = argo_vertical::load_dir(&dir).unwrap();

    // Seed two rows for the same action_id (pending then executing).
    let mut r = rec("act-1", "verify_price", "pending", false);
    seed(&memory, ws, "gtm-engineer", &r).await;
    // Bump time + state for second row.
    r.state = "executing".into();
    r.created_at = (Utc::now() + chrono::Duration::seconds(1)).to_rfc3339();
    seed(&memory, ws, "gtm-engineer", &r).await;

    let rows = collect_latest(&memory, ws, None, &registry).await;
    let ours: Vec<_> = rows.iter().filter(|r| r.action_id == "act-1").collect();
    assert_eq!(ours.len(), 1);
    assert_eq!(ours[0].state, "executing");
}

#[tokio::test]
async fn build_report_groups_counts_by_state_and_approval_class() {
    let (memory, ws) = open_test_memory().await;
    seed(
        &memory,
        ws,
        "gtm-engineer",
        &rec("a-1", "verify_price", "pending", false),
    )
    .await;
    seed(
        &memory,
        ws,
        "gtm-engineer",
        &rec("a-2", "post_listing", "pending", true),
    )
    .await;
    seed(
        &memory,
        ws,
        "gtm-engineer",
        &rec("a-3", "post_listing", "completed", true),
    )
    .await;
    seed(
        &memory,
        ws,
        "gtm-engineer",
        &rec("a-4", "drain_account", "blocked", false),
    )
    .await;
    let tmp = tempfile::tempdir().unwrap();
    let report = build_report(&memory, ws, tmp.path(), None).await.unwrap();
    assert_eq!(report.counts.pending_autonomous, 1);
    assert_eq!(report.counts.pending_approval, 1);
    assert_eq!(report.counts.completed, 1);
    assert_eq!(report.counts.blocked, 1);
    assert_eq!(report.counts.total, 4);
    assert_eq!(report.recent_blocked.len(), 1);
    assert!(report.recent_blocked[0]
        .reason
        .contains("research evidence insufficient"));
}

#[tokio::test]
async fn build_report_lists_loaded_verticals_with_action_counts() {
    let (memory, ws) = open_test_memory().await;
    let tmp = tempfile::tempdir().unwrap();
    let report = build_report(&memory, ws, tmp.path(), Some("gtm-engineer"))
        .await
        .unwrap();
    assert!(report
        .verticals
        .iter()
        .any(|v| v.id == "gtm-engineer" && v.actions_declared >= 5));
}

#[tokio::test]
async fn build_report_recent_failed_pulls_failure_reason_from_result() {
    let (memory, ws) = open_test_memory().await;
    let mut r = rec("a-fail", "verify_price", "failed", false);
    r.result = Some(serde_json::json!({"failure": "marketplace 503"}));
    seed(&memory, ws, "gtm-engineer", &r).await;
    let tmp = tempfile::tempdir().unwrap();
    let report = build_report(&memory, ws, tmp.path(), None).await.unwrap();
    assert_eq!(report.recent_failed.len(), 1);
    assert_eq!(report.recent_failed[0].reason, "marketplace 503");
}
