//! Unit tests for `crate::roster`. Loaded via
//! `#[cfg(test)] #[path = ...] mod tests;` from `roster.rs` so
//! both files stay under the 500-LOC ceiling.

use super::*;
use crate::types::{NotifyChannelRef, WorkerId};
use argo_store::SessionStore;
use std::sync::Arc;

async fn open_roster_store() -> RosterStore {
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    let roster = RosterStore::new(store);
    roster.ensure_schema().await.unwrap();
    roster
}

fn enabled_policy(daily_micros: u64) -> AutoApprovalPolicy {
    AutoApprovalPolicy {
        enabled: true,
        daily_budget_usd_micros: Some(daily_micros),
        ..Default::default()
    }
}

#[test]
fn most_restrictive_empty_returns_default() {
    let merged = AutoApprovalPolicy::most_restrictive(&[]);
    assert!(!merged.enabled);
    assert!(merged.daily_budget_usd_micros.is_none());
}

#[test]
fn most_restrictive_picks_min_daily_budget() {
    let big = enabled_policy(50_000);
    let small = enabled_policy(10_000);
    let merged = AutoApprovalPolicy::most_restrictive(&[&big, &small]);
    assert!(merged.enabled);
    assert_eq!(merged.daily_budget_usd_micros, Some(10_000));
}

#[test]
fn most_restrictive_picks_min_per_action() {
    let a = AutoApprovalPolicy {
        enabled: true,
        max_usd_micros_per_action: Some(2_000),
        ..Default::default()
    };
    let b = AutoApprovalPolicy {
        enabled: true,
        max_usd_micros_per_action: Some(8_000),
        ..Default::default()
    };
    let merged = AutoApprovalPolicy::most_restrictive(&[&a, &b]);
    assert_eq!(merged.max_usd_micros_per_action, Some(2_000));
}

#[test]
fn most_restrictive_intersects_wake_action_types() {
    let a = AutoApprovalPolicy {
        enabled: true,
        wake_action_types: vec!["page_oncall".into(), "escalate".into()],
        ..Default::default()
    };
    let b = AutoApprovalPolicy {
        enabled: true,
        wake_action_types: vec!["escalate".into(), "ack".into()],
        ..Default::default()
    };
    let merged = AutoApprovalPolicy::most_restrictive(&[&a, &b]);
    assert_eq!(merged.wake_action_types, vec!["escalate".to_string()]);
}

#[test]
fn most_restrictive_picks_shortest_time_windows() {
    use crate::auto_approval::TimeWindow;
    use chrono::NaiveTime;
    let many = AutoApprovalPolicy {
        enabled: true,
        time_windows: vec![
            TimeWindow {
                days: Vec::new(),
                start: NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
                end: NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
            },
            TimeWindow {
                days: Vec::new(),
                start: NaiveTime::from_hms_opt(13, 0, 0).unwrap(),
                end: NaiveTime::from_hms_opt(17, 0, 0).unwrap(),
            },
        ],
        ..Default::default()
    };
    let few = AutoApprovalPolicy {
        enabled: true,
        time_windows: vec![TimeWindow {
            days: Vec::new(),
            start: NaiveTime::from_hms_opt(10, 0, 0).unwrap(),
            end: NaiveTime::from_hms_opt(11, 0, 0).unwrap(),
        }],
        ..Default::default()
    };
    let merged = AutoApprovalPolicy::most_restrictive(&[&many, &few]);
    assert_eq!(merged.time_windows.len(), 1);
}

#[test]
fn roster_id_uniqueness() {
    let a = RosterId::new();
    let b = RosterId::new();
    assert_ne!(a, b);
}

#[test]
fn roster_new_starts_empty() {
    let r = Roster::new(
        TenantId::new(),
        "ops",
        AutoApprovalPolicy::default(),
        Vec::new(),
        None,
    );
    assert!(r.member_worker_ids.is_empty());
    assert_eq!(r.name, "ops");
}

#[tokio::test]
async fn create_then_get_round_trip() {
    let store = open_roster_store().await;
    let tenant = TenantId::new();
    let roster = Roster::new(
        tenant,
        "alpha",
        enabled_policy(20_000),
        vec![NotifyChannelRef::new("email", "ops@example.com")],
        Some(123.45),
    );
    let id = roster.id;
    store.create(&roster).await.unwrap();
    let loaded = store.get(tenant, id).await.unwrap().unwrap();
    assert_eq!(loaded.name, "alpha");
    assert!(loaded.policy.enabled);
    assert_eq!(loaded.channels.len(), 1);
    let cap = loaded.budget_cap_usd.unwrap();
    assert!((cap - 123.45).abs() < 1e-6);
}

#[tokio::test]
async fn get_by_name_finds_roster() {
    let store = open_roster_store().await;
    let tenant = TenantId::new();
    let roster = Roster::new(
        tenant,
        "beta",
        AutoApprovalPolicy::default(),
        Vec::new(),
        None,
    );
    store.create(&roster).await.unwrap();
    let by_name = store.get_by_name(tenant, "beta").await.unwrap().unwrap();
    assert_eq!(by_name.id, roster.id);
}

#[tokio::test]
async fn list_returns_rosters_ordered_by_name() {
    let store = open_roster_store().await;
    let tenant = TenantId::new();
    for name in ["zulu", "alpha", "mike"] {
        store
            .create(&Roster::new(
                tenant,
                name,
                AutoApprovalPolicy::default(),
                Vec::new(),
                None,
            ))
            .await
            .unwrap();
    }
    let rows = store.list(tenant).await.unwrap();
    let names: Vec<&str> = rows.iter().map(|r| r.name.as_str()).collect();
    assert_eq!(names, vec!["alpha", "mike", "zulu"]);
}

