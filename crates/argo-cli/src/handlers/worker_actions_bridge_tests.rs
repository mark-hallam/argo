//! Tests for the chat→delivery bridge (WAVE-2 ⑥a). Loaded via
//! `#[path = "worker_actions_bridge_tests.rs"] mod tests;` from
//! `worker_actions_bridge.rs`.
//!
//! The load-bearing proof is [`enqueue_targets_real_executor_and_parks_at_gate`]:
//! calling the bridge yields an action that (a) targets a REAL executor (not
//! `human`) AND (b) is in pending-approval state (it did NOT auto-execute).

use super::*;
use crate::handlers::worker_actions_transitions;
use argo_memory::MemoryStore;
use argo_store::SessionStore;
use argo_types::WorkspaceId;
use std::sync::Arc;

/// A fixture vertical with three shapes the bridge must distinguish:
/// * `export_report` — approval-gated AND bound to a REAL executor
///   (`document_csv`). This is the only one the bridge may enqueue.
/// * `draft_note` — approval-gated but `executor = "human"` (the
///   born-`human`-and-stop dead-end the bridge refuses).
/// * `read_inventory` — bound to a real executor but `requires_approval =
///   false` (an auto-deliver the bridge refuses).
fn fixture_vertical() -> argo_vertical::VerticalManifest {
    argo_vertical::parse(
        r#"
schema_version = "1"

[vertical]
id      = "bridgefix"
name    = "bridge fixture"
version = "0.0.1"

[actions.export_report]
risk              = "medium"
requires_approval = true
description       = "Write the customer CSV report."
executor          = "document_csv"

[actions.draft_note]
risk              = "low"
requires_approval = true
description       = "Draft a note for a human to send."
executor          = "human"

[actions.read_inventory]
risk              = "low"
requires_approval = false
description       = "Read-only inventory snapshot."
executor          = "document_csv"
"#,
    )
    .expect("fixture manifest parses")
}

async fn open_test_memory() -> (MemoryStore, WorkspaceId) {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    let store = Arc::new(store);
    let (_tenant, workspace) = store.ensure_workspace("ws", "/tmp/ws").await.unwrap();
    (MemoryStore::new(store), workspace)
}

const VID: &str = "bridgefix";

/// THE safety proof. Calling the bridge for an approval-gated, real-executor
/// action yields a persisted row that:
///   (a) targets a REAL executor (`document_csv`, NOT `human`), AND
///   (b) is in pending-approval state (`state = "pending"`,
///       `requires_approval = true`) — i.e. it did NOT auto-execute.
#[tokio::test]
async fn enqueue_targets_real_executor_and_parks_at_gate() {
    let (memory, ws) = open_test_memory().await;
    let manifest = fixture_vertical();

    let outcome = enqueue_action_for_delivery(
        &memory,
        ws,
        &manifest,
        VID,
        "job-bridge-1",
        argo_identity::WorkerId::nil(),
        "export_report",
        serde_json::json!({ "rows": 3 }),
    )
    .await
    .expect("bridge enqueues the approval-gated real-executor action");

    let BridgeOutcome::SubmittedForApproval {
        record,
        executor_kind,
    } = outcome;

    // (a) targets a REAL executor — not the human dead-end.
    assert_eq!(executor_kind, "document_csv");
    assert_ne!(executor_kind, "human");

    // (b) parked at the approval gate — did NOT auto-execute.
    assert_eq!(record.state, "pending");
    assert!(record.requires_approval);
    assert!(record.executed_at.is_none());
    assert!(record.approved_by.is_none());

    // And it is actually PERSISTED to the ledger (so /approvals + `argo
    // worker actions list` see it) at pending — re-read it back.
    let located = worker_actions_transitions::find_action(&memory, ws, &record.action_id)
        .await
        .unwrap()
        .expect("the enqueued action is in the ledger");
    assert_eq!(located.record.state, "pending");
    assert!(located.record.requires_approval);
    assert_eq!(located.record.action_type, "export_report");
}

/// SAFETY: the bridge refuses the born-`human`-and-stop dead-end (the bug it
/// exists to fix). Nothing is persisted.
#[tokio::test]
async fn enqueue_refuses_human_dead_end() {
    let (memory, ws) = open_test_memory().await;
    let manifest = fixture_vertical();

    let err = enqueue_action_for_delivery(
        &memory,
        ws,
        &manifest,
        VID,
        "job-bridge-2",
        argo_identity::WorkerId::nil(),
        "draft_note",
        serde_json::json!({}),
    )
    .await
    .expect_err("bridge must refuse executor=human");
    assert!(
        format!("{err}").contains("executor=human"),
        "error names the human dead-end: {err}"
    );
}

/// SAFETY: the bridge refuses an action the manifest marks
/// `requires_approval = false` — it will not be the vehicle for an un-gated
/// auto-deliver. Nothing is persisted.
#[tokio::test]
async fn enqueue_refuses_auto_deliver() {
    let (memory, ws) = open_test_memory().await;
    let manifest = fixture_vertical();

    let err = enqueue_action_for_delivery(
        &memory,
        ws,
        &manifest,
        VID,
        "job-bridge-3",
        argo_identity::WorkerId::nil(),
        "read_inventory",
        serde_json::json!({}),
    )
    .await
    .expect_err("bridge must refuse requires_approval=false");
    assert!(
        format!("{err}").contains("requires_approval=false"),
        "error names the auto-deliver refusal: {err}"
    );
}

/// SAFETY: an undeclared action_type fails closed — the bridge cannot route
/// something the manifest never declared.
#[tokio::test]
async fn enqueue_refuses_undeclared_action() {
    let (memory, ws) = open_test_memory().await;
    let manifest = fixture_vertical();

    let err = enqueue_action_for_delivery(
        &memory,
        ws,
        &manifest,
        VID,
        "job-bridge-4",
        argo_identity::WorkerId::nil(),
        "launch_missiles",
        serde_json::json!({}),
    )
    .await
    .expect_err("bridge must refuse an undeclared action_type");
    assert!(
        format!("{err}").contains("not declared"),
        "error names the undeclared action: {err}"
    );
}
