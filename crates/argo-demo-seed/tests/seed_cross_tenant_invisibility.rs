//! Cross-tenant invisibility regression for `argo_demo_seed::seed_workers`.
//!
//! This is the load-bearing acceptance test for brief
//! `mission-demo-seed-crate-scaffold` — without it the seed is just
//! code, not evidence that the seeded rows respect the
//! multi-tenant-by-construction invariant. Verifies three properties:
//!
//! 1. Seeding tenant A produces exactly six workers, names matching
//!    the canonical [`DEMO_PERSONAS`] spine.
//! 2. Tenant B sees zero workers (different tenant, complete
//!    isolation).
//! 3. Re-seeding tenant A returns the same six worker IDs in the
//!    same order — no duplicates, no new rows.

use argo_demo_seed::{seed_workers, DEMO_PERSONAS};
use argo_identity::IdentityStore;
use argo_store::SessionStore;
use argo_types::TenantId;

async fn fresh_store() -> SessionStore {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    store
}

#[tokio::test]
async fn tenant_a_sees_exactly_six_named_workers() {
    let store = fresh_store().await;
    let tenant_a = TenantId::new();

    let ids = seed_workers(&store, tenant_a).await.unwrap();
    assert_eq!(ids.len(), 6);

    let identities = IdentityStore::new(&store);
    let listed = identities.list_workers(tenant_a, false).await.unwrap();
    assert_eq!(listed.len(), 6, "tenant A should see exactly 6 workers");

    let mut names: Vec<String> =
        listed.iter().map(|w| w.persona.name.clone()).collect();
    names.sort();
    let mut expected: Vec<String> =
        DEMO_PERSONAS.iter().map(|p| p.name.to_string()).collect();
    expected.sort();
    assert_eq!(
        names, expected,
        "tenant A worker names must match the canonical 6-persona spine",
    );

    for worker in &listed {
        assert_eq!(
            worker.tenant_id, tenant_a,
            "every seeded worker must carry tenant A's id",
        );
    }
}

#[tokio::test]
async fn tenant_b_sees_zero_workers_after_seeding_tenant_a() {
    let store = fresh_store().await;
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();

    let _ = seed_workers(&store, tenant_a).await.unwrap();

    let identities = IdentityStore::new(&store);
    let tenant_b_workers =
        identities.list_workers(tenant_b, false).await.unwrap();
    assert!(
        tenant_b_workers.is_empty(),
        "tenant B must see zero workers after only tenant A was seeded; got {} rows",
        tenant_b_workers.len(),
    );
}

#[tokio::test]
async fn reseeding_tenant_a_returns_same_six_ids() {
    let store = fresh_store().await;
    let tenant_a = TenantId::new();

    let first = seed_workers(&store, tenant_a).await.unwrap();
    let second = seed_workers(&store, tenant_a).await.unwrap();

    assert_eq!(
        first, second,
        "re-seeding must return the same six worker IDs in the same order",
    );

    let identities = IdentityStore::new(&store);
    let listed = identities.list_workers(tenant_a, false).await.unwrap();
    assert_eq!(
        listed.len(),
        6,
        "re-seed must not insert duplicate rows; got {} workers, expected 6",
        listed.len(),
    );
}

#[tokio::test]
async fn parallel_tenant_seeds_remain_isolated() {
    let store = fresh_store().await;
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();

    let ids_a = seed_workers(&store, tenant_a).await.unwrap();
    let ids_b = seed_workers(&store, tenant_b).await.unwrap();

    assert_eq!(ids_a.len(), 6);
    assert_eq!(ids_b.len(), 6);

    let intersection: Vec<_> =
        ids_a.iter().filter(|id| ids_b.contains(id)).collect();
    assert!(
        intersection.is_empty(),
        "tenant A and tenant B worker ids must not overlap; got {} shared ids",
        intersection.len(),
    );

    let identities = IdentityStore::new(&store);
    let a_listed = identities.list_workers(tenant_a, false).await.unwrap();
    let b_listed = identities.list_workers(tenant_b, false).await.unwrap();
    assert_eq!(a_listed.len(), 6);
    assert_eq!(b_listed.len(), 6);

    for w in &a_listed {
        assert_eq!(w.tenant_id, tenant_a);
    }
    for w in &b_listed {
        assert_eq!(w.tenant_id, tenant_b);
    }
}
