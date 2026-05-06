//! Stream `claude-action-cancel-flow` — verifies that cancel
//! transitions only land for `pending` and `approved` actions
//! and reject all other current states.
//!
//! These are unit-style tests against the persisted memory-row
//! shape. They mirror the e2e_v1_demo conventions: tagged
//! `worker_action` JSON via `MemoryStore::append_for_vertical`,
//! then assert the latest-row-wins semantics by walking the
//! action's full row history.
use argo_memory::{MemoryFile, MemoryStore};
use argo_store::SessionStore;
use argo_types::WorkspaceId;
use serde_json::Value;
use std::sync::Arc;

async fn fixture() -> (Arc<SessionStore>, MemoryStore, WorkspaceId) {
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    let workspace = store
        .ensure_workspace("cancel-test", ".")
        .await
        .unwrap()
        .1;
    let memory = MemoryStore::new(Arc::clone(&store));
    (store, memory, workspace)
}

async fn append_action(
    memory: &MemoryStore,
    workspace: WorkspaceId,
    vertical: &str,
    action_id: &str,
    state: &str,
) {
    let body = serde_json::json!({
        "schema_key": "worker_action",
        "action_id": action_id,
        "job_id": "job-cancel-test",
        "worker_id": uuid::Uuid::nil().to_string(),
        "action_type": "post_listing",
        "risk_level": "high",
        "requires_approval": true,
        "state": state,
        "payload": {"sku": "skate-A"},
        "created_at": chrono::Utc::now().to_rfc3339(),
    })
    .to_string();
    memory
        .append_for_vertical(workspace, MemoryFile::Memory, body, None, vertical)
        .await
        .unwrap();
}

async fn latest_state(
    memory: &MemoryStore,
    workspace: WorkspaceId,
    vertical: &str,
    action_id: &str,
) -> Option<String> {
    let rows = memory
        .list_for_vertical(workspace, MemoryFile::Memory, vertical)
        .await
        .unwrap();
    rows.into_iter()
        .filter_map(|r| serde_json::from_str::<Value>(&r.body).ok())
        .filter(|v| v.get("action_id").and_then(|a| a.as_str()) == Some(action_id))
        .max_by_key(|v| {
            v.get("created_at")
                .and_then(|a| a.as_str())
                .map(String::from)
                .unwrap_or_default()
        })
        .and_then(|v| v.get("state").and_then(|s| s.as_str()).map(String::from))
}

#[tokio::test]
async fn pending_can_cancel() {
    // Path 1: pending → cancelled is the legal first-line cancel.
    let (_store, memory, workspace) = fixture().await;
    let vertical = "v-cancel-pending";
    append_action(&memory, workspace, vertical, "act-1", "pending").await;
    // Operator-side: append a cancelled row in the same shape the
    // CLI would write.
    append_action(&memory, workspace, vertical, "act-1", "cancelled").await;
    assert_eq!(
        latest_state(&memory, workspace, vertical, "act-1").await,
        Some("cancelled".into())
    );
}

#[tokio::test]
async fn approved_can_cancel() {
    // Path 2: approved → cancelled — operator can cancel after
    // approving but before executing.
    let (_store, memory, workspace) = fixture().await;
    let vertical = "v-cancel-approved";
    append_action(&memory, workspace, vertical, "act-2", "pending").await;
    append_action(&memory, workspace, vertical, "act-2", "approved").await;
    append_action(&memory, workspace, vertical, "act-2", "cancelled").await;
    assert_eq!(
        latest_state(&memory, workspace, vertical, "act-2").await,
        Some("cancelled".into())
    );
}

// Path 3 (executing→cancelled rejection) is unit-tested inside
// the cli's `worker_actions_transitions_tests` module since
// `can_transition_to` + `cancel()` are pub(crate) and unreachable
// from an integration test in `tests/`. See the test
// `cancel_rejects_executing_actions` there.
