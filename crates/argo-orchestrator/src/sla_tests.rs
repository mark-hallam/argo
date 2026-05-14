use super::sla::{
    BreachAction, EscalationPolicy, ObservedRun, Sla, SlaStore, SlaTarget, SlaTracker,
};
use argo_identity::WorkerId;
use argo_store::SessionStore;
use argo_types::TenantId;
use chrono::{Duration, Utc};
use std::sync::Arc;

async fn store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    Arc::new(store)
}

fn worker_sla(tenant: TenantId, latency: u64) -> (WorkerId, Sla) {
    let worker = WorkerId::new();
    let sla = Sla::new(
        tenant,
        SlaTarget::Worker(worker),
        latency,
        EscalationPolicy::Notify("slack:#ops".into()),
        BreachAction::Notify,
    );
    (worker, sla)
}

#[tokio::test]
async fn crud_round_trips_sla_definition() {
    let store = SlaStore::new(store().await);
    let tenant = TenantId::new();
    let (_, mut sla) = worker_sla(tenant, 5000);
    store.create(&sla).await.unwrap();
    assert_eq!(store.list(tenant).await.unwrap().len(), 1);
    let fetched = store.get(tenant, &sla.id).await.unwrap().unwrap();
    assert_eq!(fetched.target_latency_ms, 5000);
    sla.target_latency_ms = 7500;
    sla.enabled = false;
    sla.updated_at = Utc::now();
    store.update(&sla).await.unwrap();
    let updated = store.get(tenant, &sla.id).await.unwrap().unwrap();
    assert_eq!(updated.target_latency_ms, 7500);
    assert!(!updated.enabled);
    assert!(store.delete(tenant, &sla.id).await.unwrap());
    assert!(store.get(tenant, &sla.id).await.unwrap().is_none());
}

#[test]
fn tracker_emits_breach_when_latency_exceeds_threshold() {
    let tenant = TenantId::new();
    let (worker, sla) = worker_sla(tenant, 1000);
    let now = Utc::now();
    let run = ObservedRun {
        tenant_id: tenant,
        target: SlaTarget::Worker(worker),
        started_at: now - Duration::milliseconds(1500),
        observed_at: now,
    };
    let event = SlaTracker::evaluate(&sla, &run).unwrap();
    assert_eq!(event.observed_latency_ms, 1500);
    assert_eq!(event.threshold_ms, 1000);
    assert_eq!(event.action, BreachAction::Notify);
}

#[test]
fn tracker_ignores_in_scope_run_within_threshold() {
    let tenant = TenantId::new();
    let (worker, sla) = worker_sla(tenant, 2000);
    let now = Utc::now();
    let run = ObservedRun {
        tenant_id: tenant,
        target: SlaTarget::Worker(worker),
        started_at: now - Duration::milliseconds(250),
        observed_at: now,
    };
    assert!(SlaTracker::evaluate(&sla, &run).is_none());
}

#[test]
fn tracker_ignores_wrong_tenant_and_target() {
    let tenant = TenantId::new();
    let (_, sla) = worker_sla(tenant, 1000);
    let now = Utc::now();
    let run = ObservedRun {
        tenant_id: TenantId::new(),
        target: SlaTarget::Skill("invoice".into()),
        started_at: now - Duration::milliseconds(5000),
        observed_at: now,
    };
    assert!(SlaTracker::evaluate(&sla, &run).is_none());
}

#[test]
fn disabled_sla_does_not_breach() {
    let tenant = TenantId::new();
    let (worker, mut sla) = worker_sla(tenant, 1000);
    sla.enabled = false;
    let now = Utc::now();
    let run = ObservedRun {
        tenant_id: tenant,
        target: SlaTarget::Worker(worker),
        started_at: now - Duration::milliseconds(5000),
        observed_at: now,
    };
    assert!(SlaTracker::evaluate(&sla, &run).is_none());
}

#[test]
fn escalation_policy_round_trips_storage_label() {
    let notify = EscalationPolicy::Notify("email:ops@example.com".into());
    let reroute = EscalationPolicy::Reroute("worker:oncall".into());
    assert_eq!(EscalationPolicy::decode(&notify.encode()), notify);
    assert_eq!(EscalationPolicy::decode(&reroute.encode()), reroute);
    assert_eq!(EscalationPolicy::decode("fail"), EscalationPolicy::Fail);
}

#[tokio::test]
async fn breach_event_persists() {
    let store = SlaStore::new(store().await);
    let tenant = TenantId::new();
    let (worker, sla) = worker_sla(tenant, 100);
    store.create(&sla).await.unwrap();
    let now = Utc::now();
    let run = ObservedRun {
        tenant_id: tenant,
        target: SlaTarget::Worker(worker),
        started_at: now - Duration::milliseconds(250),
        observed_at: now,
    };
    let event = SlaTracker::evaluate(&sla, &run).unwrap();
    store.record_breach(&event).await.unwrap();
}
