//! Integration roll-up for stream
//! `web-audit-kinds-for-journey-events`.
//!
//! Verifies the 6 new domain audit kinds round-trip and emit through
//! the paths that ship in this brief:
//!
//! 1. `AuditKind::all()` lists `WorkerHired`, `VerticalRunRequested`,
//!    `VerticalRunCompleted`, `CronJobCreated`, `CronJobDeleted`,
//!    `CronJobFired`.
//! 2. Each label round-trips via `label()` / `from_label()`.
//! 3. `Scheduler::tick_with_audit` writes a `cron_job_fired` row to
//!    the supplied [`argo_audit::AuditSink`] per successful dispatch.
//!
//! Full HTTP-roundtrip coverage for the gateway emit points (hire,
//! vertical-run, cron POST/DELETE) is exercised by the per-route
//! integration tests in this directory; this file documents the
//! end-to-end shape so a regression deleting a variant or skipping
//! the audit write surfaces a clean failure.

use argo_audit::{AuditKind, AuditSink};
use argo_cron::{
    CronExpr, CronJob, CronTarget, InMemoryJobStore, JobRunner, JobStore, Scheduler,
};
use argo_store::SessionStore;
use argo_types::{ArgoResult, TenantId};
use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use std::sync::Arc;

struct NoopRunner;

#[async_trait]
impl JobRunner for NoopRunner {
    async fn run(&self, _job: &CronJob) -> ArgoResult<()> {
        Ok(())
    }
}

#[test]
fn journey_kinds_listed_and_round_trip() {
    let kinds = [
        ("worker.hired", AuditKind::WorkerHired),
        ("vertical.run.requested", AuditKind::VerticalRunRequested),
        ("vertical.run.completed", AuditKind::VerticalRunCompleted),
        ("cron_job_created", AuditKind::CronJobCreated),
        ("cron_job_deleted", AuditKind::CronJobDeleted),
        ("cron_job_fired", AuditKind::CronJobFired),
    ];
    let all = AuditKind::all();
    for (label, kind) in kinds {
        assert_eq!(kind.label(), label);
        assert_eq!(AuditKind::from_label(label), Some(kind));
        assert!(
            all.contains(&kind),
            "AuditKind::all() must include {label}"
        );
    }
}

#[tokio::test]
async fn scheduler_tick_with_audit_emits_cron_job_fired() {
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    let tenant_id = TenantId::new();
    let sink = AuditSink::new_with_tenant(Arc::clone(&store), tenant_id.to_string());

    let job_store = InMemoryJobStore::new();
    // `* * * * *` is due every minute — guaranteed to fire at any tick.
    let schedule = CronExpr::parse("* * * * *").expect("parse schedule");
    let job = CronJob::new(
        tenant_id,
        "fire-test",
        schedule,
        "test prompt",
        CronTarget {
            platform: "telegram".into(),
            chat: "demo".into(),
        },
    );
    job_store.save(&job).await.unwrap();

    let runner = NoopRunner;
    let now = Utc.with_ymd_and_hms(2026, 5, 21, 9, 0, 0).unwrap();
    let report = Scheduler
        .tick_with_audit(&job_store, &runner, now, Some(&sink))
        .await
        .expect("tick_with_audit");
    assert_eq!(report.fired.len(), 1, "expected one fired job");

    sink.flush_pending().await.unwrap();

    let row: (String, String) = sqlx::query_as(
        "SELECT kind, payload_json FROM audit_record \
         WHERE tenant_id = ? AND kind = ? LIMIT 1",
    )
    .bind(tenant_id.to_string())
    .bind(AuditKind::CronJobFired.label())
    .fetch_one(store.pool())
    .await
    .expect("cron_job_fired row missing from audit chain");
    let payload: serde_json::Value = serde_json::from_str(&row.1).expect("payload parses");
    assert_eq!(payload["job_name"], "fire-test");
    assert_eq!(payload["schedule"], "* * * * *");
    assert_eq!(payload["dispatch_target"]["platform"], "telegram");
    assert_eq!(payload["dispatch_target"]["chat"], "demo");
}

#[tokio::test]
async fn scheduler_tick_without_audit_is_unchanged() {
    // Back-compat: callers that pass `None` (or use the existing
    // `tick` method) keep their current behaviour and never emit.
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    let tenant_id = TenantId::new();

    let job_store = InMemoryJobStore::new();
    let schedule = CronExpr::parse("* * * * *").unwrap();
    let job = CronJob::new(
        tenant_id,
        "no-audit",
        schedule,
        "p",
        CronTarget {
            platform: "telegram".into(),
            chat: "c".into(),
        },
    );
    job_store.save(&job).await.unwrap();

    let runner = NoopRunner;
    let now = Utc.with_ymd_and_hms(2026, 5, 21, 9, 0, 0).unwrap();
    let report = Scheduler
        .tick(&job_store, &runner, now)
        .await
        .expect("tick");
    assert_eq!(report.fired.len(), 1);

    let count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM audit_record WHERE tenant_id = ? AND kind = ?",
    )
    .bind(tenant_id.to_string())
    .bind(AuditKind::CronJobFired.label())
    .fetch_one(store.pool())
    .await
    .unwrap();
    assert_eq!(
        count.0, 0,
        "tick without audit_sink must not emit cron_job_fired"
    );
}
