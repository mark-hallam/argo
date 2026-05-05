//! Unit tests for `worker_actions_transitions` (#730). Drives
//! a real `MemoryStore` (`:memory:` SQLite) so the round-trip
//! through `append_for_vertical` + `list_for_vertical` is
//! exercised end-to-end in process — no temp dirs, no spawned
//! processes.
//!
//! Loaded via `#[cfg(test)] #[path = ...] mod tests;` from
//! `worker_actions_transitions.rs`.

use super::*;
use crate::handlers::worker_actions::ActionRecord;
use argo_memory::{MemoryFile, MemoryStore};
use argo_store::SessionStore;
use argo_types::WorkspaceId;
use std::path::PathBuf;
use std::sync::Arc;

const FIXTURE_VERTICAL: &str = "gtm-engineer";

/// Point `ARGO_VERTICALS_DIR` at the workspace's real verticals
/// directory so `find_action`'s `load_dir` walks `gtm-engineer`
/// (which the seed records below are scoped to).
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
    let (_tenant, workspace) = store.ensure_workspace("ws", "/tmp/ws").await.unwrap();
    (MemoryStore::new(store), workspace)
}

fn pending_record(action_id: &str, requires_approval: bool) -> ActionRecord {
    ActionRecord {
        schema_key: ActionRecord::SCHEMA_KEY.to_string(),
        action_id: action_id.to_string(),
        job_id: "job-1".to_string(),
        item_id: None,
        action_type: if requires_approval {
            "post_listing".into()
        } else {
            "verify_price".into()
        },
        risk_level: if requires_approval {
            "medium".into()
        } else {
            "low".into()
        },
        requires_approval,
        state: "pending".to_string(),
        payload: serde_json::json!({}),
        approved_by: None,
        approved_at: None,
        executed_at: None,
        platform: None,
        result: None,
        evidence: Vec::new(),
        blocked_reason: None,
        created_at: chrono::Utc::now().to_rfc3339(),
    }
}

async fn seed(memory: &MemoryStore, ws: WorkspaceId, rec: &ActionRecord) {
    let body = serde_json::to_string(rec).unwrap();
    memory
        .append_for_vertical(ws, MemoryFile::Memory, body, None, FIXTURE_VERTICAL)
        .await
        .unwrap();
}

#[tokio::test]
async fn approve_advances_pending_approval_gated_action() {
    let (memory, ws) = open_test_memory().await;
    let seed_rec = pending_record("act-1", true);
    seed(&memory, ws, &seed_rec).await;

    let next = approve(&memory, ws, "act-1", "alice", Some("looks good"))
        .await
        .expect("approve");
    assert_eq!(next.state, "approved");
    assert_eq!(next.approved_by.as_deref(), Some("alice"));
    assert!(next.approved_at.is_some());
    assert_eq!(
        next.result.as_ref().and_then(|r| r.get("approval_note")),
        Some(&serde_json::Value::String("looks good".into()))
    );
}

#[tokio::test]
async fn approve_refuses_autonomous_safe_action() {
    let (memory, ws) = open_test_memory().await;
    let seed_rec = pending_record("act-2", false);
    seed(&memory, ws, &seed_rec).await;

    let err = approve(&memory, ws, "act-2", "alice", None)
        .await
        .expect_err("approve should refuse autonomous-safe");
    assert!(
        err.to_string().contains("autonomous-safe"),
        "error should explain why: got '{err}'"
    );
}

#[tokio::test]
async fn execute_refuses_pending_approval_gated_action() {
    let (memory, ws) = open_test_memory().await;
    let seed_rec = pending_record("act-3", true);
    seed(&memory, ws, &seed_rec).await;

    let err = execute(&memory, ws, "act-3")
        .await
        .expect_err("execute should refuse pending+requires_approval");
    let msg = err.to_string();
    assert!(msg.contains("approval required"), "got: {msg}");
}

#[tokio::test]
async fn execute_advances_autonomous_safe_pending_to_executing() {
    let (memory, ws) = open_test_memory().await;
    let seed_rec = pending_record("act-4", false);
    seed(&memory, ws, &seed_rec).await;

    let next = execute(&memory, ws, "act-4").await.expect("execute");
    assert_eq!(next.state, "executing");
    assert!(next.executed_at.is_some());
}

