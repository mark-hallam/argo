use super::*;
use argo_types::{ArgoError, ArgoResult, TenantId};
use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use parking_lot::Mutex;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

// ── expression parsing ───────────────────────────────────────

#[test]
fn parses_wildcard_every_field() {
    let e = CronExpr::parse("* * * * *").unwrap();
    let now = Utc.with_ymd_and_hms(2026, 4, 30, 12, 34, 0).unwrap();
    assert!(e.matches(now));
}

#[test]
fn parses_literal_minute_and_hour() {
    let e = CronExpr::parse("0 8 * * *").unwrap();
    let on = Utc.with_ymd_and_hms(2026, 4, 30, 8, 0, 0).unwrap();
    let off = Utc.with_ymd_and_hms(2026, 4, 30, 8, 1, 0).unwrap();
    assert!(e.matches(on));
    assert!(!e.matches(off));
}

#[test]
fn parses_range_for_weekdays() {
    let e = CronExpr::parse("0 8 * * 1-5").unwrap();
    // 2026-04-30 is a Thursday → posix dow 4 → in 1..=5.
    let thu = Utc.with_ymd_and_hms(2026, 4, 30, 8, 0, 0).unwrap();
    assert!(e.matches(thu));
    // 2026-05-02 is a Saturday → posix dow 6 → out.
    let sat = Utc.with_ymd_and_hms(2026, 5, 2, 8, 0, 0).unwrap();
    assert!(!e.matches(sat));
}

#[test]
fn parses_comma_list() {
    let e = CronExpr::parse("0,30 * * * *").unwrap();
    let zero = Utc.with_ymd_and_hms(2026, 4, 30, 12, 0, 0).unwrap();
    let thirty = Utc.with_ymd_and_hms(2026, 4, 30, 12, 30, 0).unwrap();
    let fifteen = Utc.with_ymd_and_hms(2026, 4, 30, 12, 15, 0).unwrap();
    assert!(e.matches(zero));
    assert!(e.matches(thirty));
    assert!(!e.matches(fifteen));
}

#[test]
fn rejects_wrong_arity() {
    let err = CronExpr::parse("* * * *").unwrap_err();
    assert!(matches!(err, CronParseError::WrongArity(4)));
}

#[test]
fn rejects_out_of_range_minute() {
    let err = CronExpr::parse("60 * * * *").unwrap_err();
    assert!(matches!(
        err,
        CronParseError::InvalidField {
            field: "minute",
            ..
        }
    ));
}

#[test]
fn rejects_inverted_range() {
    let err = CronExpr::parse("* * 5-2 * *").unwrap_err();
    assert!(matches!(
        err,
        CronParseError::BadRange {
            field: "day_of_month",
            ..
        }
    ));
}

#[test]
fn dom_dow_or_semantics() {
    // "every Sunday OR the 1st of any month at 9am"
    let e = CronExpr::parse("0 9 1 * 0").unwrap();
    let first = Utc.with_ymd_and_hms(2026, 5, 1, 9, 0, 0).unwrap(); // Friday the 1st → dom matches
    let sunday = Utc.with_ymd_and_hms(2026, 5, 3, 9, 0, 0).unwrap(); // Sunday the 3rd → dow matches
    let plain = Utc.with_ymd_and_hms(2026, 5, 4, 9, 0, 0).unwrap(); // Monday the 4th → neither
    assert!(e.matches(first));
    assert!(e.matches(sunday));
    assert!(!e.matches(plain));
}

// ── jobs ─────────────────────────────────────────────────────

fn job_at_eight() -> CronJob {
    CronJob::new(
        TenantId::new(),
        "morning",
        CronExpr::parse("0 8 * * *").unwrap(),
        "Daily summary",
        CronTarget {
            platform: "telegram".into(),
            chat: "x".into(),
        },
    )
}

#[test]
fn disabled_job_is_never_due() {
    let mut j = job_at_eight();
    j.enabled = false;
    let now = Utc.with_ymd_and_hms(2026, 4, 30, 8, 0, 0).unwrap();
    assert!(!j.is_due(now));
}

#[test]
fn already_fired_this_minute_is_not_due() {
    let mut j = job_at_eight();
    let now = Utc.with_ymd_and_hms(2026, 4, 30, 8, 0, 30).unwrap();
    j.last_fired_at = Some(now - chrono::Duration::seconds(15));
    assert!(!j.is_due(now));
}

#[test]
fn fired_last_minute_can_fire_this_minute() {
    let mut j = CronJob::new(
        TenantId::new(),
        "every-minute",
        CronExpr::parse("* * * * *").unwrap(),
        "tick",
        CronTarget {
            platform: "x".into(),
            chat: "y".into(),
        },
    );
    let prev = Utc.with_ymd_and_hms(2026, 4, 30, 8, 0, 0).unwrap();
    let now = Utc.with_ymd_and_hms(2026, 4, 30, 8, 1, 0).unwrap();
    j.last_fired_at = Some(prev);
    assert!(j.is_due(now));
}

