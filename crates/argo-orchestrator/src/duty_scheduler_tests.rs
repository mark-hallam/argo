//! Unit tests for [`crate::duty_scheduler`].
//!
//! Extracted from the inline `#[cfg(test)] mod tests` block to keep
//! `duty_scheduler.rs` under the 500-LOC ceiling once the
//! `DutyAgentRunner` wire + `WorkerDutyFired`/`Failed` audit
//! emission landed (`fix-duty-scheduler-invokes-agent-runner`).
//! The pre-existing tests are preserved verbatim; the new
//! failure-injection + audit-row pinning tests live at the bottom.
use super::*;
use argo_audit::AuditSink;
use argo_store::SessionStore;
use chrono::Duration as CDur;
use sqlx::Row;
use std::sync::atomic::{AtomicUsize, Ordering};

async fn open_stores() -> (Arc<DutyStore>, Arc<OrchestratorStore>) {
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    let duties = Arc::new(DutyStore::new(store.clone()));
    duties.ensure_schema().await.unwrap();
    let orch = Arc::new(OrchestratorStore::new(store));
    orch.ensure_schema().await.unwrap();
    (duties, orch)
}

fn record(cron: &str) -> DutyRecord {
    DutyRecord::new(TenantId::new(), WorkerId::new(), cron, "ping", "test duty")
}

#[tokio::test]
async fn disabled_duty_never_fires() {
    let mut d = record("0 * * * * *");
    d.enabled = false;
    assert!(!d.should_fire(Utc::now()).unwrap());
}

#[tokio::test]
async fn paused_duty_skipped_until_window_passes() {
    let mut d = record("0 * * * * *");
    let now = Utc::now();
    d.paused_until = Some(now + CDur::hours(1));
    assert!(!d.should_fire(now).unwrap());
    d.paused_until = Some(now - CDur::hours(1));
    assert!(!d.should_fire(now).unwrap());
    assert!(d.should_fire(now + CDur::minutes(2)).unwrap());
}

#[tokio::test]
async fn cron_window_idempotent_via_last_fired_at() {
    let mut d = record("0 * * * * *");
    let base = Utc.with_ymd_and_hms(2026, 5, 6, 12, 0, 0).unwrap();
    d.created_at = base;
    assert!(d.should_fire(base + CDur::minutes(2)).unwrap());
    d.last_fired_at = Some(base + CDur::minutes(2));
    assert!(!d.should_fire(base + CDur::minutes(2)).unwrap());
    assert!(d.should_fire(base + CDur::minutes(3)).unwrap());
}

#[tokio::test]
async fn invalid_cron_surfaces_schema_error() {
    let d = record("not a cron");
    assert!(matches!(
        d.should_fire(Utc::now()),
        Err(ArgoError::Schema(_))
    ));
}

#[tokio::test]
async fn fire_now_creates_session_attributed_to_duty() {
    let (duties, orch) = open_stores().await;
    let scheduler = DutyScheduler::new(duties.clone(), orch, Arc::new(OpenGate));
    let d = record("0 0 * * * *");
    duties.insert(&d).await.unwrap();
    let session_id = scheduler.fire_now(d.tenant_id, &d.id).await.unwrap();
    let attribution = duties.sessions_for_duty(d.tenant_id, &d.id).await.unwrap();
    assert_eq!(attribution.len(), 1);
    assert_eq!(attribution[0].0, session_id);
    assert_eq!(attribution[0].2, "manual_fire");
}

#[tokio::test]
async fn tick_skips_paused_and_disabled_duties() {
    let (duties, orch) = open_stores().await;
    let scheduler = DutyScheduler::new(duties.clone(), orch, Arc::new(OpenGate));
    let mut paused = record("* * * * * *");
    paused.paused_until = Some(Utc::now() + CDur::hours(1));
    let mut disabled = record("* * * * * *");
    disabled.enabled = false;
    duties.insert(&paused).await.unwrap();
    duties.insert(&disabled).await.unwrap();
    let report = scheduler.tick(Utc::now()).await.unwrap();
    assert_eq!(report.evaluated, 0);
    assert!(report.fired.is_empty());
}

