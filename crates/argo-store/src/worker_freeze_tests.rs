use super::*;
use crate::SessionStore;
use sqlx::SqlitePool;

async fn fresh_pool() -> SqlitePool {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    store.pool().clone()
}

#[tokio::test]
async fn freeze_then_is_frozen_returns_some() {
    let pool = fresh_pool().await;

    let row = freeze(&pool, "tenant-a", "worker-1", "principal:owner", "pause")
        .await
        .unwrap();
    let fetched = is_frozen(&pool, "tenant-a", "worker-1")
        .await
        .unwrap()
        .expect("active freeze");

    assert_eq!(row, fetched);
    assert_eq!(fetched.tenant_id, "tenant-a");
    assert_eq!(fetched.worker_id, "worker-1");
    assert_eq!(fetched.frozen_by, "principal:owner");
    assert_eq!(fetched.reason, "pause");
    assert!(fetched.unfrozen_at.is_none());
    assert!(fetched.unfrozen_by.is_none());
}

#[tokio::test]
async fn unfreeze_makes_is_frozen_return_none() {
    let pool = fresh_pool().await;
    freeze(&pool, "tenant-a", "worker-1", "principal:owner", "pause")
        .await
        .unwrap();

    assert!(unfreeze(&pool, "tenant-a", "worker-1", "principal:owner")
        .await
        .unwrap());
    assert!(is_frozen(&pool, "tenant-a", "worker-1")
        .await
        .unwrap()
        .is_none());
    assert!(
        !unfreeze(&pool, "tenant-a", "worker-1", "principal:owner")
            .await
            .unwrap(),
        "second unfreeze should be idempotent"
    );
}

#[tokio::test]
async fn double_freeze_updates_existing_row() {
    let pool = fresh_pool().await;
    let first = freeze(&pool, "tenant-a", "worker-1", "actor-a", "first")
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(2)).await;
    let second = freeze(&pool, "tenant-a", "worker-1", "actor-b", "second")
        .await
        .unwrap();

    assert_ne!(first.frozen_at, second.frozen_at);
    assert_eq!(second.frozen_by, "actor-b");
    assert_eq!(second.reason, "second");
    let rows = list_frozen(&pool, "tenant-a").await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0], second);
}

#[tokio::test]
async fn list_frozen_returns_only_active_rows() {
    let pool = fresh_pool().await;
    freeze(&pool, "tenant-a", "worker-a", "actor", "active")
        .await
        .unwrap();
    freeze(&pool, "tenant-a", "worker-b", "actor", "inactive")
        .await
        .unwrap();
    unfreeze(&pool, "tenant-a", "worker-b", "actor")
        .await
        .unwrap();

    let rows = list_frozen(&pool, "tenant-a").await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].worker_id, "worker-a");
    assert_eq!(rows[0].reason, "active");
}

#[tokio::test]
async fn freeze_is_tenant_scoped() {
    let pool = fresh_pool().await;
    freeze(&pool, "tenant-a", "worker-1", "actor-a", "tenant-a freeze")
        .await
        .unwrap();

    assert!(is_frozen(&pool, "tenant-a", "worker-1")
        .await
        .unwrap()
        .is_some());
    assert!(is_frozen(&pool, "tenant-b", "worker-1")
        .await
        .unwrap()
        .is_none());

    freeze(&pool, "tenant-b", "worker-1", "actor-b", "tenant-b freeze")
        .await
        .unwrap();
    let tenant_a = list_frozen(&pool, "tenant-a").await.unwrap();
    let tenant_b = list_frozen(&pool, "tenant-b").await.unwrap();
    assert_eq!(tenant_a.len(), 1);
    assert_eq!(tenant_b.len(), 1);
    assert_eq!(tenant_a[0].reason, "tenant-a freeze");
    assert_eq!(tenant_b[0].reason, "tenant-b freeze");
}
