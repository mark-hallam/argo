//! Unit tests for [`crate::operator_risk_overrides`].

use super::*;
use crate::SessionStore;

async fn fresh_store() -> SessionStore {
    let store = SessionStore::open(":memory:").await.expect("open");
    store.migrate().await.expect("migrate");
    store
}

#[tokio::test]
async fn set_then_get_roundtrip() {
    let store = fresh_store().await;
    let s = OperatorRiskOverridesStore::new(store.pool());
    s.set("alice", "send_email", None, "high", false, 1_700_000_000)
        .await
        .expect("set");
    let rows = s.get_for_operator("alice").await.expect("get");
    assert_eq!(rows.len(), 1);
    let row = &rows[0];
    assert_eq!(row.operator_id, "alice");
    assert_eq!(row.action_kind, "send_email");
    assert_eq!(row.vertical_id, None);
    assert_eq!(row.risk_level, "high");
    assert!(!row.allow_downgrade);
    assert_eq!(row.created_at, 1_700_000_000);
}

#[tokio::test]
async fn upsert_overwrites_same_key() {
    let store = fresh_store().await;
    let s = OperatorRiskOverridesStore::new(store.pool());
    s.set(
        "alice",
        "book_travel",
        Some("legal"),
        "high",
        false,
        1_700_000_000,
    )
    .await
    .expect("first set");
    s.set(
        "alice",
        "book_travel",
        Some("legal"),
        "low",
        true,
        1_700_000_500,
    )
    .await
    .expect("upsert");
    let rows = s.get_for_operator("alice").await.expect("get");
    assert_eq!(rows.len(), 1);
    let row = &rows[0];
    assert_eq!(row.risk_level, "low");
    assert!(row.allow_downgrade);
    assert_eq!(row.created_at, 1_700_000_500);
    assert_eq!(row.vertical_id.as_deref(), Some("legal"));
}

#[tokio::test]
async fn list_sorts_across_operators_stably() {
    let store = fresh_store().await;
    let s = OperatorRiskOverridesStore::new(store.pool());
    s.set("bob", "send_email", None, "high", false, 1_700_000_100)
        .await
        .expect("set bob");
    s.set(
        "alice",
        "send_email",
        Some("legal"),
        "medium",
        false,
        1_700_000_200,
    )
    .await
    .expect("set alice per-vertical");
    s.set("alice", "send_email", None, "high", false, 1_700_000_300)
        .await
        .expect("set alice all-verticals");
    let rows = s.list().await.expect("list");
    assert_eq!(rows.len(), 3);
    // alice rows come first (operator_id ASC); within alice, the
    // empty-string vertical_id sorts before "legal".
    assert_eq!(rows[0].operator_id, "alice");
    assert_eq!(rows[0].vertical_id, None);
    assert_eq!(rows[1].operator_id, "alice");
    assert_eq!(rows[1].vertical_id.as_deref(), Some("legal"));
    assert_eq!(rows[2].operator_id, "bob");
}

#[tokio::test]
async fn delete_returns_true_only_when_row_existed() {
    let store = fresh_store().await;
    let s = OperatorRiskOverridesStore::new(store.pool());
    s.set("alice", "send_email", None, "high", false, 1_700_000_000)
        .await
        .expect("set");
    assert!(s
        .delete("alice", "send_email", None)
        .await
        .expect("delete first"));
    assert!(!s
        .delete("alice", "send_email", None)
        .await
        .expect("delete second"));
    assert!(s.get_for_operator("alice").await.expect("get").is_empty());
    // Wrong vertical scope must NOT delete the all-verticals row.
    s.set("alice", "send_email", None, "high", false, 1_700_000_400)
        .await
        .expect("re-set");
    assert!(!s
        .delete("alice", "send_email", Some("legal"))
        .await
        .expect("delete wrong scope"));
    assert_eq!(
        s.get_for_operator("alice").await.expect("get").len(),
        1,
        "all-verticals row should survive a per-vertical delete"
    );
}

#[tokio::test]
async fn invalid_risk_level_is_rejected() {
    let store = fresh_store().await;
    let s = OperatorRiskOverridesStore::new(store.pool());
    let err = s
        .set("alice", "send_email", None, "critical", false, 1_700_000_000)
        .await
        .expect_err("should reject critical");
    let msg = err.to_string();
    assert!(
        msg.contains("low / medium / high"),
        "error message should list valid levels, got: {msg}"
    );
}
