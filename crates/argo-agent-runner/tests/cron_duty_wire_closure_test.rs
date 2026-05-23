//! Wire-closure tests for `fix-duty-scheduler-invokes-agent-runner`.
//!
//! Extracted from `cron_duty_e2e_test.rs` to keep that file under the
//! HARD RULE 6 / 500-LOC ceiling once the new `DutyAgentRunner` +
//! `WorkerDutyFired`/`Failed` audit assertions landed.
//!
//! ## What this file covers
//!
//! These were `#[ignore]`'d FINDINGS-ONLY hooks on PR #2085 because
//! the wire they assert did not exist on `origin/main`. They are now
//! live: `DutyScheduler` holds an injected `DutyAgentRunner` and an
//! optional `AuditSink`; on each auto-approved fire it invokes the
//! runner and writes a `WorkerDutyFired` (or `WorkerDutyFailed`)
//! audit row. The session row + audit row commit inside the same
//! sqlx transaction — if the audit write fails the session row is
//! rolled back too (atomic, not best-effort).
//!
//! The companion file `cron_duty_e2e_test.rs` retains the three
//! pre-existing happy/no-match/cross-tenant tests for the cron
//! windowing + attribution layer.

use argo_audit::AuditSink;
use argo_identity::WorkerId;
use argo_orchestrator::{
    DutyAgentRunner, DutyRecord, DutyRunContext, DutyScheduler, DutyStore, OpenGate,
    OrchestratorStore,
};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use sqlx::Row;
use std::sync::Arc;

/// Build an in-memory `(DutyStore, OrchestratorStore)` pair, schemas
/// migrated. Mirrors the helper in `cron_duty_e2e_test.rs`; duplicated
/// here so the two test binaries remain independent.
async fn fresh_stores() -> (Arc<DutyStore>, Arc<OrchestratorStore>, Arc<SessionStore>) {
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    let duties = Arc::new(DutyStore::new(store.clone()));
    duties.ensure_schema().await.unwrap();
    let orch = Arc::new(OrchestratorStore::new(store.clone()));
    orch.ensure_schema().await.unwrap();
    (duties, orch, store)
}

#[derive(Default)]
struct RecordingAgentRunner {
    calls: tokio::sync::Mutex<Vec<DutyRunContext>>,
}

#[async_trait]
impl DutyAgentRunner for RecordingAgentRunner {
    async fn run_turn(&self, ctx: DutyRunContext) -> ArgoResult<()> {
        self.calls.lock().await.push(ctx);
        Ok(())
    }
}

struct AlwaysErrRunner;

#[async_trait]
impl DutyAgentRunner for AlwaysErrRunner {
    async fn run_turn(&self, _ctx: DutyRunContext) -> ArgoResult<()> {
        Err(ArgoError::Provider("synthetic agent failure".into()))
    }
}

#[tokio::test]
async fn duty_tick_invokes_agent_runner() {
    let (duties, orch, _store) = fresh_stores().await;
    let probe = Arc::new(RecordingAgentRunner::default());
    let scheduler = DutyScheduler::new(duties.clone(), orch.clone(), Arc::new(OpenGate))
        .with_agent_runner(probe.clone());

    let tenant = TenantId::new();
    let worker = WorkerId::new();
    let mut duty = DutyRecord::new(tenant, worker, "0 * * * * *", "ping", "every minute");
    duty.created_at = Utc.with_ymd_and_hms(2026, 5, 6, 12, 0, 0).unwrap();
    duties.insert(&duty).await.unwrap();

    let tick_at = Utc.with_ymd_and_hms(2026, 5, 6, 12, 1, 30).unwrap();
    let report = scheduler.tick(tick_at).await.unwrap();
    assert_eq!(report.fired, vec![duty.id.clone()]);
    assert!(report.run_failed.is_empty());

    let calls = probe.calls.lock().await;
    assert_eq!(calls.len(), 1, "runner invoked exactly once per fire");
    let call = &calls[0];
    assert_eq!(call.tenant_id, tenant);
    assert_eq!(call.worker_id, worker);
    assert_eq!(call.duty_id, duty.id);
    assert_eq!(call.entry_action, "ping");
    assert_eq!(call.cron_spec, "0 * * * * *");
    assert_eq!(call.fired_at, tick_at);
}

