//! Unit tests for `crate::routing`.

use super::*;
use crate::EscalationStep;
use argo_identity::WorkerId;
use argo_store::SessionStore;
use chrono::TimeZone;

fn ctx(now: DateTime<Utc>, outcome: &str) -> RoutingContext {
    RoutingContext::new(now, outcome)
}

fn at_hour(hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 5, 7, hour, 0, 0).unwrap()
}

#[test]
fn empty_when_clause_always_matches() {
    let when = WhenClause::default();
    assert!(when.matches(&ctx(at_hour(12), "anything")));
}

#[test]
fn expected_outcome_substring_match_is_case_insensitive() {
    let when = WhenClause {
        expected_outcome_contains: Some("Copy Review".into()),
        ..Default::default()
    };
    assert!(when.matches(&ctx(at_hour(12), "please do copy review for this")));
    assert!(!when.matches(&ctx(at_hour(12), "ship the build")));
}

#[test]
fn hour_of_day_constraint_matches_in_window() {
    let when = WhenClause {
        hour_of_day_utc_in: vec![22, 23, 0, 1],
        ..Default::default()
    };
    assert!(when.matches(&ctx(at_hour(23), "x")));
    assert!(!when.matches(&ctx(at_hour(12), "x")));
}

#[test]
fn weekday_constraint_filters() {
    // 2026-05-07 is a Thursday → weekday 3 (Mon=0).
    let when = WhenClause {
        weekday_utc_in: vec![5, 6], // Sat/Sun only
        ..Default::default()
    };
    assert!(!when.matches(&ctx(at_hour(12), "x")));
}

#[test]
fn conjunctive_match_fails_when_any_field_misses() {
    let when = WhenClause {
        expected_outcome_contains: Some("review".into()),
        hour_of_day_utc_in: vec![23],
        ..Default::default()
    };
    let outcome_ok_hour_wrong = ctx(at_hour(12), "review please");
    assert!(!when.matches(&outcome_ok_hour_wrong));
    let hour_ok_outcome_wrong = ctx(at_hour(23), "deploy");
    assert!(!when.matches(&hour_ok_outcome_wrong));
    let both_match = ctx(at_hour(23), "review please");
    assert!(when.matches(&both_match));
}

#[test]
fn route_returns_none_when_no_rules_match() {
    let engine = RoutingEngine::new(vec![]);
    assert!(engine.route(&ctx(at_hour(12), "x")).is_none());
}

#[test]
fn route_picks_highest_priority_match() {
    let w_low = WorkerId::new();
    let w_high = WorkerId::new();
    let low = HandoffRoutingRule {
        id: RuleId::new(),
        name: "fallback".into(),
        when: WhenClause::default(),
        target: TargetSelector::SpecificWorker(w_low),
        priority: 100,
        active: true,
    };
    let high = HandoffRoutingRule {
        id: RuleId::new(),
        name: "primary".into(),
        when: WhenClause::default(),
        target: TargetSelector::SpecificWorker(w_high),
        priority: 1,
        active: true,
    };
    let engine = RoutingEngine::new(vec![low, high]);
    let step = engine.route(&ctx(at_hour(12), "x")).unwrap();
    assert_eq!(step, EscalationStep::Worker(w_high));
}

#[test]
fn route_skips_inactive_rules() {
    let w = WorkerId::new();
    let inactive = HandoffRoutingRule {
        id: RuleId::new(),
        name: "off".into(),
        when: WhenClause::default(),
        target: TargetSelector::SpecificWorker(w),
        priority: 1,
        active: false,
    };
    let engine = RoutingEngine::new(vec![inactive]);
    assert!(engine.route(&ctx(at_hour(12), "x")).is_none());
}

#[test]
fn round_robin_cycles_through_workers() {
    let a = WorkerId::new();
    let b = WorkerId::new();
    let c = WorkerId::new();
    let rule = HandoffRoutingRule::new(
        "rr",
        WhenClause::default(),
        TargetSelector::RoundRobin(vec![a, b, c]),
        1,
    );
    let engine = RoutingEngine::new(vec![rule]);
    let step1 = engine.route(&ctx(at_hour(12), "x")).unwrap();
    let step2 = engine.route(&ctx(at_hour(12), "x")).unwrap();
    let step3 = engine.route(&ctx(at_hour(12), "x")).unwrap();
    let step4 = engine.route(&ctx(at_hour(12), "x")).unwrap();
    assert_eq!(step1, EscalationStep::Worker(a));
    assert_eq!(step2, EscalationStep::Worker(b));
    assert_eq!(step3, EscalationStep::Worker(c));
    assert_eq!(step4, EscalationStep::Worker(a));
}

#[test]
fn round_robin_with_empty_list_returns_none() {
    let rule = HandoffRoutingRule::new(
        "rr",
        WhenClause::default(),
        TargetSelector::RoundRobin(Vec::new()),
        1,
    );
    let engine = RoutingEngine::new(vec![rule]);
    assert!(engine.route(&ctx(at_hour(12), "x")).is_none());
}

#[test]
fn skill_match_is_unresolved_in_v1() {
    let rule = HandoffRoutingRule::new(
        "skill",
        WhenClause::default(),
        TargetSelector::SkillMatch("marketing-editor".into()),
        1,
    );
    let engine = RoutingEngine::new(vec![rule]);
    assert!(engine.route(&ctx(at_hour(12), "x")).is_none());
}

