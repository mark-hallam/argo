//! # Mission Claim 2 LIVE — cron fires per-tenant (substrate proof)
//!
//! Pins the operator-locked LIVE criteria from `docs/skycot/MISSION.md`
//! (added round 15 with operator sign-off):
//!
//! > Claim 2 LIVE = a hired worker's duty cron fires AND invokes its
//! > agent runner AND writes a WorkerDutyFired audit row within 60s.
//!
//! This file pins the **scheduler half** of that criteria — the
//! `(JobStore, Scheduler, JobRunner)` triple that decides which jobs
//! are due, fans out across tenants, and dispatches without double-
//! firing inside the same minute.
//!
//! ## Why this complements the in-crate unit tests
//!
//! `argo-cron/src/tests.rs` already covers single-tenant happy-path
//! firing. What it does NOT cover is the cross-tenant integration
//! scenario where:
//!
//! - Multiple tenants each own jobs with overlapping schedules.
//! - Some jobs are enabled, some disabled.
//! - A single tick fans out across all of them in one call.
//! - The dedupe-within-minute and runner-error-doesn't-record paths
//!   compose with multi-tenant fan-out (not just isolated single-job
//!   scenarios).
//!
//! ## What this test does NOT prove yet
//!
//! - **Agent runner invocation**: the brief
//!   `fix-duty-scheduler-invokes-agent-runner` (CLAIMED, fix-by
//!   2026-05-30) is the path from `JobRunner::run` → `argo-agent-runner`
//!   `Agent::turn`. Until that lands, this test uses a recording
//!   mock `JobRunner` to pin the *scheduler* contract.
//! - **WorkerDutyFired audit row**: covered by the existing in-crate
//!   `tick_fires_due_jobs_and_records_them` test plus the
//!   `Scheduler::tick_with_audit` path. End-to-end audit-row shape
//!   verification belongs in a follow-up brief once the agent-runner
//!   wire lands.
//!
//! ## Authored
//!
//! Evaluator round 26, 2026-05-23.

use std::sync::Arc;

use argo_cron::{
    CronExpr, CronJob, CronTarget, InMemoryJobStore, JobRunner, JobStore, Scheduler,
};
use argo_types::{ArgoError, ArgoResult, TenantId};
use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use parking_lot::Mutex;
use uuid::Uuid;

/// Mock JobRunner that records every job id it sees and can be
/// configured to fail a subset (to pin the runner-error-doesn't-record
/// invariant under multi-tenant fan-out).
#[derive(Debug, Default, Clone)]
struct RecordingRunner {
    seen: Arc<Mutex<Vec<Uuid>>>,
    fail_ids: Arc<Mutex<Vec<Uuid>>>,
}

impl RecordingRunner {
    fn new() -> Self {
        Self::default()
    }

    fn fail_id(&self, id: Uuid) {
        self.fail_ids.lock().push(id);
    }

    fn seen(&self) -> Vec<Uuid> {
        self.seen.lock().clone()
    }
}

#[async_trait]
impl JobRunner for RecordingRunner {
    async fn run(&self, job: &CronJob) -> ArgoResult<()> {
        self.seen.lock().push(job.id);
        if self.fail_ids.lock().contains(&job.id) {
            return Err(ArgoError::Provider(format!(
                "claim-2 LIVE substrate: forced failure for job {}",
                job.id
            )));
        }
        Ok(())
    }
}

fn cron_every_minute() -> CronExpr {
    CronExpr::parse("* * * * *").expect("`* * * * *` parses")
}

fn cron_at_minute(min: u8) -> CronExpr {
    CronExpr::parse(&format!("{min} * * * *")).expect("specific-minute cron parses")
}

fn target(platform: &str, chat: &str) -> CronTarget {
    CronTarget {
        platform: platform.into(),
        chat: chat.into(),
    }
}

