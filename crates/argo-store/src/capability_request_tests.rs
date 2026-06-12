use super::*;
use crate::SessionStore;
use sqlx::SqlitePool;

async fn fresh_pool() -> SqlitePool {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    store.pool().clone()
}

#[tokio::test]
async fn record_then_list_returns_the_pending_row() {
    let pool = fresh_pool().await;

    let row = record_pending(&pool, "tenant-a", "worker-1", "salesforce", "write", Some("run-1"))
        .await
        .unwrap();
    assert_eq!(row.tenant_id, "tenant-a");
    assert_eq!(row.worker_id, "worker-1");
    assert_eq!(row.tool_or_connector, "salesforce");
    assert_eq!(row.requested_access, "write");
    assert_eq!(row.run_id.as_deref(), Some("run-1"));
    assert_eq!(row.status, "pending");

    let pending = list_pending(&pool, "tenant-a", "worker-1").await.unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0], row);
}

#[tokio::test]
async fn second_deny_bumps_not_duplicates() {
    let pool = fresh_pool().await;

    let first = record_pending(&pool, "tenant-a", "worker-1", "gmail", "read", None)
        .await
        .unwrap();
    // A re-deny of the SAME tool updates the existing row, never inserts
    // a second — acceptance: "second deny bumps, not duplicates".
    let second = record_pending(&pool, "tenant-a", "worker-1", "gmail", "write", Some("run-2"))
        .await
        .unwrap();

    assert_eq!(first.id, second.id, "same row id across re-denies");
    assert_eq!(first.created_utc, second.created_utc, "created_utc preserved");
    assert_eq!(second.requested_access, "write", "latest access wins");
    assert_eq!(second.run_id.as_deref(), Some("run-2"));

    let pending = list_pending(&pool, "tenant-a", "worker-1").await.unwrap();
    assert_eq!(pending.len(), 1, "exactly one row for (worker, tool)");
}

#[tokio::test]
async fn distinct_tools_are_distinct_rows() {
    let pool = fresh_pool().await;
    record_pending(&pool, "tenant-a", "worker-1", "salesforce", "write", None)
        .await
        .unwrap();
    record_pending(&pool, "tenant-a", "worker-1", "gmail", "read", None)
        .await
        .unwrap();

    let pending = list_pending(&pool, "tenant-a", "worker-1").await.unwrap();
    assert_eq!(pending.len(), 2);
}

#[tokio::test]
async fn list_is_tenant_scoped() {
    let pool = fresh_pool().await;
    record_pending(&pool, "tenant-a", "worker-1", "salesforce", "write", None)
        .await
        .unwrap();

    // A different tenant with the SAME worker id string sees nothing —
    // the UNIQUE key-space and the list filter are both tenant-first.
    let other = list_pending(&pool, "tenant-b", "worker-1").await.unwrap();
    assert!(other.is_empty());
}

#[tokio::test]
async fn rejects_bad_access() {
    let pool = fresh_pool().await;
    let err = record_pending(&pool, "tenant-a", "worker-1", "salesforce", "execute", None)
        .await
        .unwrap_err();
    assert!(matches!(err, ArgoError::Schema(_)));
}

#[tokio::test]
async fn rejects_empty_required_fields() {
    let pool = fresh_pool().await;
    assert!(record_pending(&pool, "", "worker-1", "salesforce", "read", None)
        .await
        .is_err());
    assert!(record_pending(&pool, "tenant-a", "", "salesforce", "read", None)
        .await
        .is_err());
    assert!(record_pending(&pool, "tenant-a", "worker-1", "", "read", None)
        .await
        .is_err());
}