#[test]
fn role_match_is_unresolved_in_v1() {
    let rule = HandoffRoutingRule::new(
        "role",
        WhenClause::default(),
        TargetSelector::RoleMatch("on-call-sre".into()),
        1,
    );
    let engine = RoutingEngine::new(vec![rule]);
    assert!(engine.route(&ctx(at_hour(12), "x")).is_none());
}

#[test]
fn dsl_json_round_trip() {
    let w = WorkerId::new();
    let rule = HandoffRoutingRule {
        id: RuleId::new(),
        name: "copy-review".into(),
        when: WhenClause {
            expected_outcome_contains: Some("copy review".into()),
            hour_of_day_utc_in: vec![9, 10, 11],
            weekday_utc_in: Vec::new(),
        },
        target: TargetSelector::SpecificWorker(w),
        priority: 5,
        active: true,
    };
    let json = serde_json::to_string(&rule).unwrap();
    let parsed: HandoffRoutingRule = serde_json::from_str(&json).unwrap();
    assert_eq!(rule, parsed);
}

#[test]
fn rule_id_uniqueness() {
    let a = RuleId::new();
    let b = RuleId::new();
    assert_ne!(a, b);
}

async fn open_store() -> RoutingRuleStore {
    let inner = std::sync::Arc::new(SessionStore::open(":memory:").await.unwrap());
    inner.migrate().await.unwrap();
    let s = RoutingRuleStore::new(inner);
    s.ensure_schema().await.unwrap();
    s
}

#[tokio::test]
async fn store_create_then_get_round_trip() {
    let store = open_store().await;
    let tenant = argo_types::TenantId::new();
    let w = WorkerId::new();
    let rule = HandoffRoutingRule::new(
        "primary",
        WhenClause {
            expected_outcome_contains: Some("review".into()),
            ..Default::default()
        },
        TargetSelector::SpecificWorker(w),
        10,
    );
    store.create(tenant, &rule).await.unwrap();
    let loaded = store.get(tenant, rule.id).await.unwrap().unwrap();
    assert_eq!(loaded, rule);
}

#[tokio::test]
async fn store_list_orders_by_priority() {
    let store = open_store().await;
    let tenant = argo_types::TenantId::new();
    let w = WorkerId::new();
    for (name, prio) in [("low", 100), ("high", 1), ("mid", 50)] {
        store
            .create(
                tenant,
                &HandoffRoutingRule::new(
                    name,
                    WhenClause::default(),
                    TargetSelector::SpecificWorker(w),
                    prio,
                ),
            )
            .await
            .unwrap();
    }
    let rows = store.list(tenant).await.unwrap();
    let names: Vec<&str> = rows.iter().map(|r| r.name.as_str()).collect();
    assert_eq!(names, vec!["high", "mid", "low"]);
}

#[tokio::test]
async fn store_delete_removes_rule() {
    let store = open_store().await;
    let tenant = argo_types::TenantId::new();
    let w = WorkerId::new();
    let rule = HandoffRoutingRule::new(
        "doomed",
        WhenClause::default(),
        TargetSelector::SpecificWorker(w),
        10,
    );
    store.create(tenant, &rule).await.unwrap();
    store.delete(tenant, rule.id).await.unwrap();
    assert!(store.get(tenant, rule.id).await.unwrap().is_none());
}

#[tokio::test]
async fn store_engine_for_tenant_skips_inactive() {
    let store = open_store().await;
    let tenant = argo_types::TenantId::new();
    let w_active = WorkerId::new();
    let w_inactive = WorkerId::new();
    store
        .create(
            tenant,
            &HandoffRoutingRule::new(
                "off",
                WhenClause::default(),
                TargetSelector::SpecificWorker(w_inactive),
                1,
            ),
        )
        .await
        .unwrap();
    let mut active = HandoffRoutingRule::new(
        "on",
        WhenClause::default(),
        TargetSelector::SpecificWorker(w_active),
        10,
    );
    active.active = true;
    store.create(tenant, &active).await.unwrap();
    // Manually flip the first row inactive by deleting + re-creating.
    let listed = store.list(tenant).await.unwrap();
    let mut inactive_rule = listed.into_iter().find(|r| r.name == "off").unwrap();
    inactive_rule.active = false;
    store.delete(tenant, inactive_rule.id).await.unwrap();
    store.create(tenant, &inactive_rule).await.unwrap();
    let engine = store.engine_for_tenant(tenant).await.unwrap();
    assert_eq!(engine.rules().len(), 1);
    assert_eq!(engine.rules()[0].name, "on");
}

#[tokio::test]
async fn store_isolated_per_tenant() {
    let store = open_store().await;
    let tenant_a = argo_types::TenantId::new();
    let tenant_b = argo_types::TenantId::new();
    let w = WorkerId::new();
    store
        .create(
            tenant_a,
            &HandoffRoutingRule::new(
                "shared-name",
                WhenClause::default(),
                TargetSelector::SpecificWorker(w),
                1,
            ),
        )
        .await
        .unwrap();
    store
        .create(
            tenant_b,
            &HandoffRoutingRule::new(
                "shared-name",
                WhenClause::default(),
                TargetSelector::SpecificWorker(w),
                1,
            ),
        )
        .await
        .unwrap();
    assert_eq!(store.list(tenant_a).await.unwrap().len(), 1);
    assert_eq!(store.list(tenant_b).await.unwrap().len(), 1);
}
