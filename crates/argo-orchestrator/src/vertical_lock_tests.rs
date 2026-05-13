use std::sync::Arc;
use std::time::{Duration, Instant};

use argo_store::SessionStore;
use argo_types::TenantId;
use sqlx::Row;

use crate::{LockConfig, VerticalConcurrencyLock};

async fn fresh_lock() -> (VerticalConcurrencyLock, Arc<SessionStore>) {
    let store = Arc::new(SessionStore::open(":memory:").await.expect("store"));
    store.migrate().await.expect("migrate");
    let lock = VerticalConcurrencyLock::with_config(
        Arc::clone(&store),
        LockConfig {
            lease_secs: 30,
            acquire_timeout_secs: 1,
        },
    );
    (lock, store)
}

async fn lock_count(store: &SessionStore) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM vertical_locks")
        .fetch_one(store.pool())
        .await
        .expect("count")
}

async fn wait_for_empty(store: &SessionStore) {
    for _ in 0..20 {
        if lock_count(store).await == 0 {
            return;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    assert_eq!(lock_count(store).await, 0);
}

#[tokio::test]
async fn acquire_release_reacquire_round_trip() {
    let (lock, store) = fresh_lock().await;
    let tenant = TenantId::new();
    let guard = lock
        .acquire(tenant, "support", "maya")
        .await
        .expect("acquire");
    assert_eq!(lock_count(&store).await, 1);
    drop(guard);
    wait_for_empty(&store).await;
    let guard = lock
        .acquire(tenant, "support", "maya")
        .await
        .expect("reacquire");
    drop(guard);
}

#[tokio::test]
async fn second_acquire_while_held_times_out() {
    let (lock, _store) = fresh_lock().await;
    let tenant = TenantId::new();
    let guard = lock
        .acquire(tenant, "support", "maya")
        .await
        .expect("first");
    let started = Instant::now();
    let result = lock.acquire(tenant, "support", "maya").await;
    let err = match result {
        Ok(_) => panic!("busy lock must time out"),
        Err(err) => err,
    };
    assert!(started.elapsed() >= Duration::from_secs(1));
    assert!(err.to_string().contains("vertical-lock contention timeout"));
    drop(guard);
}

#[tokio::test]
async fn stale_lease_recovery_overwrites_abandoned_row() {
    let (lock, store) = fresh_lock().await;
    let tenant = TenantId::new();
    sqlx::query(
        "INSERT INTO vertical_locks
         (tenant_id, vertical_id, worker_id, holder_pid, acquired_at, lease_secs)
         VALUES (?, 'support', 'maya', 999, strftime('%s','now') - 60, 30)",
    )
    .bind(tenant.to_string())
    .execute(store.pool())
    .await
    .expect("seed stale row");
    let guard = lock
        .acquire(tenant, "support", "maya")
        .await
        .expect("acquire");
    let pid: i64 = sqlx::query("SELECT holder_pid FROM vertical_locks")
        .fetch_one(store.pool())
        .await
        .expect("row")
        .get("holder_pid");
    assert_eq!(pid, i64::from(std::process::id()));
    drop(guard);
}

#[tokio::test]
async fn per_tenant_isolation() {
    let (lock, _store) = fresh_lock().await;
    let a = lock
        .acquire(TenantId::new(), "support", "maya")
        .await
        .expect("tenant a");
    let b = lock
        .acquire(TenantId::new(), "support", "maya")
        .await
        .expect("tenant b");
    drop((a, b));
}

#[tokio::test]
async fn per_vertical_isolation() {
    let (lock, _store) = fresh_lock().await;
    let tenant = TenantId::new();
    let a = lock
        .acquire(tenant, "support", "maya")
        .await
        .expect("support");
    let b = lock
        .acquire(tenant, "recruiting", "maya")
        .await
        .expect("recruiting");
    drop((a, b));
}

#[tokio::test]
async fn per_worker_isolation() {
    let (lock, _store) = fresh_lock().await;
    let tenant = TenantId::new();
    let a = lock.acquire(tenant, "support", "maya").await.expect("maya");
    let b = lock
        .acquire(tenant, "support", "riley")
        .await
        .expect("riley");
    drop((a, b));
}
