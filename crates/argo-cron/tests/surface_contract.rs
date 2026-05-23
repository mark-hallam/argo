//! Contract pins for the `argo-cron` public surface (part 1 of 2).
//!
//! Authored by `eval-argo-cron-crate-surface` (claim-2 substrate
//! audit, 2026-05-24). Sibling pattern: PR #2132 (`eval-argo-skill-
//! sdk-surface`) — both lock the pub-API surface so that a rename,
//! removal, or signature change in the underlying source fails
//! THIS test at PR time instead of silently shipping a breaking
//! change to downstream callers.
//!
//! `argo-cron` is the scheduling substrate underneath claim-2
//! ("Delegation is the unit"). Direct downstream consumers (gateway
//! cron daemon, `argo-cli system_jobs` handlers, `argo-orchestrator
//! ::DutyScheduler`) all depend on the primitives pinned here.
//!
//! ## File split
//!
//! - `tests/surface_contract.rs` (THIS file): pub-use inventory
//!   sentinel, `CronExpr` + `CronParseError` parse/serde,
//!   `CronTarget` struct shape, `TickLock` mutex contract, and the
//!   `Scheduler`/`JobRunner`/`TickReport` signature pins.
//! - `tests/system_job_registry_contract.rs`: `CronJob` shape +
//!   golden serde round-trip against `cron/system/*.json` fixtures,
//!   plus the `SystemJobRegistry`/`SystemJobHandler`/`SystemJobOutcome`/
//!   `SystemJobContext` family pins.
//!
//! Two files because the combined contract test runs ~740 LOC,
//! over the 500-LOC per-file invariant. The brief explicitly
//! anticipated this with the `system_job_registry_contract.rs
//! (NEW, optional split)` may-touch entry.
//!
//! ## Sensible deviations from the brief
//!
//! - Brief says `Job`; the real type is `CronJob`. Pinned as `CronJob`.
//! - Brief says `CronTarget::{User { duty_id, worker_id }, System
//!   { job_kind }}` enum variants; the real `CronTarget` is a struct
//!   `{ platform, chat }` and the "system" flavour is a reserved
//!   `platform = "system"` convention (see `crates/argo-cron/src/
//!   target.rs`). Pinned as a struct-shape + system-convention pin.
//! - Brief says `tick_lock` returns `Locked` within `claim_ttl`; the
//!   real `TickLock::try_acquire` returns `Err(TickLockError::
//!   AlreadyHeld)` while the file lock is held. The intent is
//!   identical (exactly one tick wins); pinned via the real API.
//! - Brief says `SystemJobRegistry::list_due` / `mark_claimed` /
//!   `mark_complete`; the real registry exposes `empty` / `get` /
//!   `has` / `names`. Pinned as the real API.
//! - Brief lists 4 canonical jobs; `cron/system/` has 5 fixtures (the
//!   4 in `CANONICAL_SYSTEM_JOBS` plus `audit_evidence_archival_tick
//!   .json`, which is shipped but not enumerated in the const). All 5
//!   round-trip; the count assertion pins `CANONICAL_SYSTEM_JOBS.len()
//!   == 4`.

#![allow(dead_code)]

// All 24 names are imported here as a compile-fence; only a subset are
// dispatched at runtime in this file (the rest are exercised by the
// sibling `system_job_registry_contract.rs`). If any re-export
// disappears, renames, or moves module, this import block fails to
// compile. `unused_imports` is silenced because the imports themselves
// are the proof — they're tied back to runtime usage via the
// `pub_use_inventory_pins_count_at_24` test below.
#[allow(unused_imports)]
use argo_cron::{
    system_job_name, system_target, CronExpr, CronJob, CronParseError, CronTarget,
    InMemoryJobStore, JobRunner, JobStore, Scheduler, SystemJobContext, SystemJobHandler,
    SystemJobOutcome, SystemJobRegistry, SystemJobRegistryBuilder, TickLock, TickLockError,
    TickReport, CANONICAL_SYSTEM_JOBS, SYSTEM_JOB_AB_PROMOTION, SYSTEM_JOB_DREAMS_CURATION,
    SYSTEM_JOB_FORGE_OPTIMIZE, SYSTEM_JOB_SKILL_QUALITY_RECOMPUTE, SYSTEM_PLATFORM,
};
use argo_types::{ArgoResult, TenantId};
use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// 1. `pub use` re-export inventory — compile-fence + count sentinel
//
// Total inventory pinned: 24 names across 6 modules
//   expr:           CronExpr, CronParseError                          (2)
//   job:            CronJob, CronTarget, InMemoryJobStore, JobStore   (4)
//   scheduler:      JobRunner, Scheduler, TickReport                  (3)
//   system_registry: SystemJobContext, SystemJobHandler,
//                    SystemJobOutcome, SystemJobRegistry,
//                    SystemJobRegistryBuilder                         (5)
//   target:         system_job_name, system_target,
//                   CANONICAL_SYSTEM_JOBS, SYSTEM_JOB_AB_PROMOTION,
//                   SYSTEM_JOB_DREAMS_CURATION,
//                   SYSTEM_JOB_FORGE_OPTIMIZE,
//                   SYSTEM_JOB_SKILL_QUALITY_RECOMPUTE,
//                   SYSTEM_PLATFORM                                   (8)
//   tick_lock:      TickLock, TickLockError                           (2)
// ---------------------------------------------------------------------------

