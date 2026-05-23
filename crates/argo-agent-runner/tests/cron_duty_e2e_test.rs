//! Mission-claim 2 (delegation-is-the-unit) regression hook for the
//! `Scheduler::tick → JobRunner → AgentRunner` chain.
//!
//! Brief: `eval-cron-duty-fires-agent-runner-end-to-end`.
//! Source row: round-8 manifest `docs/skycot/evals/scheduled/2026-05-23/
//! mission-vs-code-gap-manifest.md`, argo-cron line.
//!
//! ## FINDINGS-ONLY — wire is broken end-to-end
//!
//! The brief asked: "does the duty `entry_action` actually invoke
//! the worker's agent runner?" The production trace says **no**.
//!
//! Pre-flight grep (read-only, on origin/main):
//!
//! 1. The duty loop wired at gateway boot is
//!    [`argo_orchestrator::DutyScheduler`], not
//!    [`argo_cron::Scheduler`]. See
//!    `crates/argo-cli/src/handlers/gateway/boot_spawns.rs:158` —
//!    `spawn_gateway_duty_scheduler`. The `argo-cron` crate's
//!    `JobRunner`/`Scheduler` are *not* spawned anywhere in
//!    gateway boot.
//!
//! 2. [`argo_orchestrator::DutyScheduler::tick`] only inserts an
//!    orchestrator [`Session`] row and a `duty_fired_session`
//!    attribution row. It does **not** invoke any `AgentRunner` —
//!    in fact, the struct holds no `AgentRunner` handle at all.
//!    See `crates/argo-orchestrator/src/duty_scheduler.rs:218`,
//!    `spawn_session`.
//!
//! 3. No `WorkerDutyFired` `AuditKind` exists; `DutyScheduler::tick`
//!    writes no audit row at all. The closest variant in
//!    [`argo_audit::AuditKind`] is `CronJobFired`, which belongs to
//!    `argo-cron`'s [`argo_cron::Scheduler::tick_with_audit`] (a
//!    separate, not-wired path).
//!
//! ## What this test file does
//!
//! The three "passing" tests below pin the duty scheduler's
//! **current** behaviour assertively (happy / no-match negative /
//! cross-tenant isolation). They catch regressions in the wire that
//! *does* exist today: cron windowing, gate evaluation, attribution
//! row writes, tenant binding on the session row.
//!
//! The two `#[ignore]`'d tests at the bottom are FINDINGS-ONLY
//! regression hooks for the wire that does **not** exist:
//! `DutyScheduler → AgentRunner::run_turn` and
//! `DutyScheduler → audit_record(WorkerDutyFired)`. They `panic!`
//! with a FINDINGS-ONLY message describing the gap; when a follow-up
//! brief lands the wire, the implementer flips off the `#[ignore]`
//! and replaces the panic with the real assertions.
//!
//! ## Why this file lives in `argo-agent-runner/tests/`
//!
//! Per brief may-touch list. The agent-runner crate already depends
//! on `argo-orchestrator`, `argo-store`, `argo-identity`,
//! `argo-audit` and `argo-types`, so no new dev-deps are required;
//! `argo-cron/tests/` would have required adding all four to a
//! crate that doesn't otherwise need them. The brief's
//! "AgentRunner is the natural test owner" clause covers this
//! choice — the FINDINGS-ONLY hooks belong next to the runner
//! they would call.

use argo_identity::WorkerId;
use argo_orchestrator::{
    DutyRecord, DutyScheduler, DutyStore, FireDecision, OpenGate, OrchestratorStore,
};
use argo_store::SessionStore;
use argo_types::TenantId;
use chrono::{TimeZone, Utc};
use std::sync::Arc;

/// Build an in-memory `(DutyStore, OrchestratorStore)` pair, schemas
/// migrated. Mirrors the helper in
/// `crates/argo-orchestrator/tests/duty_scheduler_integration.rs`.
async fn fresh_stores() -> (Arc<DutyStore>, Arc<OrchestratorStore>) {
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    let duties = Arc::new(DutyStore::new(store.clone()));
    duties.ensure_schema().await.unwrap();
    let orch = Arc::new(OrchestratorStore::new(store));
    orch.ensure_schema().await.unwrap();
    (duties, orch)
}

