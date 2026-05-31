//! Tests for `seed_trust_grades` — proves the canonical demo seed sets
//! an honest non-zero `worker.trust_grade` per persona (the Trust tab
//! reads this column), that no grade is inflated to the reserved
//! `Builtin` ladder index, and that the seed is idempotent.

use super::{grade_for_persona, seed_trust_grades};
use crate::personas::DEMO_SEED_PERSONAS;
use argo_identity::{IdentityStore, Persona, Role, Worker};
use argo_store::SessionStore;
use argo_types::TenantId;
use std::sync::Arc;

async fn store_with_workers() -> (Arc<SessionStore>, TenantId, Vec<argo_identity::WorkerId>) {
    let store = Arc::new(SessionStore::open(":memory:").await.expect("open"));
    store.migrate().await.expect("migrate");
    let tenant = TenantId::new();
    let identities = IdentityStore::new(&store);
    identities.ensure_schema().await.expect("identity schema");
    let mut ids = Vec::new();
    for p in DEMO_SEED_PERSONAS {
        let worker = Worker::draft(
            tenant,
            p.vertical_id,
            Persona::named(p.name, "argo-demo-seed"),
            Role::new("seed"),
        );
        ids.push(worker.id);
        identities.create_worker(&worker).await.expect("create");
    }
    (store, tenant, ids)
}

#[tokio::test]
async fn sets_a_grade_for_every_persona() {
    let (store, tenant, ids) = store_with_workers().await;
    let set = seed_trust_grades(&store, tenant, DEMO_SEED_PERSONAS, &ids)
        .await
        .expect("seed grades");
    // Every persona in the canonical roster has an assigned grade.
    assert_eq!(set, DEMO_SEED_PERSONAS.len());

    let identities = IdentityStore::new(&store);
    for (p, id) in DEMO_SEED_PERSONAS.iter().zip(ids.iter()) {
        let worker = identities
            .get_worker(tenant, *id)
            .await
            .expect("get")
            .expect("worker present");
        let expected = grade_for_persona(p.name).expect("persona has a grade");
        assert_eq!(
            worker.trust_grade, expected,
            "{} should be grade {expected}",
            p.name
        );
    }
}

#[test]
fn grades_are_honest_not_inflated() {
    // No teammate is promoted to the reserved Builtin index (4); the
    // floor for a seeded teammate is Published community (1) — none
    // left at the unreviewed (0) default.
    for p in DEMO_SEED_PERSONAS {
        let g = grade_for_persona(p.name).expect("every persona graded");
        assert!((1..=3).contains(&g), "{} grade {g} out of the honest 1..=3 band", p.name);
    }
    // The headline established teammates sit at Published verified (3).
    assert_eq!(grade_for_persona("Mariah"), Some(3));
    assert_eq!(grade_for_persona("Portia"), Some(3));
    // The newest draft-only teammate sits at Published community (1).
    assert_eq!(grade_for_persona("Marcus"), Some(1));
}

#[tokio::test]
async fn seed_is_idempotent() {
    let (store, tenant, ids) = store_with_workers().await;
    let first = seed_trust_grades(&store, tenant, DEMO_SEED_PERSONAS, &ids)
        .await
        .expect("first");
    let second = seed_trust_grades(&store, tenant, DEMO_SEED_PERSONAS, &ids)
        .await
        .expect("second");
    // Re-applying the same value is a no-op UPDATE, not a duplicate —
    // the count is stable and the grades don't drift.
    assert_eq!(first, second, "re-seed sets the same grades");

    let identities = IdentityStore::new(&store);
    let mariah = identities
        .get_worker(tenant, ids[0])
        .await
        .expect("get")
        .expect("worker present");
    assert_eq!(mariah.trust_grade, 3, "Mariah grade stable after re-seed");
}