#[tokio::test]
async fn add_member_then_list_members() {
    let store = open_roster_store().await;
    let tenant = TenantId::new();
    let roster = Roster::new(
        tenant,
        "alpha",
        AutoApprovalPolicy::default(),
        Vec::new(),
        None,
    );
    store.create(&roster).await.unwrap();
    let w1 = WorkerId::new();
    let w2 = WorkerId::new();
    store.add_member(roster.id, w1).await.unwrap();
    store.add_member(roster.id, w2).await.unwrap();
    let members = store.list_members(roster.id).await.unwrap();
    assert_eq!(members.len(), 2);
    assert!(members.contains(&w1));
    assert!(members.contains(&w2));
}

#[tokio::test]
async fn remove_member_drops_worker() {
    let store = open_roster_store().await;
    let tenant = TenantId::new();
    let roster = Roster::new(
        tenant,
        "alpha",
        AutoApprovalPolicy::default(),
        Vec::new(),
        None,
    );
    store.create(&roster).await.unwrap();
    let w = WorkerId::new();
    store.add_member(roster.id, w).await.unwrap();
    store.remove_member(roster.id, w).await.unwrap();
    let members = store.list_members(roster.id).await.unwrap();
    assert!(members.is_empty());
}

#[tokio::test]
async fn rosters_for_worker_finds_membership() {
    let store = open_roster_store().await;
    let tenant = TenantId::new();
    let r1 = Roster::new(
        tenant,
        "team-a",
        AutoApprovalPolicy::default(),
        Vec::new(),
        None,
    );
    let r2 = Roster::new(
        tenant,
        "team-b",
        AutoApprovalPolicy::default(),
        Vec::new(),
        None,
    );
    store.create(&r1).await.unwrap();
    store.create(&r2).await.unwrap();
    let w = WorkerId::new();
    store.add_member(r1.id, w).await.unwrap();
    store.add_member(r2.id, w).await.unwrap();
    let rosters = store.rosters_for_worker(tenant, w).await.unwrap();
    assert_eq!(rosters.len(), 2);
}

#[tokio::test]
async fn rosters_for_worker_isolated_per_tenant() {
    let store = open_roster_store().await;
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    let w = WorkerId::new();
    let ra = Roster::new(
        tenant_a,
        "team",
        AutoApprovalPolicy::default(),
        Vec::new(),
        None,
    );
    let rb = Roster::new(
        tenant_b,
        "team",
        AutoApprovalPolicy::default(),
        Vec::new(),
        None,
    );
    store.create(&ra).await.unwrap();
    store.create(&rb).await.unwrap();
    store.add_member(ra.id, w).await.unwrap();
    store.add_member(rb.id, w).await.unwrap();
    let only_a = store.rosters_for_worker(tenant_a, w).await.unwrap();
    assert_eq!(only_a.len(), 1);
    assert_eq!(only_a[0].tenant_id, tenant_a);
}

#[tokio::test]
async fn delete_cascades_members() {
    let store = open_roster_store().await;
    let tenant = TenantId::new();
    let roster = Roster::new(
        tenant,
        "doomed",
        AutoApprovalPolicy::default(),
        Vec::new(),
        None,
    );
    store.create(&roster).await.unwrap();
    let w = WorkerId::new();
    store.add_member(roster.id, w).await.unwrap();
    store.delete(tenant, roster.id).await.unwrap();
    let members = store.list_members(roster.id).await.unwrap();
    assert!(members.is_empty());
    assert!(store.get(tenant, roster.id).await.unwrap().is_none());
}

#[test]
fn worker_resolved_policy_with_rosters_prefers_worker_when_enabled() {
    use crate::{Persona, Role, Worker};
    let mut worker = Worker::draft(
        TenantId::new(),
        "ops",
        Persona::named("Maya", "op"),
        Role::new("triage"),
    );
    worker.auto_approval = enabled_policy(80_000);
    let roster_policy = enabled_policy(10_000);
    let workspace = AutoApprovalPolicy::default();
    let resolved = worker.resolved_policy_with_rosters(&[roster_policy], &workspace);
    assert_eq!(resolved.daily_budget_usd_micros, Some(80_000));
}

#[test]
fn worker_resolved_policy_falls_through_to_roster_when_worker_disabled() {
    use crate::{Persona, Role, Worker};
    let worker = Worker::draft(
        TenantId::new(),
        "ops",
        Persona::named("Maya", "op"),
        Role::new("triage"),
    );
    let roster_policy = enabled_policy(10_000);
    let workspace = enabled_policy(99_999);
    let resolved = worker.resolved_policy_with_rosters(&[roster_policy], &workspace);
    assert_eq!(resolved.daily_budget_usd_micros, Some(10_000));
}

#[test]
fn worker_resolved_policy_falls_through_to_workspace_when_no_enabled_roster() {
    use crate::{Persona, Role, Worker};
    let worker = Worker::draft(
        TenantId::new(),
        "ops",
        Persona::named("Maya", "op"),
        Role::new("triage"),
    );
    let disabled = AutoApprovalPolicy::default();
    let workspace = enabled_policy(99_999);
    let resolved = worker.resolved_policy_with_rosters(&[disabled], &workspace);
    assert_eq!(resolved.daily_budget_usd_micros, Some(99_999));
}

#[path = "roster_policy_override_tests.rs"]
mod roster_policy_override_tests;