/// Happy path. A duty whose cron window has elapsed on the tick
/// instant produces:
///
/// - exactly one `report.fired` entry for that duty id,
/// - exactly one `duty_fired_session` attribution row with the
///   `auto_approved` decision,
/// - exactly one active `Session` row on the orchestrator store
///   for the duty's tenant,
/// - the session participant list contains the duty's worker.
///
/// All four are assertive — no smoke-only checks.
#[tokio::test]
async fn duty_tick_creates_session_for_matching_cron() {
    let (duties, orch) = fresh_stores().await;
    let scheduler = DutyScheduler::new(duties.clone(), orch.clone(), Arc::new(OpenGate));

    let tenant = TenantId::new();
    let worker = WorkerId::new();
    let mut duty = DutyRecord::new(tenant, worker, "0 * * * * *", "ping", "every minute");
    // Anchor created_at so the next cron boundary (12:01:00) lies
    // strictly between created_at and tick_at, and the elapsed-
    // window predicate fires.
    duty.created_at = Utc.with_ymd_and_hms(2026, 5, 6, 12, 0, 0).unwrap();
    duties.insert(&duty).await.unwrap();

    let tick_at = Utc.with_ymd_and_hms(2026, 5, 6, 12, 1, 30).unwrap();
    let report = scheduler.tick(tick_at).await.unwrap();

    assert_eq!(report.evaluated, 1, "exactly one duty was evaluated");
    assert_eq!(
        report.fired,
        vec![duty.id.clone()],
        "the matching duty fired"
    );
    assert!(
        report.denied_by_gate.is_empty(),
        "OpenGate must not deny: {:?}",
        report.denied_by_gate
    );

    let attribution = duties.sessions_for_duty(tenant, &duty.id).await.unwrap();
    assert_eq!(
        attribution.len(),
        1,
        "exactly one attribution row for the fire"
    );
    assert_eq!(
        attribution[0].2,
        FireDecision::AutoApprove.label(),
        "fire decision recorded as auto_approved"
    );

    let sessions = orch.list_active_sessions(tenant).await.unwrap();
    assert_eq!(
        sessions.len(),
        1,
        "exactly one active session was created on the duty's tenant"
    );
    assert_eq!(
        sessions[0].id, attribution[0].0,
        "session id on the orchestrator store matches the attribution row"
    );
    assert_eq!(
        sessions[0].tenant_id, tenant,
        "session inherited the duty's tenant_id (cross-tenant safety bound)"
    );
    assert!(
        sessions[0].participants.contains(&worker),
        "session participants list contains the duty's worker_id"
    );
}

/// Negative-paired test. A duty whose cron window has **not**
/// elapsed on the tick instant must not fire, must not produce an
/// attribution row, and must not create a session.
#[tokio::test]
async fn duty_tick_does_not_fire_for_non_matching_cron() {
    let (duties, orch) = fresh_stores().await;
    let scheduler = DutyScheduler::new(duties.clone(), orch.clone(), Arc::new(OpenGate));

    let tenant = TenantId::new();
    let worker = WorkerId::new();
    // cron = "0 * * * * *" → next boundary after created_at (12:00:00)
    // is 12:01:00. tick at 12:00:30 is BEFORE that boundary → no fire.
    let mut duty = DutyRecord::new(tenant, worker, "0 * * * * *", "ping", "every minute");
    duty.created_at = Utc.with_ymd_and_hms(2026, 5, 6, 12, 0, 0).unwrap();
    duties.insert(&duty).await.unwrap();

    let tick_at = Utc.with_ymd_and_hms(2026, 5, 6, 12, 0, 30).unwrap();
    let report = scheduler.tick(tick_at).await.unwrap();

    assert_eq!(
        report.evaluated, 1,
        "the duty was inspected (list_active returns it)"
    );
    assert!(
        report.fired.is_empty(),
        "no fire because next cron boundary {next} is after tick {tick}",
        next = "12:01:00",
        tick = "12:00:30"
    );
    assert!(report.denied_by_gate.is_empty());

    let attribution = duties.sessions_for_duty(tenant, &duty.id).await.unwrap();
    assert!(
        attribution.is_empty(),
        "no attribution row written for a non-firing tick: {attribution:?}"
    );

    let sessions = orch.list_active_sessions(tenant).await.unwrap();
    assert!(
        sessions.is_empty(),
        "no session row created for a non-firing tick: {sessions:?}"
    );

    let reloaded = duties.get(tenant, &duty.id).await.unwrap().unwrap();
    assert!(
        reloaded.last_fired_at.is_none(),
        "last_fired_at must NOT advance on a no-match tick: {:?}",
        reloaded.last_fired_at
    );
}

