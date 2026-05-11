//! Unit tests for [`super`] (`tenant_rate_limit` store).
#![cfg(test)]
use super::{TenantRateLimitOverride, TenantRateLimitStore};
use crate::SessionStore;
use argo_types::TenantId;
use sqlx::{Pool, Sqlite};

async fn fresh_pool() -> Pool<Sqlite> {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    store.pool().clone()
}

#[tokio::test]
async fn set_then_get_round_trips_row() {
    let pool = fresh_pool().await;
    let store = TenantRateLimitStore::new(&pool);
    let tenant = TenantId::new();
    store.set(tenant, 1500, 1_700_000_000).await.unwrap();
    let got = store.get(tenant).await.unwrap().expect("row should exist");
    assert_eq!(
        got,
        TenantRateLimitOverride {
            tenant_id: tenant,
            requests_per_minute: 1500,
            updated_at: 1_700_000_000,
        }
    );
}

#[tokio::test]
async fn set_upserts_on_conflict() {
    let pool = fresh_pool().await;
    let store = TenantRateLimitStore::new(&pool);
    let tenant = TenantId::new();
    store.set(tenant, 900, 100).await.unwrap();
    store.set(tenant, 1800, 200).await.unwrap();
    let got = store.get(tenant).await.unwrap().expect("row should exist");
    assert_eq!(got.requests_per_minute, 1800);
    assert_eq!(got.updated_at, 200, "second set should win on conflict");
    let all = store.list().await.unwrap();
    assert_eq!(all.len(), 1, "upsert should not duplicate the row");
}

#[tokio::test]
async fn list_and_delete_clear_the_row() {
    let pool = fresh_pool().await;
    let store = TenantRateLimitStore::new(&pool);
    let a = TenantId::new();
    let b = TenantId::new();
    store.set(a, 500, 1).await.unwrap();
    store.set(b, 700, 2).await.unwrap();
    let all = store.list().await.unwrap();
    assert_eq!(all.len(), 2);
    assert!(store.delete(a).await.unwrap(), "delete should report 1 row");
    assert!(
        !store.delete(a).await.unwrap(),
        "re-delete should report 0 rows"
    );
    let rest = store.list().await.unwrap();
    assert_eq!(rest.len(), 1);
    assert_eq!(rest[0].tenant_id, b);
}

#[tokio::test]
async fn set_rejects_zero_rpm() {
    let pool = fresh_pool().await;
    let store = TenantRateLimitStore::new(&pool);
    let tenant = TenantId::new();
    let err = store.set(tenant, 0, 0).await.unwrap_err();
    let msg = format!("{err}");
    assert!(
        msg.contains("requests_per_minute must be > 0"),
        "error should reject zero rpm, got: {msg}"
    );
    assert!(
        store.get(tenant).await.unwrap().is_none(),
        "rejected set must not write a row"
    );
}