// ── store ────────────────────────────────────────────────────

#[tokio::test]
async fn store_round_trips_save_list_delete() {
    let store = InMemoryJobStore::new();
    let j = job_at_eight();
    let id = j.id;
    store.save(&j).await.unwrap();
    let listed = store.list_all().await.unwrap();
    assert_eq!(listed.len(), 1);
    store.delete(id).await.unwrap();
    assert!(store.list_all().await.unwrap().is_empty());
}

#[tokio::test]
async fn save_overwrites_by_tenant_and_name() {
    let store = InMemoryJobStore::new();
    let mut j = job_at_eight();
    store.save(&j).await.unwrap();
    j.prompt = "Updated".into();
    store.save(&j).await.unwrap();
    let all = store.list_all().await.unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].prompt, "Updated");
}

#[tokio::test]
async fn record_fire_updates_last_fired_at() {
    let store = InMemoryJobStore::new();
    let j = job_at_eight();
    let id = j.id;
    store.save(&j).await.unwrap();
    let when = Utc.with_ymd_and_hms(2026, 4, 30, 8, 0, 0).unwrap();
    store.record_fire(id, when).await.unwrap();
    let fetched = store.list_all().await.unwrap().pop().unwrap();
    assert_eq!(fetched.last_fired_at, Some(when));
}

// ── scheduler ────────────────────────────────────────────────

#[derive(Default)]
struct CountingRunner {
    fired: Mutex<Vec<uuid::Uuid>>,
    fail_next: AtomicBool,
}

#[async_trait]
impl JobRunner for CountingRunner {
    async fn run(&self, job: &CronJob) -> ArgoResult<()> {
        if self.fail_next.swap(false, Ordering::SeqCst) {
            return Err(ArgoError::Provider("synthetic failure".into()));
        }
        self.fired.lock().push(job.id);
        Ok(())
    }
}

#[tokio::test]
async fn tick_fires_due_jobs_and_records_them() {
    let store = InMemoryJobStore::new();
    let j = job_at_eight();
    let id = j.id;
    store.save(&j).await.unwrap();
    let runner = CountingRunner::default();
    let now = Utc.with_ymd_and_hms(2026, 4, 30, 8, 0, 0).unwrap();
    let report = Scheduler.tick(&store, &runner, now).await.unwrap();
    assert_eq!(report.fired, vec![id]);
    assert_eq!(report.errors, Vec::<uuid::Uuid>::new());
    let after = store.list_all().await.unwrap();
    assert_eq!(after[0].last_fired_at, Some(now));
}

#[tokio::test]
async fn tick_skips_not_yet_due_jobs() {
    let store = InMemoryJobStore::new();
    let j = job_at_eight();
    store.save(&j).await.unwrap();
    let runner = CountingRunner::default();
    let now = Utc.with_ymd_and_hms(2026, 4, 30, 9, 0, 0).unwrap();
    let report = Scheduler.tick(&store, &runner, now).await.unwrap();
    assert!(report.attempted.is_empty());
    assert!(runner.fired.lock().is_empty());
}

#[tokio::test]
async fn runner_error_does_not_record_fire() {
    let store = InMemoryJobStore::new();
    let j = job_at_eight();
    let id = j.id;
    store.save(&j).await.unwrap();
    let runner = Arc::new(CountingRunner::default());
    runner.fail_next.store(true, Ordering::SeqCst);
    let now = Utc.with_ymd_and_hms(2026, 4, 30, 8, 0, 0).unwrap();
    let report = Scheduler.tick(&store, &*runner, now).await.unwrap();
    assert_eq!(report.errors, vec![id]);
    assert!(report.fired.is_empty());
    let after = store.list_all().await.unwrap();
    assert_eq!(after[0].last_fired_at, None);
}

#[tokio::test]
async fn tick_does_not_double_fire_within_same_minute() {
    let store = InMemoryJobStore::new();
    let j = job_at_eight();
    store.save(&j).await.unwrap();
    let runner = CountingRunner::default();
    let now = Utc.with_ymd_and_hms(2026, 4, 30, 8, 0, 5).unwrap();
    let r1 = Scheduler.tick(&store, &runner, now).await.unwrap();
    let later = now + chrono::Duration::seconds(30);
    let r2 = Scheduler.tick(&store, &runner, later).await.unwrap();
    assert_eq!(r1.fired.len(), 1);
    assert!(r2.fired.is_empty());
}