/// Cross-tenant safety. Two tenants each have a duty with an
/// identical cron expression. After one tick at a matching instant,
/// each tenant must own exactly its own session — no leakage of one
/// tenant's session into the other tenant's `list_active_sessions`
/// query, and each session's participant list contains only that
/// tenant's worker.
///
/// Non-optional per `feedback_enterprise_grade_defaults.md`: every
/// multi-tenant primitive needs a cross-tenant negative test.
#[tokio::test]
async fn duty_tick_cross_tenant_isolation() {
    let (duties, orch) = fresh_stores().await;
    let scheduler = DutyScheduler::new(duties.clone(), orch.clone(), Arc::new(OpenGate));

    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    let worker_a = WorkerId::new();
    let worker_b = WorkerId::new();

    let mut duty_a = DutyRecord::new(tenant_a, worker_a, "0 * * * * *", "ping", "tenant_a");
    duty_a.created_at = Utc.with_ymd_and_hms(2026, 5, 6, 12, 0, 0).unwrap();
    let mut duty_b = DutyRecord::new(tenant_b, worker_b, "0 * * * * *", "ping", "tenant_b");
    duty_b.created_at = Utc.with_ymd_and_hms(2026, 5, 6, 12, 0, 0).unwrap();
    duties.insert(&duty_a).await.unwrap();
    duties.insert(&duty_b).await.unwrap();

    let tick_at = Utc.with_ymd_and_hms(2026, 5, 6, 12, 1, 30).unwrap();
    let report = scheduler.tick(tick_at).await.unwrap();

    assert_eq!(report.evaluated, 2, "both tenants' duties evaluated");
    assert_eq!(report.fired.len(), 2, "both duties fired");
    assert!(report.fired.contains(&duty_a.id));
    assert!(report.fired.contains(&duty_b.id));

    // Per-duty attribution: each duty owns exactly one session.
    let attr_a = duties
        .sessions_for_duty(tenant_a, &duty_a.id)
        .await
        .unwrap();
    let attr_b = duties
        .sessions_for_duty(tenant_b, &duty_b.id)
        .await
        .unwrap();
    assert_eq!(attr_a.len(), 1, "duty_a owns exactly one fire row");
    assert_eq!(attr_b.len(), 1, "duty_b owns exactly one fire row");
    let session_a = attr_a[0].0.clone();
    let session_b = attr_b[0].0.clone();
    assert_ne!(
        session_a, session_b,
        "the two duties produced distinct session ids"
    );

    // Tenant-scoped session list isolation: tenant_a must see exactly
    // session_a; tenant_b must see exactly session_b; neither sees
    // the other.
    let sessions_a = orch.list_active_sessions(tenant_a).await.unwrap();
    let sessions_b = orch.list_active_sessions(tenant_b).await.unwrap();
    assert_eq!(
        sessions_a.len(),
        1,
        "tenant_a session list contains exactly one row"
    );
    assert_eq!(
        sessions_b.len(),
        1,
        "tenant_b session list contains exactly one row"
    );
    assert_eq!(
        sessions_a[0].id, session_a,
        "tenant_a sees its own session by id"
    );
    assert_eq!(
        sessions_b[0].id, session_b,
        "tenant_b sees its own session by id"
    );

    // Direct cross-query: looking up tenant_a's session id on
    // tenant_b's tenant must miss (the WHERE tenant_id = ? bind).
    let leak_a_into_b = orch.find_session(tenant_b, &session_a).await.unwrap();
    let leak_b_into_a = orch.find_session(tenant_a, &session_b).await.unwrap();
    assert!(
        leak_a_into_b.is_none(),
        "tenant_a's session must not be visible to tenant_b: {leak_a_into_b:?}"
    );
    assert!(
        leak_b_into_a.is_none(),
        "tenant_b's session must not be visible to tenant_a: {leak_b_into_a:?}"
    );

    // Participants: each session contains only its own worker.
    assert!(
        sessions_a[0].participants.contains(&worker_a),
        "session_a contains worker_a"
    );
    assert!(
        !sessions_a[0].participants.contains(&worker_b),
        "session_a must NOT contain worker_b"
    );
    assert!(
        sessions_b[0].participants.contains(&worker_b),
        "session_b contains worker_b"
    );
    assert!(
        !sessions_b[0].participants.contains(&worker_a),
        "session_b must NOT contain worker_a"
    );
}