#[tokio::test]
async fn duty_round_trips_through_store() {
    let (duties, _) = open_stores().await;
    let d = record("0 * * * * *");
    duties.insert(&d).await.unwrap();
    assert_eq!(duties.get(d.tenant_id, &d.id).await.unwrap().unwrap(), d);
}

#[tokio::test]
async fn list_for_tenant_isolates_other_tenants() {
    let (duties, _) = open_stores().await;
    let a = TenantId::new();
    let b = TenantId::new();
    duties
        .insert(&DutyRecord::new(
            a,
            WorkerId::new(),
            "0 * * * * *",
            "ping",
            "",
        ))
        .await
        .unwrap();
    duties
        .insert(&DutyRecord::new(
            b,
            WorkerId::new(),
            "0 * * * * *",
            "ping",
            "",
        ))
        .await
        .unwrap();
    assert_eq!(duties.list_for_tenant(a).await.unwrap().len(), 1);
    assert_eq!(duties.list_for_tenant(b).await.unwrap().len(), 1);
}

#[tokio::test]
async fn pause_then_resume_round_trips() {
    let (duties, _) = open_stores().await;
    let d = record("0 * * * * *");
    duties.insert(&d).await.unwrap();
    duties
        .set_paused(d.tenant_id, &d.id, Some(Utc::now() + CDur::hours(2)))
        .await
        .unwrap();
    assert!(duties
        .get(d.tenant_id, &d.id)
        .await
        .unwrap()
        .unwrap()
        .paused_until
        .is_some());
    duties.set_paused(d.tenant_id, &d.id, None).await.unwrap();
    assert!(duties
        .get(d.tenant_id, &d.id)
        .await
        .unwrap()
        .unwrap()
        .paused_until
        .is_none());
}

// ─── DutyAgentRunner + WorkerDutyFired/Failed audit wiring ─────────
//
// These pin the contract added by
// `fix-duty-scheduler-invokes-agent-runner`:
//   1. `DutyScheduler::with_agent_runner` injects a runner whose
//      `run_turn` is invoked exactly once per auto-approved fire.
//   2. Successful runs emit a `worker_duty.fired` audit row whose
//      payload carries `tenant_id` / `worker_id` / `duty_id` /
//      `session_id` / `cron_spec` / `entry_action`.
//   3. Failed runs emit a `worker_duty.failed` audit row with the
//      error stringified; the scheduler keeps ticking subsequent
//      duties (no panic, no early return).
//   4. The attribution row + `last_fired_at` still advance on a
//      failed run (the duty *did* fire — only the agent turn
//      crashed), so a retry-storm doesn't double-fire the same
//      cron window.

#[derive(Default)]
struct RecordingRunner {
    calls: tokio::sync::Mutex<Vec<DutyRunContext>>,
    fail_with: Option<&'static str>,
}

impl RecordingRunner {
    fn fail(msg: &'static str) -> Self {
        Self {
            calls: tokio::sync::Mutex::new(Vec::new()),
            fail_with: Some(msg),
        }
    }
}

#[async_trait]
impl DutyAgentRunner for RecordingRunner {
    async fn run_turn(&self, ctx: DutyRunContext) -> ArgoResult<()> {
        self.calls.lock().await.push(ctx);
        match self.fail_with {
            None => Ok(()),
            Some(msg) => Err(ArgoError::Provider(msg.to_string())),
        }
    }
}

#[derive(Default)]
struct PanickyRunner {
    invocations: AtomicUsize,
}

#[async_trait]
impl DutyAgentRunner for PanickyRunner {
    async fn run_turn(&self, _ctx: DutyRunContext) -> ArgoResult<()> {
        self.invocations.fetch_add(1, Ordering::SeqCst);
        Err(ArgoError::Provider("synthetic agent failure".into()))
    }
}

