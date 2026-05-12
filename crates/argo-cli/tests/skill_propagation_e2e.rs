//! Stream `mission-skill-propagation-loop` — end-to-end assertion
//! of mission claim 4 (cross-worker skill propagation).
//!
//! ## What this proves
//!
//! Worker A in tenant T installs skill X carrying a
//! `min_trust_grade` floor. The propagation contract is then that
//! `SkillStore::available_for_worker` returns skill X for every
//! other worker in T whose `worker.trust_grade` clears the floor,
//! and filters it out for every worker that does not — without any
//! per-worker copy or replication step.
//!
//! ## Why this test layer
//!
//! The brief's headline example is "Worker B's session boots and
//! the prompt includes skill X". That boot wiring lands in
//! `mission-skill-session-boot-wire` (M6 precursor 3/3, still
//! `TODO` on `origin/main` at the time of this brief). The
//! propagation guarantee itself, however, sits at the SQL +
//! trust-reader layer that `available_for_worker` already
//! implements. This test exercises that layer end-to-end across
//! the real `argo-skill`, `argo-identity`, and `argo-store` crates
//! — once the boot-wire brief lands, a follow-up can assert the
//! prompt-format on top of the same fixture.
//!
//! ## Setup
//!
//! - Tenant T, three workers: A (trust_grade 4), B (trust_grade 2),
//!   C (trust_grade 0).
//! - Skill X with `min_trust_grade = TrustGrade::Tenant` (= 2 on
//!   the same numeric ladder as `worker.trust_grade`).
//! - Assertions:
//!   - A sees X (4 >= 2).
//!   - B sees X (2 >= 2).
//!   - C does not see X (0 < 2).
//!
//! The brief's illustrative grades (A=80, B=60, C=20, X=50) are
//! beyond the `TrustGrade` enum's 0..4 ladder. The public setter
//! `SkillStore::set_min_trust_grade` only accepts a `TrustGrade`,
//! so the test uses the enum's discriminants — same propagation
//! relation, in-range values.

use argo_identity::{IdentityStore, Persona, Role, Worker, WorkerId};
use argo_skill::trust::TrustGrade;
use argo_skill::{SkillStore, WorkerTrustReader};
use argo_store::SessionStore;
use argo_types::{ArgoResult, TenantId};
use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

/// `argo-skill` does not depend on `argo-identity`, so the
/// production code path injects a `WorkerTrustReader` adapter (lives
/// in `argo-cli/src/handlers/worker_trust_reader.rs`). For this
/// integration test we re-implement the same adapter locally — both
/// crates are in scope at the test level, and inlining keeps the
/// fixture self-contained.
struct E2eTrustReader<'a, 's> {
    identity: &'a IdentityStore<'s>,
}

#[async_trait]
impl WorkerTrustReader for E2eTrustReader<'_, '_> {
    async fn trust_grade(
        &self,
        tenant_id: TenantId,
        worker_id: Uuid,
    ) -> ArgoResult<Option<u8>> {
        let worker = self
            .identity
            .get_worker(tenant_id, WorkerId(worker_id))
            .await?;
        Ok(worker.map(|w| w.trust_grade))
    }
}

const SKILL_X_MD: &str = "---
name: propagated-x
version: \"0.1.0\"
description: \"propagation test fixture\"
provenance:
  source: builtin
  hash: sha256-x
---
body for skill X";

async fn fresh_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    Arc::new(store)
}

async fn seed_worker(
    identity: &IdentityStore<'_>,
    tenant: TenantId,
    name: &str,
    trust_grade: u8,
) -> Worker {
    let worker = Worker::draft(
        tenant,
        "gtm",
        Persona::named(name, "test-built"),
        Role::new("Propagation fixture."),
    );
    identity.create_worker(&worker).await.unwrap();
    identity
        .set_worker_trust_grade(tenant, worker.id, trust_grade)
        .await
        .unwrap();
    worker
}

