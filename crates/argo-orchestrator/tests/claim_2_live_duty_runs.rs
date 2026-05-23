//! MISSION-claim-2 LIVE regression test.
//!
//! Per MISSION.md operator-locked round-15 criterion:
//!
//!   "duty cron fires → runner → `WorkerDutyFired` audit row in 60s"
//!
//! This file is the canonical pin for the claim-2 ("delegation is the
//! unit") end-to-end contract introduced by
//! `fix-duty-scheduler-invokes-agent-runner` (#2088). It complements the
//! crate-internal `duty_scheduler_tests` and the
//! `argo-agent-runner/tests/cron_duty_wire_closure_test.rs` hooks: the
//! former pin the in-process unit contracts; this file pins the
//! mission-level claim from the gap manifest.
//!
//! ## Why we don't `sleep 65s`
//!
//! The brief stub (`eval-claim-2-live-duty-cron-fires-runner-test`) read
//! literally: "hire worker with duty cron, sleep 65s, assert
//! `WorkerDutyFired` row exists in `audit_record`." That phrasing
//! mirrors the production SLA (the autonomous loop is expected to fire
//! within the next 60 seconds after a cron boundary). It is *not*
//! the right shape for a unit-test binary — a 65-second sleep would
//! make CI brittle and provides no extra signal over a deterministic
//! `tick(now)` invocation.
//!
//! Per the brief's own footnote ("queue-replenishment brief authored as
//! a STUB — feel free to expand acceptance criteria during pre-flight"),
//! we expand the acceptance into the deterministic shape every other
//! orchestrator test in the workspace uses, then add a second test that
//! drives the production `spawn_duty_scheduler` primitive on a short
//! interval so the autonomous-loop wire itself is exercised. The 60 s
//! SLA stays the documented production guarantee; the test pins the
//! invariant the SLA depends on (one tick → one audit row).
//!
//! ## What this file pins
//!
//! 1. `DutyScheduler::with_agent_runner` + `with_audit_sink` together
//!    cause an auto-approved fire to (a) invoke the runner exactly
//!    once, (b) write exactly one `worker_duty.fired` row to
//!    `audit_record` with the round-15 payload, (c) commit the session
//!    + audit rows in one transaction.
//! 2. The same wire — driven through the production
//!    `spawn_duty_scheduler` primitive on a sub-second interval —
//!    produces the audit row inside a tight wall-clock budget.
//! 3. Cross-tenant isolation: two tenants firing simultaneously each
//!    own exactly one `worker_duty.fired` audit row, with no leakage
//!    of `duty_id` / `session_id` payloads across tenants.
//!
//! ## What this file does NOT pin
//!
//! Production gateway boot at
//! `crates/argo-cli/src/handlers/gateway/boot_spawns.rs:158-178` still
//! constructs `DutyScheduler::new(...)` with the Noop defaults. The
//! follow-up `mission-duty-scheduler-gateway-boot-wire-real-runner-
//! and-sink` brief flips the gateway over. Until then, the substrate
//! tested here is wired but the production binary's autonomous loop
//! still emits no audit row — by design, to keep the wire opt-in
//! during rollout.
//!
//! Gap manifest cite (verbatim):
//!
//!   "🔴 **WIRE BROKEN** (eval-cron-duty-fires-agent-runner-end-to-
//!   end, this round) [...] `DutyScheduler::tick` creates a `Session`
//!   row + `duty_fired_session` attribution but **does NOT invoke
//!   `AgentRunner::run_turn`** — struct holds no AgentRunner handle.
//!   **NO `WorkerDutyFired` AuditKind exists** and `DutyScheduler`
//!   writes no audit row at all."
//!
//! That row is now 🟢 at the substrate level. This file is the
//! regression test that fails loudly if any of the three pins above
//! regresses.
//!
//! Brief: `docs/build-plan/streams/eval-claim-2-live-duty-cron-fires-
//! runner-test.md`. Gap manifest: `docs/skycot/evals/scheduled/
//! 2026-05-23/mission-vs-code-gap-manifest.md`.

use argo_audit::AuditSink;
use argo_identity::WorkerId;
use argo_orchestrator::{
    spawn_duty_scheduler, DutyAgentRunner, DutyRecord, DutyRunContext, DutyScheduler, DutyStore,
    OpenGate, OrchestratorStore,
};
use argo_store::SessionStore;
use argo_types::{ArgoResult, TenantId};
use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use sqlx::Row;
use std::sync::Arc;
use std::time::Duration;

