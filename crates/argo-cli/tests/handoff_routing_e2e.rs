//! Integration smoke for Tier M4 (Stream
//! `mission-routing-skillmatch-rolematch`). Seeds two workers with
//! the same skill at different trust grades, drives
//! `RoutingEngine::route_async` with an [`IdentityTargetResolver`]
//! shape, and asserts the higher-grade worker wins.
//!
//! `argo-cli` is binary-only (no `[lib]`), so we re-declare the
//! resolver here as a test-local struct over the public APIs the
//! production handler also consumes (`IdentityStore`, `SkillStore`,
//! `rank_workers`). Behaviour parity is enforced by sharing the
//! same `rank_workers` helper.

use argo_identity::{IdentityStore, Persona, Role, Worker, WorkerId};
use argo_orchestrator::{
    rank_workers, EscalationStep, HandoffRoutingRule, RoutingContext, RoutingEngine,
    TargetResolver, TargetSelector, WhenClause,
};
use argo_skill::SkillStore;
use argo_store::SessionStore;
use argo_types::{ArgoResult, TenantId};
use chrono::{TimeZone, Utc};
use std::sync::Arc;

const SKILL_MD: &str = "---
name: marketing-editor
version: \"0.1.0\"
description: \"Hand-off copy review\"
provenance:
  source: builtin
  hash: sha256-x
---
Review marketing copy carefully.";

struct E2eResolver {
    store: Arc<SessionStore>,
    skill_store: Arc<SkillStore>,
}

#[async_trait::async_trait]
impl TargetResolver for E2eResolver {
    async fn resolve_skill_match(
        &self,
        tenant: TenantId,
        skill_name: &str,
    ) -> ArgoResult<Option<WorkerId>> {
        if self
            .skill_store
            .find_active_by_name(skill_name)
            .await?
            .is_none()
        {
            return Ok(None);
        }
        let identities = IdentityStore::new(&self.store);
        let workers = identities.workers_with_skill(tenant, skill_name).await?;
        Ok(rank_workers(&tuples(&workers)))
    }

    async fn resolve_role_match(
        &self,
        tenant: TenantId,
        role_name: &str,
    ) -> ArgoResult<Option<WorkerId>> {
        let identities = IdentityStore::new(&self.store);
        let workers = identities.workers_with_role(tenant, role_name).await?;
        Ok(rank_workers(&tuples(&workers)))
    }
}

fn tuples(
    workers: &[Worker],
) -> Vec<(WorkerId, u8, Option<chrono::DateTime<chrono::Utc>>)> {
    workers
        .iter()
        .map(|w| (w.id, w.trust_grade, w.last_used_at))
        .collect()
}

async fn fresh_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    Arc::new(store)
}

async fn seed_worker(store: &SessionStore, tenant: TenantId, name: &str) -> Worker {
    let worker = Worker::draft(
        tenant,
        "gtm-engineer",
        Persona::named(name, "test-built"),
        Role::new("Marketing copy review."),
    );
    IdentityStore::new(store)
        .create_worker(&worker)
        .await
        .unwrap();
    worker
}

#[tokio::test]
async fn skill_match_handoff_picks_higher_trust_grade_worker() {
    let store = fresh_store().await;
    let tenant = TenantId::new();
    let identities = IdentityStore::new(&store);

    let low = seed_worker(&store, tenant, "Junior").await;
    let high = seed_worker(&store, tenant, "Senior").await;

    // Bump senior to trust_grade 5; junior stays at 0.
    identities
        .update_worker(
            tenant,
            high.id,
            argo_identity::WorkerUpdate::default().with_trust_grade(5),
        )
        .await
        .unwrap();

    // Both workers know the skill; senior should still win because
    // of the grade gap.
    identities
        .add_worker_skill(low.id, "marketing-editor", 3)
        .await
        .unwrap();
    identities
        .add_worker_skill(high.id, "marketing-editor", 3)
        .await
        .unwrap();

    let skill_store = Arc::new(SkillStore::new(store.clone()));
    skill_store.install(SKILL_MD, None).await.unwrap();

    let resolver = E2eResolver {
        store: store.clone(),
        skill_store,
    };

    let rule = HandoffRoutingRule::new(
        "marketing-review",
        WhenClause::default(),
        TargetSelector::SkillMatch("marketing-editor".into()),
        1,
    );
    let engine = RoutingEngine::new(vec![rule]);
    let ctx = RoutingContext::new(
        Utc.with_ymd_and_hms(2026, 5, 12, 12, 0, 0).unwrap(),
        "review the launch copy",
    )
    .with_tenant(tenant);

    let step = engine.route_async(&ctx, &resolver).await.unwrap();
    assert_eq!(step, EscalationStep::Worker(high.id));
}