#[tokio::test]
async fn claim_2_strengthener_multi_tenant_cron_fans_out_in_one_tick() {
    // ── Phase 1: fixture — 3 tenants, mixed enabled/disabled/schedule shapes.
    let store = InMemoryJobStore::new();
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    let tenant_c = TenantId::new();

    // tenant_a: 2 enabled jobs that fire every minute.
    let a_job1 = CronJob::new(
        tenant_a,
        "mariah-morning-kickoff",
        cron_every_minute(),
        "Run morning kickoff for marketplace listings",
        target("slack", "C-MARIAH"),
    );
    let a_job2 = CronJob::new(
        tenant_a,
        "mariah-inventory-sync",
        cron_every_minute(),
        "Sync inventory CSV from drive",
        target("email", "mariah@example.com"),
    );

    // tenant_b: 1 enabled + 1 disabled job.
    let b_job_enabled = CronJob::new(
        tenant_b,
        "priya-candidate-digest",
        cron_every_minute(),
        "Daily candidate-digest summary",
        target("slack", "C-PRIYA"),
    );
    let mut b_job_disabled = CronJob::new(
        tenant_b,
        "priya-archive-old-resumes",
        cron_every_minute(),
        "Archive resumes older than 90d",
        target("email", "priya@example.com"),
    );
    b_job_disabled.enabled = false;

    // tenant_c: 1 job at minute 33 — NOT due at minute 0 — must be skipped.
    let c_job_not_due = CronJob::new(
        tenant_c,
        "jamal-ticket-rollup",
        cron_at_minute(33),
        "Hourly ticket-rollup at minute 33",
        target("slack", "C-JAMAL"),
    );

    for j in [
        &a_job1,
        &a_job2,
        &b_job_enabled,
        &b_job_disabled,
        &c_job_not_due,
    ] {
        store.save(j).await.expect("save fixture");
    }

    // Tick at 2026-05-23 09:00:00 UTC — `*` matches minute 0 + every
    // hour but `33 * * * *` does NOT.
    let now = Utc.with_ymd_and_hms(2026, 5, 23, 9, 0, 0).unwrap();
    let runner = RecordingRunner::new();
    let report = Scheduler
        .tick(&store, &runner, now)
        .await
        .expect("first tick");

    // ── Phase 2: assert fan-out across tenants — both of tenant_a's
    // jobs fired, tenant_b's enabled job fired, tenant_b's disabled
    // job was skipped, tenant_c's minute-33 job was skipped.
    assert_eq!(
        report.fired.len(),
        3,
        "claim-2 LIVE substrate: expected 3 fires (2 tenant_a + 1 tenant_b enabled); got {:?}",
        report.fired
    );
    assert!(
        report.errors.is_empty(),
        "claim-2 LIVE substrate: expected zero errors on happy path; got {:?}",
        report.errors
    );
    let seen = runner.seen();
    assert!(
        seen.contains(&a_job1.id),
        "claim-2 LIVE substrate: tenant_a job1 must fire"
    );
    assert!(
        seen.contains(&a_job2.id),
        "claim-2 LIVE substrate: tenant_a job2 must fire"
    );
    assert!(
        seen.contains(&b_job_enabled.id),
        "claim-2 LIVE substrate: tenant_b enabled job must fire"
    );
    assert!(
        !seen.contains(&b_job_disabled.id),
        "claim-2 LIVE substrate: tenant_b DISABLED job must NOT fire (got seen={seen:?})"
    );
    assert!(
        !seen.contains(&c_job_not_due.id),
        "claim-2 LIVE substrate: tenant_c minute-33 job must NOT fire at minute 0"
    );

    // ── Phase 3: same-minute re-tick — last_fired_at suppresses
    // duplicate fires. This is the "scheduler doesn't double-tick"
    // invariant that protects against the daemon waking twice.
    let runner_v2 = RecordingRunner::new();
    let report_v2 = Scheduler
        .tick(&store, &runner_v2, now)
        .await
        .expect("re-tick same minute");
    assert!(
        report_v2.fired.is_empty(),
        "claim-2 LIVE substrate: re-tick within same minute must NOT re-fire any job; got fired={:?}",
        report_v2.fired
    );
    assert!(
        runner_v2.seen().is_empty(),
        "claim-2 LIVE substrate: runner must not be invoked on same-minute re-tick"
    );

    // ── Phase 4: next-minute tick — same `*` schedule matches again,
    // jobs fire AGAIN. Pins the cursor-advance contract.
    let now_next = Utc.with_ymd_and_hms(2026, 5, 23, 9, 1, 0).unwrap();
    let runner_v3 = RecordingRunner::new();
    let report_v3 = Scheduler
        .tick(&store, &runner_v3, now_next)
        .await
        .expect("next-minute tick");
    assert_eq!(
        report_v3.fired.len(),
        3,
        "claim-2 LIVE substrate: next-minute tick must fire 3 jobs again; got {:?}",
        report_v3.fired
    );

    // ── Phase 5: tenant_c's minute-33 job fires when (and only when)
    // we tick at minute 33.
    let now_33 = Utc.with_ymd_and_hms(2026, 5, 23, 9, 33, 0).unwrap();
    let runner_v4 = RecordingRunner::new();
    let report_v4 = Scheduler
        .tick(&store, &runner_v4, now_33)
        .await
        .expect("minute-33 tick");
    assert!(
        report_v4.fired.contains(&c_job_not_due.id),
        "claim-2 LIVE substrate: tenant_c minute-33 job MUST fire at minute 33; got fired={:?}",
        report_v4.fired
    );
}

