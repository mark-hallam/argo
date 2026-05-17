//! `billing_dlq_query` round-trip tests against an in-memory store.

use super::*;
use argo_types::TenantId;
use chrono::Utc;

async fn fresh_store() -> SessionStore {
    let store = SessionStore::open(":memory:").await.expect("memory store");
    store.migrate().await.expect("migrate");
    store
}

#[tokio::test]
async fn upsert_then_fetch_round_trip() {
    let store = fresh_store().await;
    let tenant = TenantId::new();
    let now = Utc::now();
    let count = upsert_dead_letter(
        &store,
        "evt_a",
        "customer.subscription.updated",
        Some(tenant),
        r#"{"id":"evt_a"}"#,
        now,
        "tenant locked",
    )
    .await
    .expect("upsert");
    assert_eq!(count, 1);

    let entry = fetch_dead_letter(&store, "evt_a")
        .await
        .expect("fetch")
        .expect("present");
    assert_eq!(entry.event_id, "evt_a");
    assert_eq!(entry.event_type, "customer.subscription.updated");
    assert_eq!(entry.tenant_id, Some(tenant));
    assert_eq!(entry.raw_payload, r#"{"id":"evt_a"}"#);
    assert_eq!(entry.retry_count, 1);
    assert_eq!(entry.resolved_at, None);
}

#[tokio::test]
async fn upsert_increments_retry_count() {
    let store = fresh_store().await;
    let now = Utc::now();
    upsert_dead_letter(
        &store, "evt_b", "customer.subscription.updated",
        None, r#"{"id":"evt_b"}"#, now, "first",
    ).await.expect("first insert");
    let count = upsert_dead_letter(
        &store, "evt_b", "customer.subscription.updated",
        None, r#"{"id":"evt_b"}"#, now, "second",
    ).await.expect("second upsert");
    assert_eq!(count, 2);
    let entry = fetch_dead_letter(&store, "evt_b").await.expect("fetch").expect("row");
    assert_eq!(entry.retry_count, 2);
    assert_eq!(entry.failure_reason, "second");
}

#[tokio::test]
async fn list_filters_resolved_and_window() {
    let store = fresh_store().await;
    let now = Utc::now();
    upsert_dead_letter(
        &store, "evt_old", "customer.subscription.updated",
        None, r#"{}"#, now - chrono::Duration::hours(48), "old",
    ).await.expect("old");
    upsert_dead_letter(
        &store, "evt_new", "customer.subscription.updated",
        None, r#"{}"#, now, "new",
    ).await.expect("new");
    mark_resolved(&store, "evt_old", now).await.expect("resolve");

    let unresolved = list_dead_letters(&store, None, false, 100).await.expect("list");
    assert_eq!(unresolved.len(), 1);
    assert_eq!(unresolved[0].event_id, "evt_new");

    let all = list_dead_letters(&store, None, true, 100).await.expect("list all");
    assert_eq!(all.len(), 2);

    let since = (now - chrono::Duration::hours(1)).timestamp();
    let recent = list_dead_letters(&store, Some(since), true, 100).await.expect("recent");
    assert_eq!(recent.len(), 1);
    assert_eq!(recent[0].event_id, "evt_new");
}

#[tokio::test]
async fn mark_resolved_is_idempotent() {
    let store = fresh_store().await;
    let now = Utc::now();
    upsert_dead_letter(
        &store, "evt_c", "customer.subscription.updated",
        None, r#"{}"#, now, "x",
    ).await.expect("insert");
    assert!(mark_resolved(&store, "evt_c", now).await.expect("first"));
    assert!(!mark_resolved(&store, "evt_c", now).await.expect("second"));
    let entry = fetch_dead_letter(&store, "evt_c").await.expect("fetch").expect("row");
    assert!(entry.resolved_at.is_some());
}
