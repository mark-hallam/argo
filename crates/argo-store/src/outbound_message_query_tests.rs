//! Tests for the `outbound_message` idempotency helpers.

use super::*;
use crate::SessionStore;
use uuid::Uuid;

async fn fresh_store() -> SessionStore {
    let store = SessionStore::open(":memory:").await.expect("open store");
    store.migrate().await.expect("migrate");
    ensure_outbound_message_schema(&store)
        .await
        .expect("ensure outbound_message");
    store
}

#[tokio::test]
async fn ensure_schema_is_idempotent() {
    let store = fresh_store().await;
    ensure_outbound_message_schema(&store).await.unwrap();
    ensure_outbound_message_schema(&store).await.unwrap();
}

#[tokio::test]
async fn insert_then_lookup_returns_row_for_same_tenant() {
    let store = fresh_store().await;
    let tenant = TenantId(Uuid::now_v7());
    insert_outbound_message(
        &store,
        "msg-1",
        tenant,
        "slack",
        "C123",
        "deadbeef",
        100,
        r#"{"message_id":"msg-1","sent_at":"2026-05-17T00:00:00Z"}"#,
    )
    .await
    .unwrap();
    let row = lookup_recent_by_body_hash(&store, tenant, "deadbeef", 0)
        .await
        .unwrap()
        .expect("row present");
    assert_eq!(row.message_id, "msg-1");
    assert_eq!(row.channel, "slack");
    assert_eq!(row.recipient, "C123");
    assert_eq!(row.body_hash, "deadbeef");
    assert_eq!(row.sent_at, 100);
}

#[tokio::test]
async fn lookup_isolates_by_tenant_id() {
    let store = fresh_store().await;
    let a = TenantId(Uuid::now_v7());
    let b = TenantId(Uuid::now_v7());
    insert_outbound_message(&store, "msg-a", a, "slack", "C1", "hash-1", 100, "{}")
        .await
        .unwrap();
    let other = lookup_recent_by_body_hash(&store, b, "hash-1", 0).await.unwrap();
    assert!(other.is_none(), "cross-tenant lookup leaked: {other:?}");
}

#[tokio::test]
async fn lookup_filters_by_since_window() {
    let store = fresh_store().await;
    let tenant = TenantId(Uuid::now_v7());
    insert_outbound_message(&store, "old", tenant, "email", "u@x", "hash-w", 100, "{}")
        .await
        .unwrap();
    let stale = lookup_recent_by_body_hash(&store, tenant, "hash-w", 200)
        .await
        .unwrap();
    assert!(stale.is_none(), "row older than since_unix_secs leaked");
    let in_window = lookup_recent_by_body_hash(&store, tenant, "hash-w", 50)
        .await
        .unwrap();
    assert!(in_window.is_some());
}

#[tokio::test]
async fn insert_or_ignore_swallows_duplicate_message_id() {
    let store = fresh_store().await;
    let tenant = TenantId(Uuid::now_v7());
    insert_outbound_message(&store, "msg-x", tenant, "sms", "+15550000", "h", 100, "{}")
        .await
        .unwrap();
    // Second insert with same primary key must be a no-op, not an error.
    insert_outbound_message(
        &store,
        "msg-x",
        tenant,
        "sms",
        "+15550000",
        "h",
        200,
        r#"{"second":"call"}"#,
    )
    .await
    .unwrap();
    // The original row wins — sent_at stays at 100.
    let row = lookup_recent_by_body_hash(&store, tenant, "h", 0)
        .await
        .unwrap()
        .expect("row");
    assert_eq!(row.sent_at, 100);
    assert_eq!(row.outcome_json, "{}");
}

#[tokio::test]
async fn lookup_returns_most_recent_when_multiple_match() {
    let store = fresh_store().await;
    let tenant = TenantId(Uuid::now_v7());
    insert_outbound_message(&store, "first", tenant, "slack", "C1", "h2", 100, r#"{"n":1}"#)
        .await
        .unwrap();
    insert_outbound_message(&store, "second", tenant, "slack", "C1", "h2", 500, r#"{"n":2}"#)
        .await
        .unwrap();
    let row = lookup_recent_by_body_hash(&store, tenant, "h2", 0)
        .await
        .unwrap()
        .expect("row");
    assert_eq!(row.message_id, "second");
    assert_eq!(row.sent_at, 500);
}
