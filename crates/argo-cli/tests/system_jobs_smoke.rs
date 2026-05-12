//! Integration smoke for `mission-cron-tick-maintenance-jobs`.
//!
//! Boots an in-memory [`argo_cron::Scheduler`] + scheduler tick over
//! the four canonical system maintenance jobs and asserts that:
//!
//! - Each shipped `cron/system/*.json` template parses as a
//!   [`argo_cron::CronJob`].
//! - The default [`argo_cron::SystemJobRegistry`] (constructed by
//!   `argo-cli` boot wiring) registers all four canonical names.
//! - Each registered handler returns a well-formed
//!   [`argo_cron::SystemJobOutcome`].
//! - The end-to-end cron tick path resolves `system:*` targets via
//!   the registry without falling through to the logging runner.
//!
//! The handlers are wired to no-op default impls today
//! (`mission-cron-tick-maintenance-jobs` is a wiring brief); the
//! follow-up `*-real-deps` briefs swap real selectors/writers in.
//! The smoke verifies the wiring itself, not the per-handler
//! business logic — those tests live next to each handler module.

use argo_cron::{
    system_job_name, CronExpr, CronJob, CronTarget, InMemoryJobStore, JobRunner, JobStore,
    Scheduler, SystemJobContext, SystemJobHandler, SystemJobOutcome, SystemJobRegistry,
    SystemJobRegistryBuilder, SYSTEM_JOB_AB_PROMOTION, SYSTEM_JOB_DREAMS_CURATION,
    SYSTEM_JOB_FORGE_OPTIMIZE, SYSTEM_JOB_SKILL_QUALITY_RECOMPUTE,
};
use argo_types::{ArgoResult, TenantId};
use async_trait::async_trait;
use std::sync::Mutex;
use std::path::PathBuf;
use std::sync::Arc;

/// Counting handler: records every invocation and returns the given
/// outcome. Used to assert dispatch reaches the registered handler.
struct CountingHandler {
    name: &'static str,
    invocations: Mutex<usize>,
}

#[async_trait]
impl SystemJobHandler for CountingHandler {
    async fn run(&self, _ctx: SystemJobContext) -> ArgoResult<SystemJobOutcome> {
        let mut g = self.invocations.lock().unwrap();
        *g += 1;
        Ok(SystemJobOutcome {
            jobs_processed: 1,
            skipped: 0,
            errors: Vec::new(),
            summary: format!("{}: ok", self.name),
        })
    }
}

fn ws_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

#[test]
fn cron_system_templates_parse_as_cron_jobs() {
    let system_dir = ws_root().join("cron").join("system");
    assert!(
        system_dir.exists(),
        "cron/system/ directory missing at {}",
        system_dir.display()
    );
    for name in [
        "forge_optimize_tick",
        "skill_quality_recompute",
        "dreams_curation_tick",
        "ab_promotion_tick",
    ] {
        let p = system_dir.join(format!("{name}.json"));
        let body = std::fs::read_to_string(&p).expect("read template");
        let job: CronJob =
            serde_json::from_str(&body).unwrap_or_else(|e| panic!("{} parse failed: {e}", name));
        assert_eq!(job.name, name);
        let sys_name = system_job_name(&job.target).expect("target is system");
        assert!(!sys_name.is_empty());
    }
}

fn fixed_handler(name: &'static str) -> Arc<CountingHandler> {
    Arc::new(CountingHandler {
        name,
        invocations: Mutex::new(0),
    })
}

