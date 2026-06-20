//! cu5 (DEC-050 G5) approve-and-execute tests, split from
//! `routes_approvals_json_tests.rs` to keep it under the 500-LOC ceiling.
//! A sibling child module of `routes_approvals_json`; reuses the test
//! harness helpers via `super::tests::*`.
use super::decide_response;
use super::tests::{auth_post, parse, seed_action, set_test_env};
use crate::handlers::worker_actions_transitions::open_workspace_memory;

/// cu5 (DEC-050 G5): approving a held action EXECUTES it in one click —
/// the response carries an `execution` field and the worker-action row
/// advances past `approved` (the `send_outreach` fixture resolves to the
/// keyless `human` executor, which writes a real handoff bundle and
/// leaves the row at `executing`).
#[tokio::test]
async fn decide_approve_executes_held_action() {
    let _g = crate::handlers::gateway::shared_env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    seed_action("act-run").await;
    let (status, body) = decide_response(
        "act-run",
        &auth_post(
            "/api/v1/approvals/act-run/decide",
            r#"{"decision":"approve"}"#,
        ),
    )
    .await;
    assert_eq!(status, "200 OK");
    let v = parse(&body);
    assert_eq!(v["ok"], true);
    assert_eq!(v["decision"], "approve");
    // The approval re-dispatched the action through its executor.
    assert_eq!(
        v["execution"], "dispatched",
        "approve must report the execution outcome, got {:?}",
        v["execution"]
    );
    // The worker-action ledger moved past `approved` — the human executor
    // leaves an approval-gated action at `executing` (HumanHandoff).
    let (memory, ws) = open_workspace_memory().await.expect("memory");
    let state = crate::handlers::worker_actions_transitions::find_action(&memory, ws, "act-run")
        .await
        .unwrap()
        .expect("action present")
        .record
        .state;
    assert_eq!(state, "executing", "approve-and-run advanced the ledger");
}

/// cu5: re-approving an action that has ALREADY been approved-and-executed
/// stays idempotent (200), not a spurious 409. After execution the row is
/// `executing`/`completed`/`failed`, not `approved`, so the idempotency
/// check must recognise the preserved `approved_by` stamp.
#[tokio::test]
async fn decide_reapprove_after_execution_is_idempotent_200() {
    let _g = crate::handlers::gateway::shared_env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    seed_action("act-rerun").await;
    let req = auth_post(
        "/api/v1/approvals/act-rerun/decide",
        r#"{"decision":"approve"}"#,
    );
    let (s1, _) = decide_response("act-rerun", &req).await;
    assert_eq!(s1, "200 OK");
    let (s2, b2) = decide_response("act-rerun", &req).await;
    assert_eq!(s2, "200 OK", "re-approve after execution must be idempotent");
    assert_eq!(parse(&b2)["decision"], "approve");
}