#[tokio::test]
async fn duty_tick_emits_worker_duty_fired_audit() {
    let (duties, orch, store) = fresh_stores().await;
    let sink = Arc::new(AuditSink::new_with_tenant(
        store.clone(),
        TenantId::new().0.to_string(),
    ));
    let scheduler = DutyScheduler::new(duties.clone(), orch.clone(), Arc::new(OpenGate))
        .with_agent_runner(Arc::new(RecordingAgentRunner::default()))
        .with_audit_sink(sink.clone());

    let tenant = TenantId::new();
    let worker = WorkerId::new();
    let mut duty = DutyRecord::new(tenant, worker, "0 * * * * *", "ping", "every minute");
    duty.created_at = Utc.with_ymd_and_hms(2026, 5, 6, 12, 0, 0).unwrap();
    duties.insert(&duty).await.unwrap();

    let tick_at = Utc.with_ymd_and_hms(2026, 5, 6, 12, 1, 30).unwrap();
    scheduler.tick(tick_at).await.unwrap();

    sink.flush_pending().await.unwrap();
    let attribution = duties.sessions_for_duty(tenant, &duty.id).await.unwrap();
    assert_eq!(attribution.len(), 1);
    let session_id = attribution[0].0.clone();
    let session_uuid_bytes = uuid::Uuid::parse_str(session_id.as_str())
        .unwrap()
        .as_bytes()
        .to_vec();
    let rows = sqlx::query(
        "SELECT kind, payload_json FROM audit_record WHERE session_id = ?",
    )
    .bind(session_uuid_bytes)
    .fetch_all(store.pool())
    .await
    .unwrap();
    let mut saw_fired = false;
    for r in &rows {
        let kind: String = r.try_get("kind").unwrap();
        if kind == "worker_duty.fired" {
            saw_fired = true;
            let payload: String = r.try_get("payload_json").unwrap();
            let v: serde_json::Value = serde_json::from_str(&payload).unwrap();
            assert_eq!(v["duty_id"], duty.id);
            assert_eq!(v["session_id"], session_id.as_str());
            assert_eq!(v["cron_spec"], duty.cron);
            assert_eq!(v["entry_action"], duty.entry_action);
        }
    }
    assert!(saw_fired, "expected a worker_duty.fired audit row");
}

#[tokio::test]
async fn duty_tick_emits_failed_audit_and_keeps_ticking_on_runner_err() {
    let (duties, orch, store) = fresh_stores().await;
    // Shared tenant across AuditSink + DutyRecord so the
    // tenant-scoped sessions_for_duty(tenant, &duty_id) lookup
    // below resolves the row inserted by this test (signature
    // grew via #2091; fix-cron-duty-wire-closure-test-rebase-
    // onto-tenant-id-arg threads this through).
    let tenant = TenantId::new();
    let sink = Arc::new(AuditSink::new_with_tenant(
        store.clone(),
        tenant.0.to_string(),
    ));
    let scheduler = DutyScheduler::new(duties.clone(), orch.clone(), Arc::new(OpenGate))
        .with_agent_runner(Arc::new(AlwaysErrRunner))
        .with_audit_sink(sink.clone());

    let mut duty = DutyRecord::new(
        tenant,
        WorkerId::new(),
        "0 * * * * *",
        "ping",
        "broken",
    );
    duty.created_at = Utc.with_ymd_and_hms(2026, 5, 6, 12, 0, 0).unwrap();
    duties.insert(&duty).await.unwrap();

    let tick_at = Utc.with_ymd_and_hms(2026, 5, 6, 12, 1, 30).unwrap();
    let report = scheduler.tick(tick_at).await.unwrap();
    assert_eq!(report.fired, vec![duty.id.clone()]);
    assert_eq!(report.run_failed, vec![duty.id.clone()]);

    sink.flush_pending().await.unwrap();
    let attribution = duties.sessions_for_duty(tenant, &duty.id).await.unwrap();
    let session_id = attribution[0].0.clone();
    let session_uuid_bytes = uuid::Uuid::parse_str(session_id.as_str())
        .unwrap()
        .as_bytes()
        .to_vec();
    let rows = sqlx::query(
        "SELECT kind, payload_json FROM audit_record WHERE session_id = ?",
    )
    .bind(session_uuid_bytes)
    .fetch_all(store.pool())
    .await
    .unwrap();
    let mut saw_failed = false;
    for r in &rows {
        let kind: String = r.try_get("kind").unwrap();
        if kind == "worker_duty.failed" {
            saw_failed = true;
            let payload: String = r.try_get("payload_json").unwrap();
            let v: serde_json::Value = serde_json::from_str(&payload).unwrap();
            assert_eq!(v["duty_id"], duty.id);
            assert!(
                v["error"]
                    .as_str()
                    .unwrap()
                    .contains("synthetic agent failure"),
                "error string in payload: {}",
                v["error"]
            );
        }
    }
    assert!(saw_failed, "expected a worker_duty.failed audit row");
    assert_eq!(
        attribution[0].2,
        argo_orchestrator::FireDecision::AutoApprove.label(),
        "attribution row still records auto_approved even though the runner failed"
    );
}