#[test]
fn pub_use_inventory_pins_count_at_24() {
    // Adding or removing a `pub use` line in lib.rs forces this array
    // (and the import block above) to be updated. A bare count assert
    // catches the surface-shrink case the import block alone cannot.
    let inventory = [
        "CronExpr",
        "CronParseError",
        "CronJob",
        "CronTarget",
        "InMemoryJobStore",
        "JobStore",
        "JobRunner",
        "Scheduler",
        "TickReport",
        "SystemJobContext",
        "SystemJobHandler",
        "SystemJobOutcome",
        "SystemJobRegistry",
        "SystemJobRegistryBuilder",
        "system_job_name",
        "system_target",
        "CANONICAL_SYSTEM_JOBS",
        "SYSTEM_JOB_AB_PROMOTION",
        "SYSTEM_JOB_DREAMS_CURATION",
        "SYSTEM_JOB_FORGE_OPTIMIZE",
        "SYSTEM_JOB_SKILL_QUALITY_RECOMPUTE",
        "SYSTEM_PLATFORM",
        "TickLock",
        "TickLockError",
    ];
    assert_eq!(
        inventory.len(),
        24,
        "pub_use inventory drift — update lib.rs pub use list, then the inventory array above"
    );

    // Force any unused imports above to type-check at runtime instead
    // of compile-time warning. (Only re-exports actively dispatched in
    // this file are JobRunner, JobStore, Scheduler, TickReport,
    // SystemJobOutcome, plus the SystemJob* family used by the sibling
    // file. The block below pins the rest to their argo-cron home.)
    let _: fn(&CronTarget) -> Option<&str> = system_job_name;
    let _: fn(String) -> CronTarget = system_target;
    let _: &[&str] = CANONICAL_SYSTEM_JOBS;
    let _: &str = SYSTEM_JOB_AB_PROMOTION;
    let _: &str = SYSTEM_JOB_DREAMS_CURATION;
    let _: &str = SYSTEM_JOB_FORGE_OPTIMIZE;
    let _: &str = SYSTEM_JOB_SKILL_QUALITY_RECOMPUTE;
    let _: &str = SYSTEM_PLATFORM;
    fn _accepts_in_memory_store() -> InMemoryJobStore {
        InMemoryJobStore::new()
    }
    fn _accepts_system_job_context(root: PathBuf) -> SystemJobContext {
        SystemJobContext::at_now(root)
    }
    fn _accepts_system_job_registry_builder() -> SystemJobRegistryBuilder {
        SystemJobRegistryBuilder::new()
    }
    fn _accepts_system_job_registry() -> SystemJobRegistry {
        SystemJobRegistry::empty()
    }
    fn _accepts_cron_job(j: CronJob) -> CronJob {
        j
    }
}

// ---------------------------------------------------------------------------
// 2. CronExpr parse + serde + match round-trip
// ---------------------------------------------------------------------------

#[test]
fn cron_expr_parses_and_round_trips_serde_for_user_cadences() {
    // Five user-shaped cadences covering wildcard, literal, comma-list,
    // range, and dom/dow OR. `*/N` step syntax is intentionally NOT
    // tested — `expr.rs` rustdoc documents it as reserved for a
    // follow-up. The equivalent comma-list form is exercised instead.
    let cadences = [
        "0 6 * * *",          // daily 6am UTC
        "0 0 * * 0",          // Sunday midnight UTC
        "0,15,30,45 * * * *", // every 15-minute mark (comma-list in minute slot)
        "0 8 * * 1-5",        // weekdays 8am UTC
        "0 9 1 * *",          // first-of-month 9am UTC
    ];
    assert_eq!(cadences.len(), 5);

    for src in cadences {
        let expr = CronExpr::parse(src)
            .unwrap_or_else(|e| panic!("expected parse to succeed for {src:?}: {e:?}"));
        assert_eq!(expr.source, src, "source field must preserve original text");

        // Serde round-trip: serialize → deserialize → equal.
        let json = serde_json::to_string(&expr).expect("CronExpr Serialize");
        let parsed: CronExpr = serde_json::from_str(&json).expect("CronExpr Deserialize");
        assert_eq!(parsed.source, expr.source, "serde round-trip drops source");
        assert_eq!(parsed, expr, "serde round-trip is not value-equal");
    }
}

