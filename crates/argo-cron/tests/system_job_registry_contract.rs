//! Contract pins for the `argo-cron` public surface (part 2 of 2).
//!
//! Companion to `tests/surface_contract.rs`. This file covers:
//!
//! - `CronJob` shape (struct-literal pub-field pin) + `CronJob::new`
//!   constructor signature.
//! - `CronJob` serde golden round-trip against every shipped
//!   `cron/system/*.json` fixture (5 files).
//! - Rejection contract: missing required fields, malformed schedule
//!   variant tagging.
//! - `system_target` / `system_job_name` convention round-trip + the
//!   canonical-jobs constants (count, uniqueness, exact string values).
//! - `SystemJobRegistry` / `SystemJobRegistryBuilder` / `SystemJobHandler`
//!   / `SystemJobContext` / `SystemJobOutcome` surface — Send+Sync
//!   bounds, builder LWW semantics, registry empty/get/has/names
//!   contract.
//!
//! See `tests/surface_contract.rs` for the file-split rationale and
//! the sensible-deviation list (brief mis-spelling of `Job`,
//! `CronTarget` enum variants, `tick_lock` API surface, etc.).

#![allow(dead_code)]

use argo_cron::{
    system_job_name, system_target, CronExpr, CronJob, CronTarget, SystemJobContext,
    SystemJobHandler, SystemJobOutcome, SystemJobRegistry, SystemJobRegistryBuilder,
    CANONICAL_SYSTEM_JOBS, SYSTEM_JOB_AB_PROMOTION, SYSTEM_JOB_DREAMS_CURATION,
    SYSTEM_JOB_FORGE_OPTIMIZE, SYSTEM_JOB_SKILL_QUALITY_RECOMPUTE, SYSTEM_PLATFORM,
};
use argo_types::{ArgoResult, TenantId};
use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use std::path::PathBuf;
use std::sync::Arc;

// ---------------------------------------------------------------------------
// CronJob shape pin
// ---------------------------------------------------------------------------

#[test]
fn cron_job_pub_field_struct_literal_pins_shape() {
    // Struct-literal use of every pub field — adding a field is a
    // breaking change at this callsite and fails compilation.
    let tenant = TenantId::new();
    let expr = CronExpr::parse("0 8 * * *").unwrap();
    let job = CronJob {
        id: uuid::Uuid::now_v7(),
        tenant_id: tenant,
        name: "morning".to_string(),
        schedule: expr,
        prompt: "Summarize unread Slack DMs".to_string(),
        target: CronTarget {
            platform: "telegram".into(),
            chat: "abc".into(),
        },
        enabled: true,
        last_fired_at: None,
    };
    // Type-pin field access for each pub field.
    let _: uuid::Uuid = job.id;
    let _: TenantId = job.tenant_id;
    let _: &str = job.name.as_str();
    let _: &CronExpr = &job.schedule;
    let _: &str = job.prompt.as_str();
    let _: &CronTarget = &job.target;
    let _: bool = job.enabled;
    let _: Option<chrono::DateTime<chrono::Utc>> = job.last_fired_at;
}

#[test]
fn cron_job_new_constructor_signature_pinned() {
    let expr = CronExpr::parse("0 8 * * *").unwrap();
    let job = CronJob::new(
        TenantId::new(),
        "morning",
        expr,
        "prompt",
        CronTarget {
            platform: "telegram".into(),
            chat: "abc".into(),
        },
    );
    assert!(job.enabled, "freshly-built jobs default to enabled");
    assert_eq!(job.last_fired_at, None);
    assert_ne!(
        job.id,
        uuid::Uuid::nil(),
        "CronJob::new must mint a non-nil UUIDv7"
    );
}

// ---------------------------------------------------------------------------
// Golden serde round-trip vs. cron/system/*.json
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is `<repo>/crates/argo-cron` — pop twice to the
    // workspace root that contains `cron/system/*.json`.
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .and_then(|p| p.parent())
        .map(PathBuf::from)
        .expect("workspace root above crates/argo-cron")
}