#[tokio::test]
async fn approve_then_execute_then_complete_round_trips() {
    let (memory, ws) = open_test_memory().await;
    let seed_rec = pending_record("act-5", true);
    seed(&memory, ws, &seed_rec).await;

    let _ = approve(&memory, ws, "act-5", "alice", None).await.unwrap();
    let _ = execute(&memory, ws, "act-5").await.unwrap();
    let done = complete(
        &memory,
        ws,
        "act-5",
        Some("posted to ebay"),
        Some(r#"{"listing_id":"abc-123"}"#),
    )
    .await
    .expect("complete");
    assert_eq!(done.state, "completed");
    let result = done.result.as_ref().expect("result populated");
    assert_eq!(result["summary"], "posted to ebay");
    assert_eq!(result["listing_id"], "abc-123");
}

#[tokio::test]
async fn fail_records_reason_on_executing_action() {
    let (memory, ws) = open_test_memory().await;
    let seed_rec = pending_record("act-6", false);
    seed(&memory, ws, &seed_rec).await;
    let _ = execute(&memory, ws, "act-6").await.unwrap();

    let failed = fail(&memory, ws, "act-6", "marketplace api 503")
        .await
        .expect("fail");
    assert_eq!(failed.state, "failed");
    assert_eq!(
        failed.result.as_ref().and_then(|r| r.get("failure")),
        Some(&serde_json::Value::String("marketplace api 503".into()))
    );
}

#[tokio::test]
async fn fail_refuses_when_action_is_not_executing() {
    let (memory, ws) = open_test_memory().await;
    let seed_rec = pending_record("act-7", true);
    seed(&memory, ws, &seed_rec).await;

    let err = fail(&memory, ws, "act-7", "nope")
        .await
        .expect_err("fail should refuse pending");
    assert!(err.to_string().contains("illegal transition"));
}

#[tokio::test]
async fn full_history_returns_every_state_row_in_time_order() {
    let (memory, ws) = open_test_memory().await;
    seed(&memory, ws, &pending_record("act-8", true)).await;
    let _ = approve(&memory, ws, "act-8", "alice", None).await.unwrap();
    let _ = execute(&memory, ws, "act-8").await.unwrap();
    let _ = complete(&memory, ws, "act-8", Some("done"), None)
        .await
        .unwrap();

    let history = full_history(&memory, ws, "act-8").await.unwrap();
    let states: Vec<&str> = history.iter().map(|r| r.state.as_str()).collect();
    assert_eq!(
        states,
        vec!["pending", "approved", "executing", "completed"]
    );
}

#[tokio::test]
async fn find_action_returns_none_for_unknown_id() {
    let (memory, ws) = open_test_memory().await;
    let v = find_action(&memory, ws, "missing").await.unwrap();
    assert!(v.is_none());
}

#[tokio::test]
async fn approve_refuses_unknown_action_id() {
    let (memory, ws) = open_test_memory().await;
    let err = approve(&memory, ws, "missing", "alice", None)
        .await
        .expect_err("approve should fail for unknown id");
    assert!(err.to_string().contains("missing"));
}

// ───────────────── retry chain counter (#755) ─────────────

#[test]
fn count_retry_chain_returns_one_for_an_action_with_no_retry_markers() {
    let r = pending_record("a-1", false);
    let n = crate::handlers::worker_actions_dispatch::count_retry_chain(&r);
    assert_eq!(n, 1, "no argo://action/ markers → first attempt");
}

#[test]
fn count_retry_chain_increments_per_argo_action_marker() {
    let mut r = pending_record("a-2", false);
    r.evidence = vec![
        "https://example.com/comp1".to_string(),
        "argo://action/00000000-0000-0000-0000-000000000001".to_string(),
        "argo://action/00000000-0000-0000-0000-000000000002".to_string(),
        "https://example.com/comp2".to_string(),
    ];
    let n = crate::handlers::worker_actions_dispatch::count_retry_chain(&r);
    // 1 (this attempt) + 2 prior retry markers = 3.
    assert_eq!(n, 3);
}

#[test]
fn count_retry_chain_ignores_non_retry_evidence_urls() {
    let mut r = pending_record("a-3", false);
    r.evidence = vec![
        "https://example.com/x".to_string(),
        "argo://session/foo".to_string(), // not action/
        "argohttps://".to_string(),       // doesn't start with the marker
    ];
    let n = crate::handlers::worker_actions_dispatch::count_retry_chain(&r);
    assert_eq!(n, 1);
}