#[tokio::test]
async fn registry_dispatches_all_four_canonical_jobs() {
    let forge = fixed_handler("forge");
    let quality = fixed_handler("quality");
    let dreams = fixed_handler("dreams");
    let ab = fixed_handler("ab");
    let reg: SystemJobRegistry = SystemJobRegistryBuilder::new()
        .register(SYSTEM_JOB_FORGE_OPTIMIZE, forge.clone())
        .register(SYSTEM_JOB_SKILL_QUALITY_RECOMPUTE, quality.clone())
        .register(SYSTEM_JOB_DREAMS_CURATION, dreams.clone())
        .register(SYSTEM_JOB_AB_PROMOTION, ab.clone())
        .build();
    assert_eq!(
        reg.names(),
        vec![
            SYSTEM_JOB_AB_PROMOTION,
            SYSTEM_JOB_DREAMS_CURATION,
            SYSTEM_JOB_FORGE_OPTIMIZE,
            SYSTEM_JOB_SKILL_QUALITY_RECOMPUTE,
        ]
    );
    let ctx = SystemJobContext::at_now(PathBuf::from("/tmp"));
    for n in [
        SYSTEM_JOB_FORGE_OPTIMIZE,
        SYSTEM_JOB_SKILL_QUALITY_RECOMPUTE,
        SYSTEM_JOB_DREAMS_CURATION,
        SYSTEM_JOB_AB_PROMOTION,
    ] {
        let h = reg.get(n).expect("registered");
        let out = h.run(ctx.clone()).await.unwrap();
        assert_eq!(out.jobs_processed, 1);
        assert!(out.summary.contains("ok"));
    }
    assert_eq!(*forge.invocations.lock().unwrap(), 1);
    assert_eq!(*quality.invocations.lock().unwrap(), 1);
    assert_eq!(*dreams.invocations.lock().unwrap(), 1);
    assert_eq!(*ab.invocations.lock().unwrap(), 1);
}

struct SystemRunner {
    registry: SystemJobRegistry,
    workspace_root: PathBuf,
    delivered: Mutex<Vec<String>>,
}

#[async_trait]
impl JobRunner for SystemRunner {
    async fn run(&self, job: &CronJob) -> ArgoResult<()> {
        if let Some(name) = system_job_name(&job.target) {
            let handler = self
                .registry
                .get(name)
                .expect("test wires every system job");
            let ctx = SystemJobContext::at_now(self.workspace_root.clone());
            let outcome = handler.run(ctx).await?;
            self.delivered.lock().unwrap().push(outcome.summary);
        } else {
            self.delivered.lock().unwrap().push(format!(
                "non-system: {}/{}",
                job.target.platform, job.target.chat
            ));
        }
        Ok(())
    }
}

#[tokio::test]
async fn scheduler_tick_dispatches_system_targets_through_registry() {
    let forge = fixed_handler("forge");
    let reg = SystemJobRegistryBuilder::new()
        .register(SYSTEM_JOB_FORGE_OPTIMIZE, forge.clone())
        .build();
    let store = InMemoryJobStore::new();
    let now = chrono::Utc::now();
    // Build a CronJob with `* * * * *` schedule so it always fires
    // on the test tick. Target is system:forge.optimize so dispatch
    // must go through the registry.
    let job = CronJob::new(
        TenantId::new(),
        "smoke_forge",
        CronExpr::parse("* * * * *").unwrap(),
        "system: forge optimize tick",
        CronTarget {
            platform: argo_cron::SYSTEM_PLATFORM.into(),
            chat: SYSTEM_JOB_FORGE_OPTIMIZE.into(),
        },
    );
    store.save(&job).await.unwrap();
    let runner = SystemRunner {
        registry: reg,
        workspace_root: PathBuf::from("/tmp"),
        delivered: Mutex::new(Vec::new()),
    };
    let report = Scheduler.tick(&store, &runner, now).await.unwrap();
    assert_eq!(report.fired.len(), 1);
    assert_eq!(*forge.invocations.lock().unwrap(), 1);
    let delivered = runner.delivered.lock().unwrap().clone();
    assert_eq!(delivered.len(), 1);
    assert!(delivered[0].contains("forge"));
}

#[tokio::test]
async fn empty_registry_keeps_non_system_targets_undispatched_through_registry() {
    let reg = SystemJobRegistry::empty();
    let store = InMemoryJobStore::new();
    let now = chrono::Utc::now();
    let job = CronJob::new(
        TenantId::new(),
        "user_telegram",
        CronExpr::parse("* * * * *").unwrap(),
        "Daily summary",
        CronTarget {
            platform: "telegram".into(),
            chat: "abc".into(),
        },
    );
    store.save(&job).await.unwrap();
    let runner = SystemRunner {
        registry: reg,
        workspace_root: PathBuf::from("/tmp"),
        delivered: Mutex::new(Vec::new()),
    };
    let report = Scheduler.tick(&store, &runner, now).await.unwrap();
    assert_eq!(report.fired.len(), 1);
    let delivered = runner.delivered.lock().unwrap().clone();
    assert_eq!(delivered.len(), 1);
    assert!(delivered[0].starts_with("non-system: telegram/"));
}