async fn audit_rows_for_session(
    store: &Arc<SessionStore>,
    session_id: &SessionId,
) -> Vec<(String, String)> {
    let bytes = uuid::Uuid::parse_str(session_id.as_str())
        .unwrap()
        .as_bytes()
        .to_vec();
    let rows = sqlx::query(
        "SELECT kind, payload_json FROM audit_record WHERE session_id = ? ORDER BY id ASC",
    )
    .bind(bytes)
    .fetch_all(store.pool())
    .await
    .unwrap();
    rows.into_iter()
        .map(|r| {
            let k: String = r.try_get("kind").unwrap();
            let p: String = r.try_get("payload_json").unwrap();
            (k, p)
        })
        .collect()
}

#[tokio::test]
async fn tick_invokes_runner_with_duty_context_on_auto_approve() {
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    let duties = Arc::new(DutyStore::new(Arc::clone(&store)));
    duties.ensure_schema().await.unwrap();
    let orch = Arc::new(OrchestratorStore::new(Arc::clone(&store)));
    orch.ensure_schema().await.unwrap();
    let runner = Arc::new(RecordingRunner::default());

    let scheduler = DutyScheduler::new(Arc::clone(&duties), Arc::clone(&orch), Arc::new(OpenGate))
        .with_agent_runner(runner.clone());

    let tenant = TenantId::new();
    let worker = WorkerId::new();
    let mut duty = DutyRecord::new(tenant, worker, "0 * * * * *", "ping", "every minute");
    duty.created_at = Utc.with_ymd_and_hms(2026, 5, 6, 12, 0, 0).unwrap();
    duties.insert(&duty).await.unwrap();

    let tick_at = Utc.with_ymd_and_hms(2026, 5, 6, 12, 1, 30).unwrap();
    let report = scheduler.tick(tick_at).await.unwrap();
    assert_eq!(report.fired, vec![duty.id.clone()]);
    assert!(report.run_failed.is_empty());

    let calls = runner.calls.lock().await;
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
async fn tick_emits_worker_duty_fired_audit_row_with_payload() {
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    let duties = Arc::new(DutyStore::new(Arc::clone(&store)));
    duties.ensure_schema().await.unwrap();
    let orch = Arc::new(OrchestratorStore::new(Arc::clone(&store)));
    orch.ensure_schema().await.unwrap();
    let sink = Arc::new(AuditSink::new_with_tenant(
        Arc::clone(&store),
        TenantId::new().0.to_string(),
    ));
    let scheduler = DutyScheduler::new(Arc::clone(&duties), Arc::clone(&orch), Arc::new(OpenGate))
        .with_agent_runner(Arc::new(RecordingRunner::default()))
        .with_audit_sink(Arc::clone(&sink));

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

    let rows = audit_rows_for_session(&store, &session_id).await;
    let fired: Vec<_> = rows
        .iter()
        .filter(|(k, _)| k == "worker_duty.fired")
        .collect();
    assert_eq!(fired.len(), 1, "exactly one WorkerDutyFired row");
    let payload: serde_json::Value = serde_json::from_str(&fired[0].1).unwrap();
    assert_eq!(payload["tenant_id"], tenant.0.to_string());
    assert_eq!(payload["worker_id"], worker.to_string());
    assert_eq!(payload["duty_id"], duty.id);
    assert_eq!(payload["session_id"], session_id.as_str());
    assert_eq!(payload["cron_spec"], "0 * * * * *");
    assert_eq!(payload["entry_action"], "ping");
    assert!(payload.get("error").is_none(), "fired row has no error key");

    let failed: Vec<_> = rows
        .iter()
        .filter(|(k, _)| k == "worker_duty.failed")
        .collect();
    assert!(failed.is_empty(), "no failed row for a successful run");
}

#[tokio::test]
async fn tick_emits_worker_duty_failed_when_runner_errors() {
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    let duties = Arc::new(DutyStore::new(Arc::clone(&store)));
    duties.ensure_schema().await.unwrap();
    let orch = Arc::new(OrchestratorStore::new(Arc::clone(&store)));
    orch.ensure_schema().await.unwrap();
    let sink = Arc::new(AuditSink::new_with_tenant(
        Arc::clone(&store),
        TenantId::new().0.to_string(),
    ));
    let scheduler = DutyScheduler::new(Arc::clone(&duties), Arc::clone(&orch), Arc::new(OpenGate))
        .with_agent_runner(Arc::new(RecordingRunner::fail("synthetic boom")))
        .with_audit_sink(Arc::clone(&sink));

    let tenant = TenantId::new();
    let worker = WorkerId::new();
    let mut duty = DutyRecord::new(tenant, worker, "0 * * * * *", "ping", "every minute");
    duty.created_at = Utc.with_ymd_and_hms(2026, 5, 6, 12, 0, 0).unwrap();
    duties.insert(&duty).await.unwrap();

    let tick_at = Utc.with_ymd_and_hms(2026, 5, 6, 12, 1, 30).unwrap();
    let report = scheduler.tick(tick_at).await.unwrap();
    assert_eq!(report.fired, vec![duty.id.clone()], "duty still fires");
    assert_eq!(
        report.run_failed,
        vec![duty.id.clone()],
        "runner failure surfaces in run_failed"
    );

    sink.flush_pending().await.unwrap();
    let attribution = duties.sessions_for_duty(tenant, &duty.id).await.unwrap();
    assert_eq!(
        attribution.len(),
        1,
        "attribution row still written on failed run"
    );
    let reloaded = duties.get(tenant, &duty.id).await.unwrap().unwrap();
    assert!(
        reloaded.last_fired_at.is_some(),
        "last_fired_at still advances so the cron window isn't re-fired in a tight retry loop"
    );

    let rows = audit_rows_for_session(&store, &attribution[0].0).await;
    let failed: Vec<_> = rows
        .iter()
        .filter(|(k, _)| k == "worker_duty.failed")
        .collect();
    assert_eq!(failed.len(), 1, "exactly one WorkerDutyFailed row");
    let payload: serde_json::Value = serde_json::from_str(&failed[0].1).unwrap();
    assert_eq!(payload["duty_id"], duty.id);
    assert!(
        payload["error"]
            .as_str()
            .unwrap_or_default()
            .contains("synthetic boom"),
        "error string in payload: {}",
        payload["error"]
    );
}

#[tokio::test]
async fn tick_continues_after_runner_failure_for_subsequent_duties() {
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    let duties = Arc::new(DutyStore::new(Arc::clone(&store)));
    duties.ensure_schema().await.unwrap();
    let orch = Arc::new(OrchestratorStore::new(Arc::clone(&store)));
    orch.ensure_schema().await.unwrap();
    let runner = Arc::new(PanickyRunner::default());
    let scheduler = DutyScheduler::new(Arc::clone(&duties), Arc::clone(&orch), Arc::new(OpenGate))
        .with_agent_runner(runner.clone());

    let base = Utc.with_ymd_and_hms(2026, 5, 6, 12, 0, 0).unwrap();
    let mut duty_a = DutyRecord::new(
        TenantId::new(),
        WorkerId::new(),
        "0 * * * * *",
        "ping",
        "duty A",
    );
    duty_a.created_at = base;
    let mut duty_b = DutyRecord::new(
        TenantId::new(),
        WorkerId::new(),
        "0 * * * * *",
        "ping",
        "duty B",
    );
    duty_b.created_at = base;
    duties.insert(&duty_a).await.unwrap();
    duties.insert(&duty_b).await.unwrap();

    let tick_at = base + CDur::minutes(2);
    let report = scheduler.tick(tick_at).await.unwrap();
    assert_eq!(report.evaluated, 2);
    assert_eq!(
        report.fired.len(),
        2,
        "both duties fired despite per-duty runner failures"
    );
    assert_eq!(
        report.run_failed.len(),
        2,
        "both runs surfaced as failed"
    );
    assert_eq!(
        runner.invocations.load(Ordering::SeqCst),
        2,
        "the second duty's runner still ran"
    );
}