// ----------------------------------------------------------------
// FINDINGS-ONLY regression hooks — `#[ignore]`'d because the wire
// they assert does not exist yet. See module doc for the trace.
// A follow-up brief (`fix-duty-scheduler-invokes-agent-runner` or
// equivalent) is expected to:
//   1. add an `AgentRunner` handle to `DutyScheduler`
//   2. invoke `AgentRunner::run_turn` from `tick` once a session
//      is spawned
//   3. write an audit row (`AuditKind::WorkerDutyFired` or reuse of
//      `CronJobFired`) for the (tenant, worker, duty) tuple.
// When that brief lands, the implementer flips off `#[ignore]` and
// replaces each `panic!` body with the real assertion.
// ----------------------------------------------------------------

#[tokio::test]
#[ignore = "FINDINGS-ONLY: DutyScheduler::tick does not invoke AgentRunner::run_turn — wire missing on origin/main; fix-brief pending"]
async fn duty_tick_invokes_agent_runner_findings_only() {
    // The desired assertion (sketch — does not compile against
    // current main, because DutyScheduler holds no AgentRunner):
    //
    //   let probe = Arc::new(RecordingAgentRunner::default());
    //   let scheduler = DutyScheduler::with_agent_runner(
    //       duties, orch, gate, probe.clone());
    //   scheduler.tick(tick_at).await.unwrap();
    //   let calls = probe.calls();
    //   assert_eq!(calls.len(), 1);
    //   assert_eq!(calls[0].tenant_id, tenant);
    //   assert_eq!(calls[0].worker_id, worker);
    //   assert_eq!(calls[0].entry_action, duty.entry_action);
    //
    panic!(
        "FINDINGS-ONLY: argo_orchestrator::DutyScheduler::tick spawns \
         a Session row but does NOT invoke AgentRunner::run_turn. \
         The struct holds no AgentRunner handle. See module doc \
         for the production trace and the follow-up fix-brief."
    );
}

#[tokio::test]
#[ignore = "FINDINGS-ONLY: DutyScheduler::tick writes no audit row; AuditKind::WorkerDutyFired does not exist — fix-brief pending"]
async fn duty_tick_emits_worker_duty_fired_audit_findings_only() {
    // The desired assertion (sketch — does not compile against
    // current main, because AuditKind::WorkerDutyFired does not
    // exist and DutyScheduler does not own an AuditSink):
    //
    //   let sink = AuditSink::in_memory();
    //   let scheduler = DutyScheduler::with_audit_sink(
    //       duties, orch, gate, sink.clone());
    //   scheduler.tick(tick_at).await.unwrap();
    //   let rows = sink.rows_of_kind(AuditKind::WorkerDutyFired).await;
    //   assert_eq!(rows.len(), 1);
    //   assert_eq!(rows[0].tenant_id, Some(tenant));
    //   assert_eq!(rows[0].payload["worker_id"], worker.to_string());
    //   assert_eq!(rows[0].payload["duty_id"], duty.id);
    //
    panic!(
        "FINDINGS-ONLY: argo_orchestrator::DutyScheduler::tick writes \
         NO audit row, and argo_audit::AuditKind has no WorkerDutyFired \
         variant (closest is CronJobFired, which belongs to the \
         not-wired argo-cron path). See module doc + fix-brief."
    );
}