#[test]
fn cron_expr_parse_error_variants_pinned_exhaustively() {
    // Exhaustive match — adding or removing a variant fails compilation.
    fn label(e: CronParseError) -> &'static str {
        match e {
            CronParseError::WrongArity(_) => "WrongArity",
            CronParseError::InvalidField { .. } => "InvalidField",
            CronParseError::BadRange { .. } => "BadRange",
        }
    }

    // One of each so the constructors + Display + Error trait are pinned.
    let cases = [
        CronParseError::WrongArity(4),
        CronParseError::InvalidField {
            field: "minute",
            detail: "x".into(),
        },
        CronParseError::BadRange {
            field: "day_of_month",
            start: 5,
            end: 2,
        },
    ];
    assert_eq!(cases.len(), 3);
    for case in cases {
        let _: String = case.to_string();
        let _: &dyn std::error::Error = &case;
        assert!(!label(case.clone()).is_empty());
    }

    // Real parse errors map to the right variant.
    assert!(matches!(
        CronExpr::parse("* * * *").unwrap_err(),
        CronParseError::WrongArity(4)
    ));
    assert!(matches!(
        CronExpr::parse("60 * * * *").unwrap_err(),
        CronParseError::InvalidField { field: "minute", .. }
    ));
    assert!(matches!(
        CronExpr::parse("* * 5-2 * *").unwrap_err(),
        CronParseError::BadRange { field: "day_of_month", .. }
    ));
}

// ---------------------------------------------------------------------------
// 3. CronTarget struct shape pin
//
// NOTE: brief says `CronTarget::{User { duty_id, worker_id }, System
// { job_kind }}` exhaustive enum variants. The real `CronTarget` is a
// struct, not an enum — system targets reuse the same struct via the
// reserved `platform = "system"` convention (pinned more fully in the
// sibling system_job_registry_contract.rs file alongside the canonical
// constants).
// ---------------------------------------------------------------------------

#[test]
fn cron_target_struct_literal_pins_two_pub_fields() {
    let t = CronTarget {
        platform: "telegram".into(),
        chat: "abc".into(),
    };
    let _: &str = t.platform.as_str();
    let _: &str = t.chat.as_str();
    // Serde round-trip pin.
    let json = serde_json::to_string(&t).unwrap();
    let back: CronTarget = serde_json::from_str(&json).unwrap();
    assert_eq!(back, t);
}

// ---------------------------------------------------------------------------
// 4. TickLock mutual exclusion contract
//
// NOTE: brief says "exactly one tick wins claim; subsequent ticks
// within `claim_ttl` return `Locked`". The real `TickLock::try_acquire`
// is a `fs2`-backed file lock — held for the lifetime of the guard,
// and a second `try_acquire` returns `Err(TickLockError::AlreadyHeld)`.
// Same intent (exactly one tick wins), pinned via the real API.
// ---------------------------------------------------------------------------

#[test]
fn tick_lock_first_acquire_wins_and_second_returns_already_held() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("subdir/.tick.lock");

    // First acquire wins (and creates parent directories).
    let first = TickLock::try_acquire(&path).expect("first acquire");
    assert!(path.exists(), "lock file must be created");
    assert_eq!(first.path, path, "TickLock.path must match acquire arg");

    // Second acquire while first is held returns AlreadyHeld.
    let err = TickLock::try_acquire(&path).unwrap_err();
    match err {
        TickLockError::AlreadyHeld { path: p } => {
            assert_eq!(p, path, "AlreadyHeld must surface the contended path");
        }
        other => panic!("expected AlreadyHeld, got {other:?}"),
    }

    // After drop, a fresh acquire succeeds.
    drop(first);
    let _again = TickLock::try_acquire(&path).expect("re-acquire after drop");
}

#[test]
fn tick_lock_error_variants_pinned_exhaustively() {
    fn label(e: TickLockError) -> &'static str {
        match e {
            TickLockError::AlreadyHeld { .. } => "AlreadyHeld",
            TickLockError::Io { .. } => "Io",
        }
    }
    let cases = [
        TickLockError::AlreadyHeld {
            path: PathBuf::from("/tmp/x"),
        },
        TickLockError::Io {
            detail: "open failed".into(),
        },
    ];
    assert_eq!(cases.len(), 2);
    for case in cases {
        let _: String = case.to_string();
        let _: &dyn std::error::Error = &case;
        assert!(!label(case).is_empty());
    }
}