/// Audit-failure injection: drop the `audit_record` table after the
/// schema migration so the `AuditSink::write_in_tx` call inside the
/// scheduler's atomic-commit path fails. The session row inserted in
/// the same transaction MUST roll back, leaving no orchestrator
/// session for the duty.
///
/// This is the "atomic rollback" contract demanded by
/// `mission-cred-get-tenant-parameter` (#2087)'s parallel atomicity
/// expectation and explicitly called out in the
/// `fix-duty-scheduler-invokes-agent-runner` tick-85 review: "audit +
/// session writes share a transaction" → if audit fails, session is
/// absent.
#[tokio::test]
async fn duty_tick_rolls_back_session_when_audit_write_fails() {
    let (duties, orch, store) = fresh_stores().await;
    // Force the in-tx audit write to fail by dropping the table the
    // sink inserts into. The session insert still has a healthy table
    // (the orchestrator's `session` table is independent) but they
    // share a transaction, so the audit failure must roll back the
    // session insert.
    sqlx::query("DROP TABLE audit_record")
        .execute(store.pool())
        .await
        .unwrap();
    let sink = Arc::new(AuditSink::new_with_tenant(
        store.clone(),
        TenantId::new().0.to_string(),
    ));
    let scheduler = DutyScheduler::new(duties.clone(), orch.clone(), Arc::new(OpenGate))
        .with_agent_runner(Arc::new(RecordingAgentRunner::default()))
        .with_audit_sink(sink.clone());

    let tenant = TenantId::new();
    let worker = WorkerId::new();
    let mut duty = DutyRecord::new(tenant, worker, "0 * * * * *", "ping", "every minute");
    duty.created_at = Utc.with_ymd_and_hms(2026, 5, 6, 12, 0, 0).unwrap();
    duties.insert(&duty).await.unwrap();

    let tick_at = Utc.with_ymd_and_hms(2026, 5, 6, 12, 1, 30).unwrap();
    // The scheduler swallows the audit failure and reports the duty
    // as not fired (no attribution row, no session); the loop keeps
    // ticking on the next iteration.
    let report = scheduler.tick(tick_at).await.unwrap();
    assert!(
        report.fired.is_empty(),
        "duty must NOT be reported as fired when audit failure rolls back the tx"
    );

    // Atomic contract: no session row exists for this duty's tenant.
    let session_count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM session WHERE tenant_id = ?")
            .bind(tenant.0.as_bytes().as_slice())
            .fetch_one(store.pool())
            .await
            .unwrap();
    assert_eq!(
        session_count.0, 0,
        "session row must be absent — audit + session share a transaction"
    );
    let attribution = duties.sessions_for_duty(tenant, &duty.id).await.unwrap();
    assert!(
        attribution.is_empty(),
        "attribution row must be absent on rolled-back fire"
    );
}
