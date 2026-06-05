//! Spend-ledger tests for [`super::SessionStore`] — split out of
//! `lib_tests.rs` to keep that file under the 500-LOC ceiling. Covers
//! session/day/month aggregation, per-teammate `worker_id` attribution
//! (v90, DEC-038), tenant isolation, and the rolling day window.
#![cfg(test)]
use super::*;

async fn fresh_store_with_workspace() -> (SessionStore, TenantId, WorkspaceId) {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    let tenant = TenantId::new();
    let ws = WorkspaceId::new();
    sqlx::query(
        "INSERT INTO workspace (id, tenant_id, name, root_path, created_at)
         VALUES (?, ?, 'test', '.', strftime('%s','now'))",
    )
    .bind(ws.0.as_bytes().as_slice())
    .bind(tenant.0.as_bytes().as_slice())
    .execute(store.pool())
    .await
    .unwrap();
    (store, tenant, ws)
}

#[tokio::test]
async fn spend_ledger_records_and_aggregates_session_total() {
    let (store, tenant, ws) = fresh_store_with_workspace().await;
    let session = store
        .create_session(tenant, ws, "m", "p", None, None)
        .await
        .unwrap();
    let now = chrono::Utc::now();
    store
        .record_spend(
            tenant,
            session,
            now.timestamp_micros(),
            250_000,
            "anth",
            "haiku",
        )
        .await
        .unwrap();
    store
        .record_spend(
            tenant,
            session,
            now.timestamp_micros(),
            750_000,
            "anth",
            "haiku",
        )
        .await
        .unwrap();
    let counters = store
        .load_spend_counters(tenant, session, now)
        .await
        .unwrap();
    assert_eq!(counters.session_used, 1_000_000);
    assert_eq!(counters.day_used, 1_000_000);
    assert_eq!(counters.month_used, 1_000_000);
}

#[tokio::test]
async fn spend_ledger_attributes_worker_id_and_is_null_safe() {
    // Per-teammate cost attribution (v90, DEC-038): a teammate turn
    // writes its `worker_id`, a system session writes NULL, and the
    // existing tenant rollup still sums both rows unchanged.
    let (store, tenant, ws) = fresh_store_with_workspace().await;
    let session = store
        .create_session(tenant, ws, "m", "p", None, None)
        .await
        .unwrap();
    let now = chrono::Utc::now();
    let worker = WorkerId::new();
    store
        .record_spend_with_worker(
            tenant,
            session,
            now.timestamp_micros(),
            400_000,
            "anth",
            "haiku",
            Some(worker),
        )
        .await
        .unwrap();
    // A system session (no teammate) records NULL via the plain path.
    store
        .record_spend(tenant, session, now.timestamp_micros(), 100_000, "anth", "haiku")
        .await
        .unwrap();

    // The teammate row carries the worker_id BLOB.
    let attributed: Option<Vec<u8>> = sqlx::query_scalar(
        "SELECT worker_id FROM spend_ledger WHERE amount_usd_micros = 400000",
    )
    .fetch_one(store.pool())
    .await
    .unwrap();
    assert_eq!(attributed.as_deref(), Some(worker.0.as_bytes().as_slice()));

    // The system row is NULL.
    let system: Option<Vec<u8>> = sqlx::query_scalar(
        "SELECT worker_id FROM spend_ledger WHERE amount_usd_micros = 100000",
    )
    .fetch_one(store.pool())
    .await
    .unwrap();
    assert!(system.is_none());

    // Existing tenant rollup is null-safe: both rows still sum.
    let counters = store
        .load_spend_counters(tenant, session, now)
        .await
        .unwrap();
    assert_eq!(counters.session_used, 500_000);
}

#[tokio::test]
async fn spend_ledger_isolates_by_tenant() {
    let (store, tenant_a, ws_a) = fresh_store_with_workspace().await;
    let session_a = store
        .create_session(tenant_a, ws_a, "m", "p", None, None)
        .await
        .unwrap();
    let tenant_b = TenantId::new();
    let ws_b = WorkspaceId::new();
    sqlx::query(
        "INSERT INTO workspace (id, tenant_id, name, root_path, created_at)
         VALUES (?, ?, 'b', '.', strftime('%s','now'))",
    )
    .bind(ws_b.0.as_bytes().as_slice())
    .bind(tenant_b.0.as_bytes().as_slice())
    .execute(store.pool())
    .await
    .unwrap();
    let session_b = store
        .create_session(tenant_b, ws_b, "m", "p", None, None)
        .await
        .unwrap();
    let now = chrono::Utc::now();
    store
        .record_spend(
            tenant_b,
            session_b,
            now.timestamp_micros(),
            5_000_000,
            "p",
            "m",
        )
        .await
        .unwrap();
    let a = store
        .load_spend_counters(tenant_a, session_a, now)
        .await
        .unwrap();
    assert_eq!(a.session_used, 0);
    assert_eq!(a.day_used, 0);
    assert_eq!(a.month_used, 0);
}

#[tokio::test]
async fn spend_ledger_day_window_excludes_yesterday() {
    let (store, tenant, ws) = fresh_store_with_workspace().await;
    let session = store
        .create_session(tenant, ws, "m", "p", None, None)
        .await
        .unwrap();
    let now = chrono::Utc::now();
    let yesterday = now - chrono::Duration::days(2);
    store
        .record_spend(tenant, session, yesterday.timestamp_micros(), 123, "p", "m")
        .await
        .unwrap();
    let counters = store
        .load_spend_counters(tenant, session, now)
        .await
        .unwrap();
    assert_eq!(counters.session_used, 123);
    assert_eq!(counters.day_used, 0);
}
