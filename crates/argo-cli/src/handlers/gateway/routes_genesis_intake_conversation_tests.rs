//! Tests for the conversational-intake turn handler — wizard path (no provider)
//! + per-tenant persistence + isolation.

use super::*;
use serde_json::Value;

fn store() -> (tempfile::TempDir, IntakeStore) {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = IntakeStore::new(dir.path());
    (dir, store)
}

/// Drive one wizard turn (no provider) and return the JSON envelope.
async fn turn(store: &IntakeStore, tenant: &str, conv: &str, message: Option<&str>, confirm: bool) -> Value {
    let (status, value) = run_turn(store, tenant, conv, message, confirm, None)
        .await
        .expect("turn succeeds");
    assert_eq!(status, "200 OK");
    value
}

#[tokio::test]
async fn opening_turn_asks_for_the_charter() {
    let (_d, store) = store();
    let v = turn(&store, "op-a", "c1", None, false).await;
    assert_eq!(v["status"], "asking");
    assert_eq!(v["slot"], "charter");
    assert!(v["assistant_message"].as_str().is_some_and(|s| !s.is_empty()));
}

#[tokio::test]
async fn wizard_walk_reaches_ready_then_done() {
    let (_d, store) = store();
    let tenant = "op-a";
    let conv = "walk";
    // Opening question.
    turn(&store, tenant, conv, None, false).await;
    // Answer each slot in order; the deterministic wizard advances one per turn.
    let answers = [
        "Be my support agent who clears the inbound ticket queue",
        "Sam",
        "Answer tier-1 tickets",
        "weekdays on each inbound ticket",
        "never issue a refund alone",
        "first response under five minutes",
    ];
    let mut last = Value::Null;
    for a in answers {
        last = turn(&store, tenant, conv, Some(a), false).await;
    }
    // All slots filled → ready, with an assembled spec carrying the slots.
    assert_eq!(last["status"], "ready");
    assert_eq!(last["spec"]["persona"], "sam");
    assert_eq!(last["captured"]["cadence"], "weekdays on each inbound ticket");
    assert!(last["spec"]["capabilities"].as_array().is_some_and(|a| !a.is_empty()));

    // Confirming flips ready → done.
    let done = turn(&store, tenant, conv, None, true).await;
    assert_eq!(done["status"], "done");
    assert_eq!(done["spec"]["persona"], "sam");
}

#[tokio::test]
async fn conversation_persists_across_turns() {
    let (_d, store) = store();
    turn(&store, "op-a", "persist", None, false).await;
    turn(&store, "op-a", "persist", Some("Be my recruiter screener for inbound applicants"), false).await;
    // Re-load: the charter answer survived between turns.
    let reloaded = store.load("op-a", "persist").expect("persisted state");
    assert!(reloaded.charter.is_some());
}

#[tokio::test]
async fn tenant_b_starts_a_fresh_conversation_for_the_same_id() {
    let (_d, store) = store();
    // Tenant A fills the charter under a shared conversation id.
    turn(&store, "op-a", "shared", Some("Be my bookkeeping assistant for invoices"), false).await;
    // Tenant B, same id → fresh conversation, still asking for the charter.
    let b = turn(&store, "op-b", "shared", None, false).await;
    assert_eq!(b["slot"], "charter");
    assert!(store.load("op-b", "shared").is_some());
    // Tenant A's intake is untouched and unreadable by B's path.
    let a = store.load("op-a", "shared").expect("tenant A state");
    assert!(a.charter.is_some());
}
