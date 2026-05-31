//! Tests for the record-readiness prune (`prune_demo_tenant_to_canonical`).
//!
//! Covers the spec's headline case (insert a stray → seed/prune → exactly
//! the canonical roster remains live), idempotency (second prune = 0), the
//! canonical-protection invariant (a canonical worker is never pruned), the
//! tenant-scope invariant (a stray under a different tenant is untouched),
//! and the fail-closed guard (empty canonical set prunes nothing).

use super::*;
use argo_identity::{IdentityStore, Persona, Role, Worker};
use std::sync::Arc;

use crate::personas::DEMO_SEED_PERSONAS;

async fn open_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    Arc::new(store)
}

async fn create_worker(store: &Arc<SessionStore>, tenant: TenantId, name: &str, vertical: &str) {
    let worker = Worker::draft(
        tenant,
        vertical,
        Persona::named(name, "test"),
        Role::new("role"),
    );
    IdentityStore::new(store)
        .create_worker(&worker)
        .await
        .unwrap();
}

async fn seed_canonical_roster(store: &Arc<SessionStore>, tenant: TenantId) {
    for spine in DEMO_SEED_PERSONAS {
        create_worker(store, tenant, spine.name, spine.vertical_id).await;
    }
}

fn canonical_names() -> Vec<&'static str> {
    DEMO_SEED_PERSONAS.iter().map(|p| p.name).collect()
}

#[tokio::test]
async fn stray_worker_is_pruned_to_exactly_canonical() {
    let store = open_store().await;
    let tenant = TenantId::new();
    seed_canonical_roster(&store, tenant).await;
    // A stray Genesis-hired worker like "Maya" + an extra.
    create_worker(&store, tenant, "Maya", "support").await;
    create_worker(&store, tenant, "StaleBot", "marketing").await;

    let identities = IdentityStore::new(&store);
    assert_eq!(
        identities.list_workers(tenant, false).await.unwrap().len(),
        DEMO_SEED_PERSONAS.len() + 2,
        "precondition: roster + 2 strays"
    );

    let pruned = prune_demo_tenant_to_canonical(&store, tenant, &canonical_names())
        .await
        .unwrap();
    assert_eq!(pruned, 2, "exactly the 2 strays should be pruned");

    let live = identities.list_workers(tenant, false).await.unwrap();
    assert_eq!(live.len(), DEMO_SEED_PERSONAS.len(), "exactly the canonical roster remains live");
    let names: std::collections::HashSet<&str> =
        live.iter().map(|w| w.persona.name.as_str()).collect();
    assert!(!names.contains("Maya"), "Maya must be gone from the live set");
    assert!(!names.contains("StaleBot"));
    for spine in DEMO_SEED_PERSONAS {
        assert!(names.contains(spine.name), "canonical {} must survive", spine.name);
    }
}

#[tokio::test]
async fn prune_is_idempotent() {
    let store = open_store().await;
    let tenant = TenantId::new();
    seed_canonical_roster(&store, tenant).await;
    create_worker(&store, tenant, "Maya", "support").await;

    let first = prune_demo_tenant_to_canonical(&store, tenant, &canonical_names())
        .await
        .unwrap();
    assert_eq!(first, 1);
    let second = prune_demo_tenant_to_canonical(&store, tenant, &canonical_names())
        .await
        .unwrap();
    assert_eq!(second, 0, "second prune finds no strays");
}

#[tokio::test]
async fn clean_tenant_prunes_nothing() {
    let store = open_store().await;
    let tenant = TenantId::new();
    seed_canonical_roster(&store, tenant).await;
    let pruned = prune_demo_tenant_to_canonical(&store, tenant, &canonical_names())
        .await
        .unwrap();
    assert_eq!(pruned, 0, "an already-canonical tenant is untouched");
}

#[tokio::test]
async fn empty_canonical_set_fails_closed() {
    let store = open_store().await;
    let tenant = TenantId::new();
    seed_canonical_roster(&store, tenant).await;
    create_worker(&store, tenant, "Maya", "support").await;

    // Fail-closed: an empty canonical set must prune NOTHING (never
    // "delete everything").
    let pruned = prune_demo_tenant_to_canonical(&store, tenant, &[]).await.unwrap();
    assert_eq!(pruned, 0);
    let live = IdentityStore::new(&store)
        .list_workers(tenant, false)
        .await
        .unwrap();
    assert_eq!(
        live.len(),
        DEMO_SEED_PERSONAS.len() + 1,
        "empty canonical set must leave every worker live"
    );
}

#[tokio::test]
async fn prune_is_tenant_scoped() {
    let store = open_store().await;
    let demo = TenantId::new();
    let other = TenantId::new();
    seed_canonical_roster(&store, demo).await;
    // A stray under a DIFFERENT tenant must not be touched by a prune of
    // the demo tenant.
    create_worker(&store, other, "OtherTenantStray", "support").await;

    let pruned = prune_demo_tenant_to_canonical(&store, demo, &canonical_names())
        .await
        .unwrap();
    assert_eq!(pruned, 0, "demo tenant was already clean");

    let other_live = IdentityStore::new(&store)
        .list_workers(other, false)
        .await
        .unwrap();
    assert_eq!(other_live.len(), 1, "other tenant's worker must be untouched");
}