#[test]
fn cron_job_serde_round_trips_all_five_system_fixtures() {
    // Brief AC #3: every cron/system/*.json round-trips through
    // `CronJob` serde. There are 5 shipped fixtures — 4 in
    // CANONICAL_SYSTEM_JOBS + audit_evidence_archival_tick.
    let fixture_dir = workspace_root().join("cron/system");
    let fixtures = [
        "ab_promotion_tick.json",
        "audit_evidence_archival_tick.json",
        "dreams_curation_tick.json",
        "forge_optimize_tick.json",
        "skill_quality_recompute.json",
    ];
    assert_eq!(fixtures.len(), 5);

    for fname in fixtures {
        let path = fixture_dir.join(fname);
        let raw = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        let job: CronJob = serde_json::from_str(&raw)
            .unwrap_or_else(|e| panic!("parse {}: {e}", path.display()));

        // Every fixture is a system-target job.
        assert_eq!(
            job.target.platform, SYSTEM_PLATFORM,
            "{fname} target.platform must be \"system\""
        );
        assert!(
            !job.target.chat.is_empty(),
            "{fname} target.chat must be non-empty"
        );

        // Re-serialize and re-parse to confirm round-trip stability.
        let reser = serde_json::to_string(&job).expect("CronJob Serialize");
        let again: CronJob = serde_json::from_str(&reser).expect("CronJob Deserialize");
        assert_eq!(again.id, job.id);
        assert_eq!(again.tenant_id, job.tenant_id);
        assert_eq!(again.name, job.name);
        assert_eq!(again.schedule.source, job.schedule.source);
        assert_eq!(again.target, job.target);
        assert_eq!(again.enabled, job.enabled);
    }
}

#[test]
fn cron_job_serde_rejects_missing_target_field() {
    // Mutate forge_optimize_tick.json by deleting the `target` key —
    // serde must refuse the document because `target` is required.
    let raw =
        std::fs::read_to_string(workspace_root().join("cron/system/forge_optimize_tick.json"))
            .unwrap();
    let mut value: serde_json::Value = serde_json::from_str(&raw).unwrap();
    value.as_object_mut().unwrap().remove("target");
    let mutated = serde_json::to_string(&value).unwrap();
    let err = serde_json::from_str::<CronJob>(&mutated).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("target"),
        "missing-target error must mention `target`, got: {msg}"
    );
}

#[test]
fn cron_job_serde_rejects_malformed_schedule_variant() {
    // Mutate the schedule.minute field to an unknown enum variant.
    let bad = serde_json::json!({
        "id": "00000000-0000-7000-8000-000000000099",
        "tenant_id": "00000000-0000-0000-0000-000000000000",
        "name": "bad",
        "schedule": {
            "source": "* * * * *",
            "minute": "NotARealFieldVariant",
            "hour": "Wildcard",
            "day_of_month": "Wildcard",
            "month": "Wildcard",
            "day_of_week": "Wildcard"
        },
        "prompt": "x",
        "target": {"platform": "system", "chat": "x"},
        "enabled": true,
        "last_fired_at": null
    });
    let raw = serde_json::to_string(&bad).unwrap();
    let err = serde_json::from_str::<CronJob>(&raw).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("variant") || msg.contains("minute") || msg.contains("expected"),
        "malformed-field error should surface the parse failure, got: {msg}"
    );
}

// ---------------------------------------------------------------------------
// system_target / system_job_name convention + canonical-jobs constants
// ---------------------------------------------------------------------------

#[test]
fn system_target_convention_round_trips() {
    for &name in CANONICAL_SYSTEM_JOBS {
        let t = system_target(name);
        assert_eq!(t.platform, SYSTEM_PLATFORM);
        assert_eq!(t.chat, name);
        assert_eq!(system_job_name(&t), Some(name));
    }

    // Non-system targets must return None.
    let t = CronTarget {
        platform: "slack".into(),
        chat: "C123".into(),
    };
    assert_eq!(system_job_name(&t), None);
}

#[test]
fn canonical_system_jobs_count_and_uniqueness_pinned() {
    // Brief says 4 canonical jobs (audit_evidence_archival is shipped
    // as a 5th fixture but not yet enumerated in the const).
    assert_eq!(
        CANONICAL_SYSTEM_JOBS.len(),
        4,
        "CANONICAL_SYSTEM_JOBS count drift"
    );

    let canonical: Vec<&str> = CANONICAL_SYSTEM_JOBS.to_vec();
    assert!(canonical.contains(&SYSTEM_JOB_FORGE_OPTIMIZE));
    assert!(canonical.contains(&SYSTEM_JOB_SKILL_QUALITY_RECOMPUTE));
    assert!(canonical.contains(&SYSTEM_JOB_DREAMS_CURATION));
    assert!(canonical.contains(&SYSTEM_JOB_AB_PROMOTION));

    // Uniqueness — no duplicate canonical names.
    let mut seen = std::collections::HashSet::new();
    for name in CANONICAL_SYSTEM_JOBS {
        assert!(seen.insert(*name), "duplicate canonical name: {name}");
    }

    // Canonical name strings pinned as written — guards against
    // accidental rename ("forge.optimize" → "forge_optimize_tick").
    assert_eq!(SYSTEM_JOB_FORGE_OPTIMIZE, "forge.optimize");
    assert_eq!(SYSTEM_JOB_SKILL_QUALITY_RECOMPUTE, "skill.quality_recompute");
    assert_eq!(SYSTEM_JOB_DREAMS_CURATION, "dreams.curation_tick");
    assert_eq!(SYSTEM_JOB_AB_PROMOTION, "ab.promotion");
    assert_eq!(SYSTEM_PLATFORM, "system");
}