#[tokio::test]
async fn skill_propagates_to_workers_clearing_min_trust_grade() {
    let store = fresh_store().await;
    let tenant = TenantId::new();
    let identity = IdentityStore::new(&store);

    // Setup: three workers at trust grades 4 / 2 / 0.
    let worker_a = seed_worker(&identity, tenant, "Worker A", 4).await;
    let worker_b = seed_worker(&identity, tenant, "Worker B", 2).await;
    let worker_c = seed_worker(&identity, tenant, "Worker C", 0).await;

    // Worker A installs skill X with a `Tenant` (=2) trust floor.
    // In production this is the `SkillStore::install` call backing
    // `argo skill install`; the integration test calls the API
    // directly — same store, same row shape.
    let skills = SkillStore::new(Arc::clone(&store));
    skills.install(SKILL_X_MD, None).await.unwrap();
    skills
        .set_min_trust_grade("propagated-x", Some(TrustGrade::Tenant))
        .await
        .unwrap();

    let reader = E2eTrustReader { identity: &identity };

    // Worker A (grade 4) clears 2 → sees skill X.
    let a_skills = skills
        .available_for_worker(&reader, tenant, worker_a.id.0)
        .await
        .unwrap();
    assert!(
        a_skills.iter().any(|s| s.frontmatter.name == "propagated-x"),
        "Worker A (grade 4) should see propagated-x; got {:?}",
        a_skills
            .iter()
            .map(|s| &s.frontmatter.name)
            .collect::<Vec<_>>(),
    );

    // Worker B (grade 2) clears 2 (the equal case) → sees skill X.
    let b_skills = skills
        .available_for_worker(&reader, tenant, worker_b.id.0)
        .await
        .unwrap();
    assert!(
        b_skills.iter().any(|s| s.frontmatter.name == "propagated-x"),
        "Worker B (grade 2) should see propagated-x; got {:?}",
        b_skills
            .iter()
            .map(|s| &s.frontmatter.name)
            .collect::<Vec<_>>(),
    );

    // Worker C (grade 0) does not clear 2 → must NOT see skill X.
    let c_skills = skills
        .available_for_worker(&reader, tenant, worker_c.id.0)
        .await
        .unwrap();
    assert!(
        c_skills
            .iter()
            .all(|s| s.frontmatter.name != "propagated-x"),
        "Worker C (grade 0) must NOT see propagated-x; got {:?}",
        c_skills
            .iter()
            .map(|s| &s.frontmatter.name)
            .collect::<Vec<_>>(),
    );
}

#[tokio::test]
async fn unrestricted_skill_propagates_to_every_worker() {
    // Companion case to the above: a skill with NULL min_trust_grade
    // (the v1 default) is visible to every worker regardless of
    // grade, including the legacy grade=0 row. Locks the
    // "no-floor means no-restriction" half of the propagation
    // contract so a future SQL drift on either column flips this
    // test, not silently weaker rows.
    let store = fresh_store().await;
    let tenant = TenantId::new();
    let identity = IdentityStore::new(&store);

    let worker_high = seed_worker(&identity, tenant, "High", 4).await;
    let worker_low = seed_worker(&identity, tenant, "Low", 0).await;

    let skills = SkillStore::new(Arc::clone(&store));
    skills.install(SKILL_X_MD, None).await.unwrap();
    // Note: no `set_min_trust_grade` call — floor stays NULL.

    let reader = E2eTrustReader { identity: &identity };

    for (label, id) in [("high", worker_high.id), ("low", worker_low.id)] {
        let names: Vec<String> = skills
            .available_for_worker(&reader, tenant, id.0)
            .await
            .unwrap()
            .into_iter()
            .map(|s| s.frontmatter.name)
            .collect();
        assert!(
            names.iter().any(|n| n == "propagated-x"),
            "{label} worker should see unrestricted propagated-x; got {names:?}",
        );
    }
}
