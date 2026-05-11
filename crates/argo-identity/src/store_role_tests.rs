//! Unit tests for `IdentityStore::set_worker_role_name` /
//! `IdentityStore::workers_with_role` (Stream
//! `mission-routing-skillmatch-rolematch`, Tier M4). Sibling to
//! `store_role.rs` so the production file stays small — mirrors
//! `store_skill_tests.rs`.
#![cfg(test)]

use crate::{IdentityStore, Persona, Role, Worker};
use argo_store::SessionStore;
use argo_types::{ArgoError, TenantId};

async fn seed_worker(store: &SessionStore, tenant: TenantId, name: &str) -> Worker {
    let worker = Worker::draft(
        tenant,
        "gtm-engineer",
        Persona::named(name, "test-built"),
        Role::new("Close approved deals."),
    );
    IdentityStore::new(store)
        .create_worker(&worker)
        .await
        .unwrap();
    worker
}

async fn fresh_store() -> SessionStore {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    store
}

#[tokio::test]
async fn set_worker_role_name_persists() {
    let store = fresh_store().await;
    let tenant = TenantId::new();
    let worker = seed_worker(&store, tenant, "Ada").await;
    let identities = IdentityStore::new(&store);

    identities
        .set_worker_role_name(tenant, worker.id, Some("on-call-sre"))
        .await
        .unwrap();

    let reloaded = identities
        .get_worker(tenant, worker.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(reloaded.role_name.as_deref(), Some("on-call-sre"));
}

#[tokio::test]
async fn set_worker_role_name_clears_with_none() {
    let store = fresh_store().await;
    let tenant = TenantId::new();
    let worker = seed_worker(&store, tenant, "Bea").await;
    let identities = IdentityStore::new(&store);

    identities
        .set_worker_role_name(tenant, worker.id, Some("temp"))
        .await
        .unwrap();
    identities
        .set_worker_role_name(tenant, worker.id, None)
        .await
        .unwrap();

    let reloaded = identities
        .get_worker(tenant, worker.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(reloaded.role_name, None);
}

#[tokio::test]
async fn set_worker_role_name_unknown_worker_returns_not_found() {
    let store = fresh_store().await;
    let tenant = TenantId::new();
    let identities = IdentityStore::new(&store);
    let result = identities
        .set_worker_role_name(tenant, crate::WorkerId::new(), Some("foo"))
        .await;
    assert!(matches!(result, Err(ArgoError::NotFound(_))));
}

#[tokio::test]
async fn workers_with_role_filters_by_tenant_and_role() {
    let store = fresh_store().await;
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    let worker_a = seed_worker(&store, tenant_a, "Dee").await;
    let worker_b = seed_worker(&store, tenant_b, "Eve").await;
    let worker_c = seed_worker(&store, tenant_a, "Faye").await;
    let identities = IdentityStore::new(&store);

    identities
        .set_worker_role_name(tenant_a, worker_a.id, Some("triager"))
        .await
        .unwrap();
    identities
        .set_worker_role_name(tenant_b, worker_b.id, Some("triager"))
        .await
        .unwrap();
    identities
        .set_worker_role_name(tenant_a, worker_c.id, Some("editor"))
        .await
        .unwrap();

    let scoped = identities
        .workers_with_role(tenant_a, "triager")
        .await
        .unwrap();
    assert_eq!(scoped.len(), 1);
    assert_eq!(scoped[0].id, worker_a.id);
}

#[tokio::test]
async fn workers_with_role_returns_empty_when_no_match() {
    let store = fresh_store().await;
    let tenant = TenantId::new();
    let _worker = seed_worker(&store, tenant, "Gus").await;
    let identities = IdentityStore::new(&store);
    let none = identities
        .workers_with_role(tenant, "no-such-role")
        .await
        .unwrap();
    assert!(none.is_empty());
}
