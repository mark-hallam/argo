//! Integration tests for [`crate::tenant_store`].

use super::{get_tenant, insert_tenant, list_tenants, set_tenant_status};
use crate::tenant::{Tenant, TenantStatus};
use argo_store::SessionStore;
use chrono::{TimeZone, Utc};

async fn store() -> SessionStore {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    store
}

#[tokio::test]
async fn insert_get_round_trips() {
    let store = store().await;
    let t = Tenant::new("Acme".into(), "billing@acme.com".into());
    insert_tenant(&store, &t).await.unwrap();
    let fetched = get_tenant(&store, t.id).await.unwrap().expect("row");
    assert_eq!(fetched, t);
}

#[tokio::test]
async fn get_missing_returns_none() {
    let store = store().await;
    let id = argo_types::TenantId::new();
    assert!(get_tenant(&store, id).await.unwrap().is_none());
}

#[tokio::test]
async fn list_returns_every_row_newest_first() {
    let store = store().await;
    let mut a = Tenant::new("Acme".into(), "a@acme.com".into());
    let mut b = Tenant::new("Bravo".into(), "b@bravo.com".into());
    // Force ordering: a is older.
    a.created_at -= 60;
    a.updated_at = a.created_at;
    b.created_at -= 10;
    b.updated_at = b.created_at;
    insert_tenant(&store, &a).await.unwrap();
    insert_tenant(&store, &b).await.unwrap();
    let rows = list_tenants(&store).await.unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].id, b.id);
    assert_eq!(rows[1].id, a.id);
}

#[tokio::test]
async fn set_status_updates_row_and_bumps_updated_at() {
    let store = store().await;
    let t = Tenant::new("Acme".into(), "billing@acme.com".into());
    insert_tenant(&store, &t).await.unwrap();
    let later = Utc.with_ymd_and_hms(2030, 1, 1, 0, 0, 0).unwrap();
    let affected = set_tenant_status(&store, t.id, TenantStatus::Suspended, later)
        .await
        .unwrap();
    assert_eq!(affected, 1);
    let row = get_tenant(&store, t.id).await.unwrap().expect("row");
    assert_eq!(row.status, TenantStatus::Suspended);
    assert_eq!(row.updated_at, later.timestamp());
    assert_eq!(row.created_at, t.created_at);
}

#[tokio::test]
async fn set_status_on_missing_id_returns_zero() {
    let store = store().await;
    let id = argo_types::TenantId::new();
    let now = Utc::now();
    let affected = set_tenant_status(&store, id, TenantStatus::Deleted, now)
        .await
        .unwrap();
    assert_eq!(affected, 0);
}