#[tokio::test]
async fn claim_2_strengthener_runner_error_does_not_advance_cursor_across_tenants() {
    // Defensive — when a single tenant's job runner returns an error,
    // the scheduler must report it under `errors`, NOT advance that
    // job's last_fired_at cursor (so the next tick retries), AND not
    // disturb sibling tenants' successful fires.
    let store = InMemoryJobStore::new();
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();

    let a_will_succeed = CronJob::new(
        tenant_a,
        "marketplace-runner-ok",
        cron_every_minute(),
        "OK job",
        target("slack", "C-A"),
    );
    let b_will_fail = CronJob::new(
        tenant_b,
        "recruiter-runner-fail",
        cron_every_minute(),
        "Failing job",
        target("slack", "C-B"),
    );
    store.save(&a_will_succeed).await.expect("save A");
    store.save(&b_will_fail).await.expect("save B");

    let runner = RecordingRunner::new();
    runner.fail_id(b_will_fail.id);

    let now = Utc.with_ymd_and_hms(2026, 5, 23, 9, 0, 0).unwrap();
    let report = Scheduler.tick(&store, &runner, now).await.expect("tick");

    assert_eq!(
        report.fired,
        vec![a_will_succeed.id],
        "claim-2 LIVE substrate: only tenant_a's OK job appears in fired list"
    );
    assert_eq!(
        report.errors,
        vec![b_will_fail.id],
        "claim-2 LIVE substrate: tenant_b's failing job appears in errors list"
    );

    // ── Sibling tenant's success cursor advanced; failing tenant's cursor
    // did NOT — re-fetch and check last_fired_at.
    let after = store.list_all().await.expect("re-list");
    let a_row = after
        .iter()
        .find(|j| j.id == a_will_succeed.id)
        .expect("A still present");
    let b_row = after
        .iter()
        .find(|j| j.id == b_will_fail.id)
        .expect("B still present");
    assert_eq!(
        a_row.last_fired_at,
        Some(now),
        "claim-2 LIVE substrate: success cursor must advance"
    );
    assert!(
        b_row.last_fired_at.is_none(),
        "claim-2 LIVE substrate: error cursor must NOT advance (so next tick retries); got {:?}",
        b_row.last_fired_at
    );
}

#[tokio::test]
async fn claim_2_strengthener_handles_many_tenants_in_one_tick() {
    // Stress — 25 tenants each owning 1 enabled cron job; a single
    // tick must fire all 25, in deterministic order (the store sorts
    // by tenant_id then name), and the runner must see exactly 25
    // invocations.
    let store = InMemoryJobStore::new();
    let mut expected_ids = Vec::with_capacity(25);
    for i in 0..25 {
        let tenant = TenantId::new();
        let job = CronJob::new(
            tenant,
            format!("worker-{i:02}-kickoff"),
            cron_every_minute(),
            format!("Kickoff prompt for worker-{i:02}"),
            target("slack", &format!("C-{i:02}")),
        );
        expected_ids.push(job.id);
        store.save(&job).await.expect("save");
    }

    let now = Utc.with_ymd_and_hms(2026, 5, 23, 9, 0, 0).unwrap();
    let runner = RecordingRunner::new();
    let report = Scheduler.tick(&store, &runner, now).await.expect("tick");

    assert_eq!(
        report.fired.len(),
        25,
        "claim-2 LIVE substrate: 25-tenant fan-out must fire all 25 jobs in one tick; got {}",
        report.fired.len()
    );
    assert!(
        report.errors.is_empty(),
        "claim-2 LIVE substrate: 25-tenant fan-out must have zero errors"
    );
    let mut seen = runner.seen();
    seen.sort();
    expected_ids.sort();
    assert_eq!(
        seen, expected_ids,
        "claim-2 LIVE substrate: runner must see every tenant's job exactly once"
    );
}
