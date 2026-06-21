//! Tests for [`BridgeDeliverDispatcher`] — the `argo-cli` backend the
//! in-turn `enqueue_action` tool dispatches into. Loaded via
//! `#[path = "worker_actions_bridge_dispatcher_tests.rs"]`.
//!
//! The load-bearing proof
//! ([`dispatcher_enqueues_real_executor_and_parks_at_gate`]) drives the
//! SAME path the live tool drives — `DeliverDispatcher::enqueue` — and
//! asserts the action targets a real executor AND is parked pending
//! approval (it did NOT auto-execute).

use super::*;
use crate::handlers::worker_actions_transitions;
use argo_store::SessionStore;

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
"#,
    )
    .expect("fixture manifest parses")
}

async fn open_store_ws() -> (Arc<SessionStore>, WorkspaceId) {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    let store = Arc::new(store);
    let (_tenant, workspace) = store.ensure_workspace("ws", "/tmp/ws").await.unwrap();
    (store, workspace)
}

fn make_dispatcher(
    store: Arc<SessionStore>,
    ws: WorkspaceId,
) -> BridgeDeliverDispatcher {
    BridgeDeliverDispatcher::new(
        store,
        ws,
        Arc::new(fixture_vertical()),
        "bridgefix".to_string(),
        "job-disp-1".to_string(),
        argo_identity::WorkerId::nil(),
    )
}

#[tokio::test]
async fn dispatcher_enqueues_real_executor_and_parks_at_gate() {
    let (store, ws) = open_store_ws().await;
    let dispatcher = make_dispatcher(Arc::clone(&store), ws);

    let outcome = dispatcher
        .enqueue("export_report", serde_json::json!({ "rows": 3 }))
        .await
        .expect("dispatcher submits the approval-gated real-executor action");

    // Targets a REAL executor (not the human dead-end).
    assert_eq!(outcome.executor_kind, "document_csv");
    assert!(!outcome.action_id.is_empty());

    // The persisted row is PENDING (did NOT auto-execute) — re-read it.
    let memory = MemoryStore::new(store);
    let located = worker_actions_transitions::find_action(&memory, ws, &outcome.action_id)
        .await
        .unwrap()
        .expect("the enqueued action is in the ledger");
    assert_eq!(located.record.state, "pending");
    assert!(located.record.requires_approval);
    assert!(located.record.executed_at.is_none());
    assert!(located.record.approved_by.is_none());
    assert_eq!(located.record.action_type, "export_report");
    assert_eq!(located.record.job_id, "job-disp-1");
}

#[tokio::test]
async fn dispatcher_surfaces_fail_closed_human_dead_end_as_error() {
    let (store, ws) = open_store_ws().await;
    let dispatcher = make_dispatcher(store, ws);

    let err = dispatcher
        .enqueue("draft_note", serde_json::json!({}))
        .await
        .expect_err("dispatcher must refuse executor=human");
    assert!(
        err.contains("executor=human"),
        "error names the human dead-end: {err}"
    );
}

#[tokio::test]
async fn dispatcher_surfaces_undeclared_action_as_error() {
    let (store, ws) = open_store_ws().await;
    let dispatcher = make_dispatcher(store, ws);

    let err = dispatcher
        .enqueue("not_a_declared_action", serde_json::json!({}))
        .await
        .expect_err("dispatcher must refuse an undeclared action_type");
    assert!(
        err.contains("not declared"),
        "error names the undeclared action: {err}"
    );
}