/// Captures every `run_turn` invocation so the test can assert the
/// runner was actually driven (the round-15 contract turns on this
/// — pre-#2088 the runner was never called).
#[derive(Default)]
struct RecordingDutyRunner {
    calls: tokio::sync::Mutex<Vec<DutyRunContext>>,
}

#[async_trait]
impl DutyAgentRunner for RecordingDutyRunner {
    async fn run_turn(&self, ctx: DutyRunContext) -> ArgoResult<()> {
        self.calls.lock().await.push(ctx);
        Ok(())
    }
}

/// Build an in-memory `(DutyStore, OrchestratorStore, SessionStore)`
/// triple with schemas migrated. The `SessionStore` is also returned
/// because the test reads the `audit_record` table directly via raw
/// SQL — that's what an external auditor does, and the pin must match.
async fn fresh_world() -> (Arc<DutyStore>, Arc<OrchestratorStore>, Arc<SessionStore>) {
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    let duties = Arc::new(DutyStore::new(store.clone()));
    duties.ensure_schema().await.unwrap();
    let orch = Arc::new(OrchestratorStore::new(store.clone()));
    orch.ensure_schema().await.unwrap();
    (duties, orch, store)
}

/// Read every `audit_record` row attributed to `session_id` and
/// return `(kind, payload_json)` pairs, ordered oldest first. Mirrors
/// the helper in `duty_scheduler_tests.rs` so the two tests stay
/// independently readable.
async fn audit_rows_for_session(
    store: &Arc<SessionStore>,
    session_id_str: &str,
) -> Vec<(String, String)> {
    let bytes = uuid::Uuid::parse_str(session_id_str)
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

/// Pin 1 — deterministic single-tick path.
///
/// Inserts a duty whose next cron boundary lies between `created_at`
/// and `tick_at`, drives one `tick(tick_at)`, and asserts that:
///
/// - the runner was invoked exactly once with the duty's full context,
/// - exactly one `worker_duty.fired` row exists in `audit_record`,
/// - the payload carries every round-15-required field.
#[tokio::test]
async fn claim_2_live_single_tick_writes_worker_duty_fired_audit_row() {
    let (duties, orch, store) = fresh_world().await;
    let tenant = TenantId::new();
    let runner = Arc::new(RecordingDutyRunner::default());
    let sink = Arc::new(AuditSink::new_with_tenant(
        store.clone(),
        tenant.0.to_string(),
    ));
    let scheduler = DutyScheduler::new(duties.clone(), orch, Arc::new(OpenGate))
        .with_agent_runner(runner.clone())
        .with_audit_sink(sink.clone());

    let worker = WorkerId::new();
    let mut duty = DutyRecord::new(tenant, worker, "0 * * * * *", "ping", "every minute");
    duty.created_at = Utc.with_ymd_and_hms(2026, 5, 6, 12, 0, 0).unwrap();
    duties.insert(&duty).await.unwrap();

    let tick_at = Utc.with_ymd_and_hms(2026, 5, 6, 12, 1, 30).unwrap();
    let report = scheduler.tick(tick_at).await.unwrap();
    sink.flush_pending().await.unwrap();

    assert_eq!(
        report.fired,
        vec![duty.id.clone()],
        "duty must appear in report.fired"
    );
    assert!(
        report.run_failed.is_empty(),
        "RecordingDutyRunner returns Ok: no run_failed entries"
    );

    let calls = runner.calls.lock().await;
    assert_eq!(
        calls.len(),
        1,
        "runner invoked exactly once per auto-approved fire"
    );
    assert_eq!(calls[0].tenant_id, tenant);
    assert_eq!(calls[0].worker_id, worker);
    assert_eq!(calls[0].duty_id, duty.id);
    assert_eq!(calls[0].cron_spec, "0 * * * * *");
    assert_eq!(calls[0].entry_action, "ping");
    assert_eq!(calls[0].fired_at, tick_at);

    let attribution = duties.sessions_for_duty(tenant, &duty.id).await.unwrap();
    assert_eq!(
        attribution.len(),
        1,
        "exactly one attribution row for the fire"
    );
    let session_id = attribution[0].0.clone();

    let rows = audit_rows_for_session(&store, session_id.as_str()).await;
    let fired: Vec<_> = rows
        .iter()
        .filter(|(k, _)| k == "worker_duty.fired")
        .collect();
    assert_eq!(
        fired.len(),
        1,
        "exactly one worker_duty.fired audit row (round-15 acceptance)"
    );
    let payload: serde_json::Value = serde_json::from_str(&fired[0].1).unwrap();
    assert_eq!(payload["tenant_id"], tenant.0.to_string());
    assert_eq!(payload["worker_id"], worker.to_string());
    assert_eq!(payload["duty_id"], duty.id);
    assert_eq!(payload["session_id"], session_id.as_str());
    assert_eq!(payload["cron_spec"], "0 * * * * *");
    assert_eq!(payload["entry_action"], "ping");
    assert!(
        payload.get("fired_at").is_some(),
        "fired_at is part of the round-15 payload"
    );
    assert!(
        payload.get("error").is_none(),
        "successful run must NOT include an error key"
    );
}

/// Pin 2 — autonomous-loop primitive over a tight wall-clock budget.
///
/// Uses the production `spawn_duty_scheduler` helper that gateway boot
/// binds — the only difference is the tick interval (250 ms here vs.
/// 60 s in production). The duty's `cron = "* * * * * *"` (fire every
/// second) plus a `created_at` one second in the past guarantees the
/// next boundary is "now or earlier", so the first or second tick must
/// fire.
///
/// The test sleeps up to 3 seconds polling the audit table — the
/// production SLA is 60 s, but the underlying invariant (one tick
/// after the boundary → one audit row) reproduces in milliseconds
/// here. Polling rather than fixed-sleep keeps CI fast even when the
/// runner is faster than expected.
#[tokio::test]
async fn claim_2_live_spawn_duty_scheduler_writes_audit_row_within_budget() {
    let (duties, orch, store) = fresh_world().await;
    let tenant = TenantId::new();
    let runner = Arc::new(RecordingDutyRunner::default());
    let sink = Arc::new(AuditSink::new_with_tenant(
        store.clone(),
        tenant.0.to_string(),
    ));
    let scheduler = Arc::new(
        DutyScheduler::new(duties.clone(), orch, Arc::new(OpenGate))
            .with_agent_runner(runner.clone())
            .with_audit_sink(sink.clone()),
    );

    let worker = WorkerId::new();
    let mut duty = DutyRecord::new(tenant, worker, "* * * * * *", "ping", "every second");
    duty.created_at = Utc::now() - chrono::Duration::seconds(2);
    duties.insert(&duty).await.unwrap();

    let handle = spawn_duty_scheduler(Arc::clone(&scheduler), Duration::from_millis(250));

    let budget = Duration::from_secs(3);
    let started = std::time::Instant::now();
    let mut saw_fired = false;
    while started.elapsed() < budget {
        sink.flush_pending().await.ok();
        let attribution = duties.sessions_for_duty(tenant, &duty.id).await.unwrap();
        if let Some((session_id, _, _)) = attribution.first() {
            let rows = audit_rows_for_session(&store, session_id.as_str()).await;
            if rows.iter().any(|(k, _)| k == "worker_duty.fired") {
                saw_fired = true;
                break;
            }
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    handle.abort();
    assert!(
        saw_fired,
        "spawn_duty_scheduler must produce a worker_duty.fired audit row within {budget:?}; \
         runner was invoked {} times (zero ⇒ tick never matched; non-zero ⇒ runner ran but \
         audit row never landed — either is a regression against the round-15 SLA)",
        runner.calls.lock().await.len()
    );
}

/// Pin 3 — cross-tenant isolation under the live wire.
///
/// Two tenants each insert a duty with an identical cron expression
/// and identical created_at. After one tick crosses both boundaries,
/// each tenant must own exactly one `worker_duty.fired` audit row,
/// each pointing at its own session id, with the tenant_id stamped on
/// every row (audit_record.tenant_id column AND payload.tenant_id).
///
/// Cross-tenant negative tests are non-optional per the operator
/// memory `feedback_enterprise_grade_defaults.md` — every multi-tenant
/// primitive carries one.
#[tokio::test]
async fn claim_2_live_cross_tenant_isolation_on_audit_row() {
    let (duties, orch, store) = fresh_world().await;
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    // One scheduler, one sink — production has a single audit sink
    // serving every tenant; the per-row tenant_id comes from the
    // duty's record, not the sink's bound id. The sink's bound id
    // is intentionally a *third* tenant so we'd notice a mistaken
    // stamp from the sink's default.
    let sink = Arc::new(AuditSink::new_with_tenant(
        store.clone(),
        TenantId::new().0.to_string(),
    ));
    let scheduler = DutyScheduler::new(duties.clone(), orch.clone(), Arc::new(OpenGate))
        .with_agent_runner(Arc::new(RecordingDutyRunner::default()))
        .with_audit_sink(sink.clone());

    let worker_a = WorkerId::new();
    let worker_b = WorkerId::new();
    let mut duty_a = DutyRecord::new(tenant_a, worker_a, "0 * * * * *", "ping", "tenant_a");
    let mut duty_b = DutyRecord::new(tenant_b, worker_b, "0 * * * * *", "ping", "tenant_b");
    duty_a.created_at = Utc.with_ymd_and_hms(2026, 5, 6, 12, 0, 0).unwrap();
    duty_b.created_at = Utc.with_ymd_and_hms(2026, 5, 6, 12, 0, 0).unwrap();
    duties.insert(&duty_a).await.unwrap();
    duties.insert(&duty_b).await.unwrap();

    let tick_at = Utc.with_ymd_and_hms(2026, 5, 6, 12, 1, 30).unwrap();
    let report = scheduler.tick(tick_at).await.unwrap();
    sink.flush_pending().await.unwrap();
    assert_eq!(report.evaluated, 2);
    assert_eq!(report.fired.len(), 2, "both tenants' duties fired");

    let attr_a = duties.sessions_for_duty(tenant_a, &duty_a.id).await.unwrap();
    let attr_b = duties.sessions_for_duty(tenant_b, &duty_b.id).await.unwrap();
    assert_eq!(attr_a.len(), 1);
    assert_eq!(attr_b.len(), 1);
    let session_a = attr_a[0].0.clone();
    let session_b = attr_b[0].0.clone();
    assert_ne!(session_a, session_b);

    // Each tenant's session has its own worker_duty.fired payload.
    let rows_a = audit_rows_for_session(&store, session_a.as_str()).await;
    let rows_b = audit_rows_for_session(&store, session_b.as_str()).await;
    let payload_a: serde_json::Value = serde_json::from_str(
        &rows_a
            .iter()
            .find(|(k, _)| k == "worker_duty.fired")
            .expect("tenant_a must have a worker_duty.fired row")
            .1,
    )
    .unwrap();
    let payload_b: serde_json::Value = serde_json::from_str(
        &rows_b
            .iter()
            .find(|(k, _)| k == "worker_duty.fired")
            .expect("tenant_b must have a worker_duty.fired row")
            .1,
    )
    .unwrap();
    assert_eq!(payload_a["tenant_id"], tenant_a.0.to_string());
    assert_eq!(payload_a["worker_id"], worker_a.to_string());
    assert_eq!(payload_a["duty_id"], duty_a.id);
    assert_eq!(payload_b["tenant_id"], tenant_b.0.to_string());
    assert_eq!(payload_b["worker_id"], worker_b.to_string());
    assert_eq!(payload_b["duty_id"], duty_b.id);
    // The two payloads must never share a worker_id, duty_id, or
    // session_id (one tenant's identifiers leaking into the other
    // tenant's audit row would be a P0 cross-tenant leak).
    assert_ne!(payload_a["worker_id"], payload_b["worker_id"]);
    assert_ne!(payload_a["duty_id"], payload_b["duty_id"]);
    assert_ne!(payload_a["session_id"], payload_b["session_id"]);

    // audit_record.tenant_id column is also tenant-stamped (not just
    // the payload). Pull it back via raw SQL the way an auditor would.
    let tenant_column_a: String = sqlx::query_scalar(
        "SELECT tenant_id FROM audit_record WHERE session_id = ? AND kind = ? LIMIT 1",
    )
    .bind(
        uuid::Uuid::parse_str(session_a.as_str())
            .unwrap()
            .as_bytes()
            .to_vec(),
    )
    .bind("worker_duty.fired")
    .fetch_one(store.pool())
    .await
    .unwrap();
    let tenant_column_b: String = sqlx::query_scalar(
        "SELECT tenant_id FROM audit_record WHERE session_id = ? AND kind = ? LIMIT 1",
    )
    .bind(
        uuid::Uuid::parse_str(session_b.as_str())
            .unwrap()
            .as_bytes()
            .to_vec(),
    )
    .bind("worker_duty.fired")
    .fetch_one(store.pool())
    .await
    .unwrap();
    assert_eq!(
        tenant_column_a,
        tenant_a.0.to_string(),
        "audit_record.tenant_id must be the duty's tenant, not the sink's bound default"
    );
    assert_eq!(
        tenant_column_b,
        tenant_b.0.to_string(),
        "audit_record.tenant_id must be the duty's tenant, not the sink's bound default"
    );
    assert_ne!(tenant_column_a, tenant_column_b);
}