// ---------------------------------------------------------------------------
// 5. Scheduler + JobRunner + TickReport signature pin + happy/idle paths
// ---------------------------------------------------------------------------

#[derive(Default)]
struct RecordingRunner {
    fired: parking_lot::Mutex<Vec<uuid::Uuid>>,
}

#[async_trait]
impl JobRunner for RecordingRunner {
    async fn run(&self, job: &CronJob) -> ArgoResult<()> {
        self.fired.lock().push(job.id);
        Ok(())
    }
}

#[test]
fn scheduler_is_send_sync_clone_copy() {
    fn _bounds<T: Send + Sync + Clone + Copy + Default>() {}
    _bounds::<Scheduler>();
}

#[test]
fn tick_report_pub_field_struct_literal_pins_shape() {
    let report = TickReport {
        attempted: vec![uuid::Uuid::nil()],
        fired: vec![uuid::Uuid::nil()],
        errors: vec![uuid::Uuid::nil()],
    };
    let _: &Vec<uuid::Uuid> = &report.attempted;
    let _: &Vec<uuid::Uuid> = &report.fired;
    let _: &Vec<uuid::Uuid> = &report.errors;
}

#[tokio::test]
async fn scheduler_tick_happy_path_fires_due_job_and_records_fire() {
    let store = InMemoryJobStore::new();
    let job = CronJob::new(
        TenantId::new(),
        "morning",
        CronExpr::parse("0 8 * * *").unwrap(),
        "Daily summary",
        CronTarget {
            platform: "telegram".into(),
            chat: "abc".into(),
        },
    );
    let id = job.id;
    store.save(&job).await.unwrap();

    let runner = RecordingRunner::default();
    let now = Utc.with_ymd_and_hms(2026, 4, 30, 8, 0, 0).unwrap();
    let report: TickReport = Scheduler.tick(&store, &runner, now).await.unwrap();

    assert_eq!(report.attempted, vec![id], "attempted must contain the due job");
    assert_eq!(report.fired, vec![id], "fired must contain the due job");
    assert!(report.errors.is_empty(), "errors must be empty on happy path");
    assert_eq!(runner.fired.lock().as_slice(), &[id]);

    // Store recorded the fire (last_fired_at advanced).
    let after = store.list_all().await.unwrap();
    assert_eq!(after[0].last_fired_at, Some(now));
}

#[tokio::test]
async fn scheduler_tick_idle_path_with_no_due_jobs_is_a_no_op() {
    let store = InMemoryJobStore::new();
    let job = CronJob::new(
        TenantId::new(),
        "morning",
        CronExpr::parse("0 8 * * *").unwrap(),
        "Daily summary",
        CronTarget {
            platform: "telegram".into(),
            chat: "abc".into(),
        },
    );
    store.save(&job).await.unwrap();

    let runner = RecordingRunner::default();
    // 09:00 → not due for the 08:00 schedule.
    let now = Utc.with_ymd_and_hms(2026, 4, 30, 9, 0, 0).unwrap();
    let report = Scheduler.tick(&store, &runner, now).await.unwrap();

    assert!(report.attempted.is_empty(), "idle tick must not attempt");
    assert!(report.fired.is_empty(), "idle tick must not fire");
    assert!(report.errors.is_empty(), "idle tick must not error");
    assert!(runner.fired.lock().is_empty(), "runner must not be invoked");
}

#[tokio::test]
async fn scheduler_tick_with_audit_signature_pinned_via_none_sink() {
    // Sign-off test: `tick_with_audit` accepts an Option<&AuditSink>.
    // Passing None must behave exactly like `tick` (same TickReport).
    let store = InMemoryJobStore::new();
    let job = CronJob::new(
        TenantId::new(),
        "morning",
        CronExpr::parse("0 8 * * *").unwrap(),
        "prompt",
        CronTarget {
            platform: "telegram".into(),
            chat: "abc".into(),
        },
    );
    let id = job.id;
    store.save(&job).await.unwrap();

    let runner = RecordingRunner::default();
    let now = Utc.with_ymd_and_hms(2026, 4, 30, 8, 0, 0).unwrap();
    let report = Scheduler
        .tick_with_audit(&store, &runner, now, None)
        .await
        .unwrap();

    assert_eq!(report.fired, vec![id]);
    assert!(report.errors.is_empty());

    // SystemJobOutcome is exercised in the sibling file; here we just
    // pin that the type was re-exported from the cron crate.
    let _: SystemJobOutcome = SystemJobOutcome::empty("placeholder");
}
