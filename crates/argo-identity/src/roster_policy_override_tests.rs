//! `policy_override_json` column + `lookup_for_worker` helper tests
//! for `crate::roster`.

use super::*;

#[test]
fn roster_policy_override_default_is_no_op() {
    use crate::RosterPolicyOverride;
    let ov = RosterPolicyOverride::default();
    assert!(ov.auto_approve_max_amount.is_none());
    assert!(ov.additional_quiet_hours.is_none());
    let explicit = RosterPolicyOverride {
        auto_approve_max_amount: Some(250),
        ..Default::default()
    };
    assert_ne!(ov, explicit);
}

#[tokio::test]
async fn lookup_for_worker_returns_none_when_no_roster() {
    let store = open_roster_store().await;
    let worker = WorkerId::new();
    assert!(store.lookup_for_worker(worker).await.unwrap().is_none());
}

#[tokio::test]
async fn lookup_for_worker_returns_assignment_without_override() {
    let store = open_roster_store().await;
    let tenant = TenantId::new();
    let roster = Roster::new(
        tenant,
        "sales-ops",
        AutoApprovalPolicy::default(),
        Vec::new(),
        None,
    );
    store.create(&roster).await.unwrap();
    let worker = WorkerId::new();
    store.add_member(roster.id, worker).await.unwrap();
    let assignment = store
        .lookup_for_worker(worker)
        .await
        .unwrap()
        .expect("worker has a roster");
    assert_eq!(assignment.roster.id, roster.id);
    assert!(assignment.policy_override.is_none());
}

#[tokio::test]
async fn lookup_for_worker_round_trips_override() {
    use crate::RosterPolicyOverride;
    let store = open_roster_store().await;
    let tenant = TenantId::new();
    let roster = Roster::new(
        tenant,
        "finance",
        AutoApprovalPolicy::default(),
        Vec::new(),
        None,
    );
    store.create(&roster).await.unwrap();
    let worker = WorkerId::new();
    store.add_member(roster.id, worker).await.unwrap();
    let original = RosterPolicyOverride {
        auto_approve_max_amount: Some(500),
        ..Default::default()
    };
    store
        .set_policy_override(tenant, roster.id, Some(&original))
        .await
        .unwrap();
    let assignment = store
        .lookup_for_worker(worker)
        .await
        .unwrap()
        .expect("worker has a roster");
    assert_eq!(assignment.policy_override.as_ref(), Some(&original));
}

#[tokio::test]
async fn lookup_for_worker_prefers_explicit_over_default_named() {
    let store = open_roster_store().await;
    let tenant = TenantId::new();
    let explicit = Roster::new(
        tenant,
        "tier-1-ops",
        AutoApprovalPolicy::default(),
        Vec::new(),
        None,
    );
    let fallback = Roster::new(
        tenant,
        "default",
        AutoApprovalPolicy::default(),
        Vec::new(),
        None,
    );
    store.create(&explicit).await.unwrap();
    store.create(&fallback).await.unwrap();
    let worker = WorkerId::new();
    store.add_member(explicit.id, worker).await.unwrap();
    store.add_member(fallback.id, worker).await.unwrap();
    let assignment = store
        .lookup_for_worker(worker)
        .await
        .unwrap()
        .expect("worker has a roster");
    assert_eq!(assignment.roster.id, explicit.id);
}

#[tokio::test]
async fn set_policy_override_can_clear() {
    use crate::RosterPolicyOverride;
    let store = open_roster_store().await;
    let tenant = TenantId::new();
    let roster = Roster::new(
        tenant,
        "ops",
        AutoApprovalPolicy::default(),
        Vec::new(),
        None,
    );
    store.create(&roster).await.unwrap();
    let worker = WorkerId::new();
    store.add_member(roster.id, worker).await.unwrap();
    let override_val = RosterPolicyOverride {
        auto_approve_max_amount: Some(999),
        ..Default::default()
    };
    store
        .set_policy_override(tenant, roster.id, Some(&override_val))
        .await
        .unwrap();
    store
        .set_policy_override(tenant, roster.id, None)
        .await
        .unwrap();
    let assignment = store
        .lookup_for_worker(worker)
        .await
        .unwrap()
        .expect("worker has a roster");
    assert!(assignment.policy_override.is_none());
}
