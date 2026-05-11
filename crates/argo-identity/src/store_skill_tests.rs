//! Unit tests for `IdentityStore` worker↔skill methods (Stream
//! `mvp-worker-skills-table`). Sibling file to `store.rs` so the
//! production file stays under the 500-LOC ceiling — mirrors
//! `store_archive_tests.rs` from `mvp-identity-archive-worker`.
#![cfg(test)]

use crate::{IdentityStore, Persona, Role, Worker};
use argo_store::SessionStore;
use argo_types::TenantId;

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
async fn add_worker_skill_persists() {
    let store = fresh_store().await;
    let tenant = TenantId::new();
    let worker = seed_worker(&store, tenant, "Ada").await;
    let identities = IdentityStore::new(&store);

    identities
        .add_worker_skill(worker.id, "follow_up", 3)
        .await
        .unwrap();

    let skills = identities.skills_for_worker(worker.id).await.unwrap();
    assert_eq!(skills, vec![("follow_up".to_string(), 3)]);
}

#[tokio::test]
async fn add_worker_skill_is_idempotent() {
    let store = fresh_store().await;
    let tenant = TenantId::new();
    let worker = seed_worker(&store, tenant, "Bea").await;
    let identities = IdentityStore::new(&store);

    identities
        .add_worker_skill(worker.id, "follow_up", 1)
        .await
        .unwrap();
    identities
        .add_worker_skill(worker.id, "follow_up", 4)
        .await
        .unwrap();

    let skills = identities.skills_for_worker(worker.id).await.unwrap();
    assert_eq!(
        skills,
        vec![("follow_up".to_string(), 4)],
        "second add must upsert proficiency, not duplicate the row"
    );
}

#[tokio::test]
async fn remove_worker_skill_succeeds() {
    let store = fresh_store().await;
    let tenant = TenantId::new();
    let worker = seed_worker(&store, tenant, "Cy").await;
    let identities = IdentityStore::new(&store);

    identities
        .add_worker_skill(worker.id, "follow_up", 2)
        .await
        .unwrap();
    identities
        .remove_worker_skill(worker.id, "follow_up")
        .await
        .unwrap();

    assert!(identities
        .skills_for_worker(worker.id)
        .await
        .unwrap()
        .is_empty());

    // Removing again is a no-op (DELETE matches no rows).
    identities
        .remove_worker_skill(worker.id, "follow_up")
        .await
        .unwrap();
}

#[tokio::test]
async fn workers_with_skill_filters_by_tenant() {
    let store = fresh_store().await;
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    let worker_a = seed_worker(&store, tenant_a, "Dee").await;
    let worker_b = seed_worker(&store, tenant_b, "Eve").await;
    let identities = IdentityStore::new(&store);

    identities
        .add_worker_skill(worker_a.id, "follow_up", 3)
        .await
        .unwrap();
    identities
        .add_worker_skill(worker_b.id, "follow_up", 3)
        .await
        .unwrap();

    let scoped = identities
        .workers_with_skill(tenant_a, "follow_up")
        .await
        .unwrap();
    assert_eq!(scoped.len(), 1);
    assert_eq!(scoped[0].id, worker_a.id);
}

#[tokio::test]
async fn workers_with_skill_returns_empty_when_no_match() {
    let store = fresh_store().await;
    let tenant = TenantId::new();
    let worker = seed_worker(&store, tenant, "Fey").await;
    let identities = IdentityStore::new(&store);

    identities
        .add_worker_skill(worker.id, "follow_up", 3)
        .await
        .unwrap();

    let none = identities
        .workers_with_skill(tenant, "unknown_skill")
        .await
        .unwrap();
    assert!(none.is_empty());
}

#[tokio::test]
async fn skills_for_worker_returns_all_associations() {
    let store = fresh_store().await;
    let tenant = TenantId::new();
    let worker = seed_worker(&store, tenant, "Gus").await;
    let identities = IdentityStore::new(&store);

    identities
        .add_worker_skill(worker.id, "follow_up", 2)
        .await
        .unwrap();
    identities
        .add_worker_skill(worker.id, "qualify", 5)
        .await
        .unwrap();
    identities
        .add_worker_skill(worker.id, "schedule", 1)
        .await
        .unwrap();

    let skills = identities.skills_for_worker(worker.id).await.unwrap();
    assert_eq!(
        skills,
        vec![
            ("follow_up".to_string(), 2),
            ("qualify".to_string(), 5),
            ("schedule".to_string(), 1),
        ],
        "ordered by skill_id ASC for determinism"
    );
}
