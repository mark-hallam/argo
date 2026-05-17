//! Connector-aware tests for [`super`] (`tenant_rate_limit` store).
#![cfg(test)]

use super::TenantRateLimitStore;
use crate::SessionStore;
use argo_types::TenantId;
use sqlx::{Pool, Sqlite};

async fn fresh_pool() -> Pool<Sqlite> {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    store.pool().clone()
}

#[tokio::test]
async fn connector_override_falls_back_to_wildcard() {
    let pool = fresh_pool().await;
    let store = TenantRateLimitStore::new(&pool);
    let tenant = TenantId::new();
    store.set(tenant, 60, 10).await.unwrap();
    let row = store
        .get_for_connector(tenant, "gmail")
        .await
        .unwrap()
        .expect("wildcard row");
    assert_eq!(row.connector, "*");
    assert_eq!(row.requests_per_minute, 60);

    store
        .set_for_connector(tenant, "gmail", 100, 11)
        .await
        .unwrap();
    let row = store
        .get_for_connector(tenant, "gmail")
        .await
        .unwrap()
        .expect("connector row");
    assert_eq!(row.requests_per_minute, 100);
    assert_eq!(row.updated_at, 11);
}

#[tokio::test]
async fn connector_rows_can_coexist_for_same_tenant() {
    let pool = fresh_pool().await;
    let store = TenantRateLimitStore::new(&pool);
    let tenant = TenantId::new();
    store
        .set_for_connector(tenant, "gmail", 100, 1)
        .await
        .unwrap();
    store
        .set_for_connector(tenant, "slack", 30, 2)
        .await
        .unwrap();

    assert_eq!(
        store
            .get_for_connector(tenant, "gmail")
            .await
            .unwrap()
            .unwrap()
            .requests_per_minute,
        100
    );
    assert_eq!(
        store
            .get_for_connector(tenant, "slack")
            .await
            .unwrap()
            .unwrap()
            .requests_per_minute,
        30
    );
    let mut rows = store.list_connector_overrides().await.unwrap();
    rows.sort_by(|a, b| a.connector.cmp(&b.connector));
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].connector, "gmail");
    assert_eq!(rows[1].connector, "slack");
}
