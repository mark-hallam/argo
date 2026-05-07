//! Integration tests for the worker duty scheduler (Stream
//! `claude-worker-duty-scheduler`, V1.3).
//!
//! Both tests cover the full cron-fire + gate cycle: a duty whose
//! cron window has elapsed is evaluated, the AutoApprovalPolicy
//! gate is consulted, and the resulting Session (or denial row)
//! lands in the `duty_fired_session` link table for audit.

use argo_identity::WorkerId;
use argo_orchestrator::{
    ClosedGate, DutyRecord, DutyScheduler, DutyStore, FireDecision, OpenGate, OrchestratorStore,
};
use argo_store::SessionStore;
use argo_types::TenantId;
use chrono::{Duration as CDur, TimeZone, Utc};
use std::sync::Arc;

async fn fresh_store() -> (Arc<DutyStore>, Arc<OrchestratorStore>) {
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    let duties = Arc::new(DutyStore::new(store.clone()));
    duties.ensure_schema().await.unwrap();
    let orch = Arc::new(OrchestratorStore::new(store));
    orch.ensure_schema().await.unwrap();
    (duties, orch)
}

/// Acceptance criterion: "Scheduler tick fires matching duties
/// exactly once per cron window (idempotency via last_fired_at
/// per duty)" + "Fired duty creates Session attributed to
/// started_by_duty".
#[tokio::test]
async fn tick_fires_matching_duty_exactly_once_then_records_attribution() {
    let (duties, orch) = fresh_store().await;
    let scheduler = DutyScheduler::new(duties.clone(), orch.clone(), Arc::new(OpenGate));

    let tenant = TenantId::new();
    let worker = WorkerId::new();
    let mut duty = DutyRecord::new(tenant, worker, "0 * * * * *", "ping", "every minute");
    // Anchor created_at to a deterministic moment so we control
    // which cron window the tick lands in.
    duty.created_at = Utc.with_ymd_and_hms(2026, 5, 6, 12, 0, 0).unwrap();
    duties.insert(&duty).await.unwrap();

    let tick_at = Utc.with_ymd_and_hms(2026, 5, 6, 12, 1, 30).unwrap();
    let report = scheduler.tick(tick_at).await.unwrap();
    assert_eq!(report.evaluated, 1);
    assert_eq!(report.fired, vec![duty.id.clone()]);
    assert!(report.denied_by_gate.is_empty());

    // Idempotency: a second tick within the same minute window
    // must NOT fire the duty again.
    let report2 = scheduler.tick(tick_at + CDur::seconds(10)).await.unwrap();
    assert!(report2.fired.is_empty());

    // Attribution row exists for the fire.
    let attribution = duties.sessions_for_duty(&duty.id).await.unwrap();
    assert_eq!(attribution.len(), 1);
    assert_eq!(attribution[0].2, "auto_approved");

    // Crossing into the next cron window fires the duty again.
    let next_tick = Utc.with_ymd_and_hms(2026, 5, 6, 12, 3, 0).unwrap();
    let report3 = scheduler.tick(next_tick).await.unwrap();
    assert_eq!(report3.fired.len(), 1);
    let attribution2 = duties.sessions_for_duty(&duty.id).await.unwrap();
    assert_eq!(attribution2.len(), 2);
}

/// Acceptance criterion: "AutoApprovalPolicy gate honored". With
/// a [`ClosedGate`] in place, an otherwise-firable duty is
/// recorded as `denied_gate` and no Session is created.
#[tokio::test]
async fn closed_gate_denies_fire_and_records_decision() {
    let (duties, orch) = fresh_store().await;
    let scheduler = DutyScheduler::new(duties.clone(), orch, Arc::new(ClosedGate));

    let tenant = TenantId::new();
    let worker = WorkerId::new();
    let mut duty = DutyRecord::new(tenant, worker, "0 * * * * *", "ping", "every minute");
    duty.created_at = Utc.with_ymd_and_hms(2026, 5, 6, 12, 0, 0).unwrap();
    duties.insert(&duty).await.unwrap();

    let tick_at = Utc.with_ymd_and_hms(2026, 5, 6, 12, 1, 30).unwrap();
    let report = scheduler.tick(tick_at).await.unwrap();
    assert_eq!(report.evaluated, 1);
    assert!(report.fired.is_empty());
    assert_eq!(report.denied_by_gate, vec![duty.id.clone()]);

    // last_fired_at is bumped (so the duty doesn't re-evaluate
    // every minute on a denial loop) but no real session row was
    // attributed.
    let updated = duties.get(&duty.id).await.unwrap().unwrap();
    assert_eq!(updated.last_fired_at, Some(tick_at));

    // The link table records the denial via the sentinel
    // session id; production callers can filter on `decision`.
    let attribution = duties.sessions_for_duty(&duty.id).await.unwrap();
    assert_eq!(attribution.len(), 1);
    assert_eq!(attribution[0].2, FireDecision::DeniedGate.label());
}
