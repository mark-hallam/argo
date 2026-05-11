//! Unit tests for the `route_async` + [`TargetResolver`] resolver
//! path (Stream `mission-routing-skillmatch-rolematch`, Tier M4).
//! Split out of `routing_tests.rs` so each file stays under the
//! 500-LOC ceiling. Each test constructs a stub `TargetResolver` so
//! we don't drag the `IdentityStore` integration into unit tests;
//! the end-to-end smoke (real `IdentityStore` + skill seed) lives in
//! `crates/argo-cli/tests/handoff_routing_e2e.rs`.
#![cfg(test)]

use crate::routing::{
    HandoffRoutingRule, RoutingContext, RoutingEngine, TargetSelector, WhenClause,
};
use crate::routing_resolve::{rank_workers, TargetResolver};
use crate::EscalationStep;
use argo_identity::WorkerId;
use argo_types::{ArgoError, ArgoResult, TenantId};
use chrono::{DateTime, TimeZone, Utc};

struct StubResolver {
    skill_pick: Option<WorkerId>,
    role_pick: Option<WorkerId>,
    fail: bool,
}

impl StubResolver {
    fn empty() -> Self {
        Self { skill_pick: None, role_pick: None, fail: false }
    }
    fn with_skill(pick: WorkerId) -> Self {
        Self { skill_pick: Some(pick), role_pick: None, fail: false }
    }
    fn with_role(pick: WorkerId) -> Self {
        Self { skill_pick: None, role_pick: Some(pick), fail: false }
    }
    fn always_errors() -> Self {
        Self { skill_pick: None, role_pick: None, fail: true }
    }
}

#[async_trait::async_trait]
impl TargetResolver for StubResolver {
    async fn resolve_skill_match(
        &self,
        _tenant: TenantId,
        _skill_name: &str,
    ) -> ArgoResult<Option<WorkerId>> {
        if self.fail {
            return Err(ArgoError::Provider("stub fault".into()));
        }
        Ok(self.skill_pick)
    }
    async fn resolve_role_match(
        &self,
        _tenant: TenantId,
        _role_name: &str,
    ) -> ArgoResult<Option<WorkerId>> {
        if self.fail {
            return Err(ArgoError::Provider("stub fault".into()));
        }
        Ok(self.role_pick)
    }
}

fn at_hour(hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 5, 7, hour, 0, 0).unwrap()
}

fn ctx_with_tenant(now: DateTime<Utc>, outcome: &str) -> (RoutingContext, TenantId) {
    let tenant = TenantId::new();
    let c = RoutingContext::new(now, outcome).with_tenant(tenant);
    (c, tenant)
}

#[tokio::test]
async fn skill_match_returns_resolver_pick() {
    // Stub returns the higher-grade worker — engine surfaces it as
    // an EscalationStep::Worker. Stand-in for the "highest grade
    // wins" rank contract; the actual ranking is exercised by
    // `rank_workers_orders_by_grade_then_recency_then_id` below and
    // by the e2e integration test that drives the real resolver.
    let pick = WorkerId::new();
    let rule = HandoffRoutingRule::new(
        "skill",
        WhenClause::default(),
        TargetSelector::SkillMatch("marketing-editor".into()),
        1,
    );
    let engine = RoutingEngine::new(vec![rule]);
    let (ctx, _tenant) = ctx_with_tenant(at_hour(12), "x");
    let resolver = StubResolver::with_skill(pick);
    let step = engine.route_async(&ctx, &resolver).await.unwrap();
    assert_eq!(step, EscalationStep::Worker(pick));
}

#[tokio::test]
async fn role_match_returns_resolver_pick() {
    let pick = WorkerId::new();
    let rule = HandoffRoutingRule::new(
        "role",
        WhenClause::default(),
        TargetSelector::RoleMatch("on-call-sre".into()),
        1,
    );
    let engine = RoutingEngine::new(vec![rule]);
    let (ctx, _tenant) = ctx_with_tenant(at_hour(12), "x");
    let resolver = StubResolver::with_role(pick);
    let step = engine.route_async(&ctx, &resolver).await.unwrap();
    assert_eq!(step, EscalationStep::Worker(pick));
}

#[tokio::test]
async fn skill_and_role_match_both_empty_returns_none() {
    let rule_skill = HandoffRoutingRule::new(
        "skill",
        WhenClause::default(),
        TargetSelector::SkillMatch("missing".into()),
        1,
    );
    let rule_role = HandoffRoutingRule::new(
        "role",
        WhenClause::default(),
        TargetSelector::RoleMatch("missing".into()),
        2,
    );
    let engine = RoutingEngine::new(vec![rule_skill, rule_role]);
    let (ctx, _tenant) = ctx_with_tenant(at_hour(12), "x");
    let resolver = StubResolver::empty();
    assert!(engine.route_async(&ctx, &resolver).await.is_none());
}

#[test]
fn rank_workers_orders_by_grade_then_recency_then_id() {
    // Higher grade always wins outright.
    let low = WorkerId::new();
    let high = WorkerId::new();
    let now = Utc.with_ymd_and_hms(2026, 5, 12, 0, 0, 0).unwrap();
    let later = now + chrono::Duration::hours(1);
    let picked = rank_workers(&[(low, 1, Some(now)), (high, 5, Some(now))]);
    assert_eq!(picked, Some(high));

    // Tied grade → newer last_used_at wins.
    let stale = WorkerId::new();
    let fresh = WorkerId::new();
    let picked = rank_workers(&[(stale, 3, Some(now)), (fresh, 3, Some(later))]);
    assert_eq!(picked, Some(fresh));

    // Tied grade + tied recency → deterministic by `worker_id ASC`
    // (lower UUID wins, regardless of input order).
    let a = WorkerId(uuid::Uuid::parse_str("00000000-0000-7000-8000-000000000001").unwrap());
    let b = WorkerId(uuid::Uuid::parse_str("00000000-0000-7000-8000-000000000002").unwrap());
    assert_eq!(rank_workers(&[(a, 3, Some(now)), (b, 3, Some(now))]), Some(a));
    assert_eq!(rank_workers(&[(b, 3, Some(now)), (a, 3, Some(now))]), Some(a));
}

#[tokio::test]
async fn resolver_error_falls_through_to_none() {
    // A resolver that errors must not crash route_async — it should
    // return None so the walker falls back to the explicit chain.
    let rule = HandoffRoutingRule::new(
        "skill",
        WhenClause::default(),
        TargetSelector::SkillMatch("never-resolves".into()),
        1,
    );
    let engine = RoutingEngine::new(vec![rule]);
    let (ctx, _tenant) = ctx_with_tenant(at_hour(12), "x");
    let resolver = StubResolver::always_errors();
    assert!(engine.route_async(&ctx, &resolver).await.is_none());
}

#[tokio::test]
async fn route_async_skips_resolver_when_tenant_unset() {
    // Back-compat: a RoutingContext without `tenant_id` must keep
    // the v1 behaviour where SkillMatch/RoleMatch return None.
    let rule = HandoffRoutingRule::new(
        "skill",
        WhenClause::default(),
        TargetSelector::SkillMatch("anything".into()),
        1,
    );
    let engine = RoutingEngine::new(vec![rule]);
    let ctx = RoutingContext::new(at_hour(12), "x");
    let resolver = StubResolver::with_skill(WorkerId::new());
    assert!(engine.route_async(&ctx, &resolver).await.is_none());
}