// ---------------------------------------------------------------------------
// SystemJobRegistry + SystemJobHandler trait-object contract
// ---------------------------------------------------------------------------

struct StubHandler {
    outcome: SystemJobOutcome,
}

#[async_trait]
impl SystemJobHandler for StubHandler {
    async fn run(&self, _ctx: SystemJobContext) -> ArgoResult<SystemJobOutcome> {
        Ok(self.outcome.clone())
    }
}

#[test]
fn system_job_registry_is_send_and_sync_at_compile_time() {
    // Compile-time bounds check — needed by the gateway boot path,
    // which clones the registry across the async runtime. If a
    // future revision adds a non-Send field to SystemJobRegistry,
    // this fn signature stops compiling.
    fn require_send_sync<T: Send + Sync>() {}
    require_send_sync::<SystemJobRegistry>();
    require_send_sync::<SystemJobRegistryBuilder>();
    require_send_sync::<SystemJobContext>();
    require_send_sync::<SystemJobOutcome>();
    // Arc<dyn SystemJobHandler> is the registry's storage shape.
    require_send_sync::<Arc<dyn SystemJobHandler>>();
}

#[test]
fn system_job_outcome_pub_field_struct_literal_pins_shape() {
    let outcome = SystemJobOutcome {
        jobs_processed: 7,
        skipped: 1,
        errors: vec!["one failed".to_string()],
        summary: "did 7, skipped 1, 1 err".into(),
    };
    let _: usize = outcome.jobs_processed;
    let _: usize = outcome.skipped;
    let _: &Vec<String> = &outcome.errors;
    let _: &str = outcome.summary.as_str();

    let empty = SystemJobOutcome::empty("nothing to do");
    assert_eq!(empty.jobs_processed, 0);
    assert_eq!(empty.skipped, 0);
    assert!(empty.errors.is_empty());
    assert_eq!(empty.summary, "nothing to do");
}

#[test]
fn system_job_context_constructors_pinned() {
    let root = PathBuf::from("/tmp/workspace");
    let now = Utc.with_ymd_and_hms(2026, 5, 24, 12, 0, 0).unwrap();
    let ctx = SystemJobContext::new(root.clone(), now);
    assert_eq!(ctx.workspace_root, root);
    assert_eq!(ctx.now, now);

    let at_now = SystemJobContext::at_now(root.clone());
    assert_eq!(at_now.workspace_root, root);
    // `at_now` populates `now` from wall-clock; just confirm it is in
    // a reasonable post-2020 window so we know it isn't zero-init.
    assert!(at_now.now.timestamp() > 1_577_836_800);
}

#[tokio::test]
async fn system_job_registry_builder_dispatches_and_is_lww() {
    let first = Arc::new(StubHandler {
        outcome: SystemJobOutcome::empty("first"),
    });
    let second = Arc::new(StubHandler {
        outcome: SystemJobOutcome::empty("second"),
    });

    let reg = SystemJobRegistryBuilder::new()
        .register("demo.job", first)
        .register("demo.job", second) // last-writer-wins
        .build();

    assert!(reg.has("demo.job"));
    assert!(!reg.has("missing.job"));

    let handler = reg.get("demo.job").expect("registered");
    let ctx = SystemJobContext::at_now(PathBuf::from("/tmp"));
    let outcome = handler.run(ctx).await.unwrap();
    assert_eq!(
        outcome.summary, "second",
        "second register must overwrite the first (LWW)"
    );

    // Missing name returns None.
    assert!(reg.get("missing.job").is_none());
}

#[test]
fn system_job_registry_empty_and_names_sorted() {
    let empty = SystemJobRegistry::empty();
    assert!(empty.names().is_empty());
    assert!(empty.get("anything").is_none());
    assert!(!empty.has("anything"));

    let reg = SystemJobRegistryBuilder::new()
        .register(
            "zeta",
            Arc::new(StubHandler {
                outcome: SystemJobOutcome::empty(""),
            }),
        )
        .register(
            "alpha",
            Arc::new(StubHandler {
                outcome: SystemJobOutcome::empty(""),
            }),
        )
        .register(
            "mu",
            Arc::new(StubHandler {
                outcome: SystemJobOutcome::empty(""),
            }),
        )
        .build();

    assert_eq!(reg.names(), vec!["alpha", "mu", "zeta"]);
}
