//! `argo cron` — list / create / delete / trigger / tick jobs in
//! the workspace's `cron/jobs/` directory.
use crate::cli::CronCmd;
use crate::system_jobs::{
    ab_promotion_tick, audit_evidence_archival_tick, dreams_curation_tick, forge_optimize,
    skill_quality_recompute,
};
use argo_cron::{
    system_job_name, CronExpr, CronJob, CronTarget, InMemoryJobStore, JobRunner, JobStore,
    Scheduler, SystemJobContext, SystemJobRegistry, SystemJobRegistryBuilder, TickLock,
    TickLockError, SYSTEM_JOB_AB_PROMOTION, SYSTEM_JOB_DREAMS_CURATION, SYSTEM_JOB_FORGE_OPTIMIZE,
    SYSTEM_JOB_SKILL_QUALITY_RECOMPUTE,
};
use argo_types::{ArgoError, ArgoResult, TenantId};
use async_trait::async_trait;
use std::path::PathBuf;
use std::time::Duration;

/// Names of the four canonical system maintenance jobs shipped by
/// `argo cron init-system`. Stable order: `init-system` writes
/// templates in this order and `--dry-run` summaries echo it.
pub const SYSTEM_JOB_TEMPLATES: &[&str] = &[
    "forge_optimize_tick",
    "skill_quality_recompute",
    "dreams_curation_tick",
    "ab_promotion_tick",
    "audit_evidence_archival_tick",
];

/// Construct the default workspace-global [`SystemJobRegistry`].
/// Today every entry is wired to a no-op handler — operators
/// register real handlers via follow-up briefs once their store /
/// forge / dreams / ab plumbing is ready. The registry itself goes
/// live now so dispatch through `argo cron tick` works end-to-end.
#[must_use]
pub fn default_system_registry() -> SystemJobRegistry {
    SystemJobRegistryBuilder::new()
        .register(SYSTEM_JOB_FORGE_OPTIMIZE, forge_optimize::noop_handler())
        .register(
            SYSTEM_JOB_SKILL_QUALITY_RECOMPUTE,
            skill_quality_recompute::noop_handler(),
        )
        .register(
            SYSTEM_JOB_DREAMS_CURATION,
            dreams_curation_tick::noop_handler(),
        )
        .register(SYSTEM_JOB_AB_PROMOTION, ab_promotion_tick::noop_handler())
        .register(
            audit_evidence_archival_tick::SYSTEM_JOB_AUDIT_EVIDENCE_ARCHIVAL,
            audit_evidence_archival_tick::default_handler(),
        )
        .build()
}

pub async fn run(cmd: CronCmd) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let dir = cwd.join("cron").join("jobs");
    std::fs::create_dir_all(&dir)
        .map_err(|e| ArgoError::Provider(format!("create_dir_all {}: {e}", dir.display())))?;
    match cmd {
        CronCmd::List { json, format } => list_jobs(&dir, json, &format),
        CronCmd::Create {
            name,
            schedule,
            prompt,
            platform,
            chat,
        } => create_job(&dir, &name, &schedule, &prompt, &platform, &chat),
        CronCmd::Delete { name } => delete_job(&dir, &name),
        CronCmd::Edit { name } => {
            println!(
                "argo cron edit {name}: open {}/{name}.json in your editor; the daemon picks up changes on the next tick.",
                dir.display()
            );
            Ok(())
        }
        CronCmd::Trigger { name } => trigger_job(&cwd, &dir, &name).await,
        CronCmd::Tick { dry_run } => tick(&cwd, &dir, dry_run).await,
        CronCmd::Daemon {
            interval_secs,
            max_ticks,
        } => daemon(&cwd, &dir, interval_secs, max_ticks).await,
        CronCmd::InitSystem { dry_run, force } => init_system(&cwd, &dir, dry_run, force),
    }
}

/// Install the four canonical system maintenance job templates into
/// `cron/jobs/`. The source templates live in `cron/system/` at the
/// workspace root. Existing files are preserved unless `force` is
/// set; missing source files are reported and the call still
/// installs whatever it can.
fn init_system(
    workspace_root: &std::path::Path,
    jobs_dir: &std::path::Path,
    dry_run: bool,
    force: bool,
) -> ArgoResult<()> {
    let system_dir = workspace_root.join("cron").join("system");
    if !system_dir.exists() {
        return Err(ArgoError::Provider(format!(
            "{} not found — `cron/system/` ships with the repo; \
             reinstall or fetch a fresh checkout",
            system_dir.display()
        )));
    }
    let mut wrote = 0_usize;
    let mut skipped = 0_usize;
    let mut errors = 0_usize;
    for name in SYSTEM_JOB_TEMPLATES {
        let src = system_dir.join(format!("{name}.json"));
        let dst = jobs_dir.join(format!("{name}.json"));
        if !src.exists() {
            eprintln!("  missing template: {} (skipping)", src.display());
            errors += 1;
            continue;
        }
        if dst.exists() && !force {
            println!("  skip   {} (already installed; pass --force to overwrite)", dst.display());
            skipped += 1;
            continue;
        }
        if dry_run {
            println!("  would write {} (from {})", dst.display(), src.display());
            wrote += 1;
            continue;
        }
        let body = std::fs::read_to_string(&src)
            .map_err(|e| ArgoError::Provider(format!("read {}: {e}", src.display())))?;
        // Validate that the template parses as a CronJob before we
        // commit it to cron/jobs/ — catches drift between the
        // shipped templates and the on-disk schema.
        let _: CronJob = serde_json::from_str(&body).map_err(|e| {
            ArgoError::Provider(format!("template {} is not a valid CronJob: {e}", src.display()))
        })?;
        std::fs::write(&dst, body)
            .map_err(|e| ArgoError::Provider(format!("write {}: {e}", dst.display())))?;
        println!("  wrote  {}", dst.display());
        wrote += 1;
    }
    println!(
        "argo cron init-system: {} {} / {} skipped / {} missing-template{}",
        if dry_run { "would write" } else { "wrote" },
        wrote,
        skipped,
        errors,
        if dry_run { " (dry-run)" } else { "" }
    );
    Ok(())
}

/// Build a [`JobRunner`] that dispatches `system:*` targets via the
/// registry and falls back to [`LoggingRunner`] for everything else.
fn system_aware_runner(
    registry: SystemJobRegistry,
    workspace_root: PathBuf,
) -> SystemAwareRunner {
    SystemAwareRunner {
        registry,
        workspace_root,
    }
}

struct SystemAwareRunner {
    registry: SystemJobRegistry,
    workspace_root: PathBuf,
}

#[async_trait]
impl JobRunner for SystemAwareRunner {
    async fn run(&self, job: &CronJob) -> ArgoResult<()> {
        if let Some(name) = system_job_name(&job.target) {
            match self.registry.get(name) {
                Some(handler) => {
                    let ctx = SystemJobContext::at_now(self.workspace_root.clone());
                    let outcome = handler.run(ctx).await?;
                    // Structured JSON for downstream log collectors.
                    if let Ok(json) = serde_json::to_string(&outcome) {
                        println!("{json}");
                    }
                    eprintln!("  system:{} → {}", name, outcome.summary);
                    return Ok(());
                }
                None => {
                    eprintln!(
                        "  system:{} → handler not registered (skipping); \
                         operators register handlers via the default registry",
                        name
                    );
                    return Ok(());
                }
            }
        }
        LoggingRunner.run(job).await
    }
}

async fn daemon(
    workspace_root: &std::path::Path,
    dir: &std::path::Path,
    interval_secs: u64,
    max_ticks: Option<u64>,
) -> ArgoResult<()> {
    let lock_path = dir
        .parent()
        .map(|p| p.join(".tick.lock"))
        .unwrap_or_else(|| dir.join(".tick.lock"));
    let _lock = match TickLock::try_acquire(&lock_path) {
        Ok(l) => l,
        Err(TickLockError::AlreadyHeld { path }) => {
            return Err(ArgoError::Provider(format!(
                "tick lock at {} is already held — another daemon is running",
                path.display()
            )));
        }
        Err(e) => return Err(ArgoError::Provider(e.to_string())),
    };
    println!(
        "argo cron daemon: tick every {interval_secs}s (lock={}); SIGHUP triggers a config-reload + immediate tick",
        lock_path.display()
    );
    let mut ticks = 0_u64;
    let mut sighup = sighup_stream();
    loop {
        if let Err(e) = tick(workspace_root, dir, false).await {
            eprintln!("tick error: {e}");
        }
        ticks += 1;
        if max_ticks.is_some_and(|cap| ticks >= cap) {
            println!("daemon: reached --max-ticks {ticks}; exiting");
            return Ok(());
        }
        // Race the regular interval against SIGHUP. On SIGHUP we
        // log "config reload" and immediately fall through to the
        // next tick — the next iteration re-reads `cron/jobs/`
        // from disk, so any edits since boot are picked up.
        let sleep = tokio::time::sleep(Duration::from_secs(interval_secs));
        tokio::pin!(sleep);
        tokio::select! {
            () = &mut sleep => {}
            _ = wait_sighup(&mut sighup) => {
                println!("daemon: received SIGHUP — reloading jobs on next tick");
            }
        }
    }
}

/// Per-platform SIGHUP stream.
#[cfg(unix)]
fn sighup_stream() -> Option<tokio::signal::unix::Signal> {
    tokio::signal::unix::signal(tokio::signal::unix::SignalKind::hangup()).ok()
}

#[cfg(not(unix))]
fn sighup_stream() -> Option<()> {
    None
}

#[cfg(unix)]
async fn wait_sighup(s: &mut Option<tokio::signal::unix::Signal>) {
    match s {
        Some(s) => {
            let _ = s.recv().await;
        }
        None => std::future::pending::<()>().await,
    }
}

#[cfg(not(unix))]
async fn wait_sighup(_s: &mut Option<()>) {
    // Windows: SIGHUP doesn't exist; fall through to interval.
    std::future::pending::<()>().await
}

async fn trigger_job(
    workspace_root: &std::path::Path,
    dir: &std::path::Path,
    name: &str,
) -> ArgoResult<()> {
    let path = dir.join(format!("{name}.json"));
    if !path.exists() {
        return Err(ArgoError::Provider(format!("{} not found", path.display())));
    }
    let body = std::fs::read_to_string(&path)
        .map_err(|e| ArgoError::Provider(format!("read {}: {e}", path.display())))?;
    let job: CronJob = serde_json::from_str(&body)
        .map_err(|e| ArgoError::Provider(format!("parse {}: {e}", path.display())))?;
    let runner = system_aware_runner(default_system_registry(), workspace_root.to_path_buf());
    runner.run(&job).await?;
    println!("triggered '{name}' (no-op delivery; daemon binding wires the gateway send)");
    Ok(())
}

async fn tick(
    workspace_root: &std::path::Path,
    dir: &std::path::Path,
    dry_run: bool,
) -> ArgoResult<()> {
    // Load every job into an InMemoryJobStore so we can run a real
    // scheduler tick. This is the same path the daemon takes; the
    // only difference is the cursor lives in process memory, so
    // the dedup check holds within one CLI invocation but resets
    // on the next. The persisted cursor (under fcntl/msvcrt lock)
    // is the daemon's job.
    let store = InMemoryJobStore::new();
    for entry in std::fs::read_dir(dir)
        .map_err(|e| ArgoError::Provider(format!("read_dir {}: {e}", dir.display())))?
    {
        let e = entry.map_err(|e| ArgoError::Provider(format!("dir entry: {e}")))?;
        let p = e.path();
        if p.extension().is_none_or(|x| x != "json") {
            continue;
        }
        let body = std::fs::read_to_string(&p)
            .map_err(|e| ArgoError::Provider(format!("read {}: {e}", p.display())))?;
        let job: CronJob = serde_json::from_str(&body)
            .map_err(|e| ArgoError::Provider(format!("parse {}: {e}", p.display())))?;
        store.save(&job).await?;
    }
    let runner = system_aware_runner(default_system_registry(), workspace_root.to_path_buf());
    let now = chrono::Utc::now();
    let report = if dry_run {
        // Dry run: list which jobs are due without invoking the
        // runner. We re-read directly off the in-memory store to
        // avoid the side effect.
        let due: Vec<_> = store
            .list_all()
            .await?
            .into_iter()
            .filter(|j| j.is_due(now))
            .collect();
        println!("dry run: {} job(s) due at {}", due.len(), now);
        for j in due {
            println!("  {} ({})", j.name, j.schedule.source);
        }
        return Ok(());
    } else {
        Scheduler.tick(&store, &runner, now).await?
    };
    println!(
        "tick at {now}: attempted={} fired={} errors={}",
        report.attempted.len(),
        report.fired.len(),
        report.errors.len()
    );
    Ok(())
}

/// Logs the job's `(name, prompt → platform/chat)` to stdout
/// instead of actually delivering. The daemon swaps this out for
/// the real \`argo-core::Agent\` + \`argo-gateway\` chain.
struct LoggingRunner;

#[async_trait]
impl JobRunner for LoggingRunner {
    async fn run(&self, job: &CronJob) -> ArgoResult<()> {
        println!(
            "  fire: {} → {}/{} | prompt: {}",
            job.name, job.target.platform, job.target.chat, job.prompt
        );
        Ok(())
    }
}

fn list_jobs(dir: &std::path::Path, json: bool, format: &str) -> ArgoResult<()> {
    let mut entries: Vec<PathBuf> = Vec::new();
    for entry in std::fs::read_dir(dir)
        .map_err(|e| ArgoError::Provider(format!("read_dir {}: {e}", dir.display())))?
    {
        let e = entry.map_err(|e| ArgoError::Provider(format!("dir entry: {e}")))?;
        let p = e.path();
        if p.extension().is_some_and(|x| x == "json") {
            entries.push(p);
        }
    }
    entries.sort();
    let pick = if json { "json" } else { format };
    if pick == "text" && entries.is_empty() {
        println!("(no cron jobs in {})", dir.display());
        return Ok(());
    }
    let jobs: Vec<CronJob> = entries
        .iter()
        .filter_map(|p| {
            std::fs::read_to_string(p)
                .ok()
                .and_then(|b| serde_json::from_str::<CronJob>(&b).ok())
        })
        .collect();
    match pick {
        "json" => {
            for job in &jobs {
                let canonical = serde_json::to_string(job)
                    .map_err(|e| ArgoError::Provider(format!("serialize job: {e}")))?;
                println!("{canonical}");
            }
        }
        "markdown" => {
            println!("# Cron jobs ({} matches)", jobs.len());
            println!();
            if jobs.is_empty() {
                println!("_(no cron jobs in {})_", dir.display());
                return Ok(());
            }
            println!("| name | schedule | platform | chat | enabled |");
            println!("| --- | --- | --- | --- | :---: |");
            for job in &jobs {
                println!(
                    "| `{}` | `{}` | `{}` | `{}` | {} |",
                    job.name,
                    job.schedule.source,
                    job.target.platform,
                    job.target.chat,
                    if job.enabled { "✓" } else { "·" }
                );
            }
        }
        "text" => {
            for job in &jobs {
                println!(
                    "  {:<24} {:<14} platform={:<10} enabled={}",
                    job.name, job.schedule.source, job.target.platform, job.enabled
                );
            }
        }
        other => {
            return Err(ArgoError::Provider(format!(
                "unknown format '{other}' (expected 'text', 'json', or 'markdown')"
            )));
        }
    }
    Ok(())
}

fn create_job(
    dir: &std::path::Path,
    name: &str,
    schedule: &str,
    prompt: &str,
    platform: &str,
    chat: &str,
) -> ArgoResult<()> {
    let cron = CronExpr::parse(schedule)
        .map_err(|e| ArgoError::Provider(format!("schedule '{schedule}': {e}")))?;
    let job = CronJob::new(
        TenantId::new(),
        name,
        cron,
        prompt,
        CronTarget {
            platform: platform.to_string(),
            chat: chat.to_string(),
        },
    );
    let path = dir.join(format!("{name}.json"));
    if path.exists() {
        return Err(ArgoError::Provider(format!(
            "{} already exists; delete it first or pick a different name",
            path.display()
        )));
    }
    let body = serde_json::to_string_pretty(&job)
        .map_err(|e| ArgoError::Provider(format!("serialize job: {e}")))?;
    std::fs::write(&path, body)
        .map_err(|e| ArgoError::Provider(format!("write {}: {e}", path.display())))?;
    println!("created {} (schedule = '{}')", path.display(), schedule);
    Ok(())
}

fn delete_job(dir: &std::path::Path, name: &str) -> ArgoResult<()> {
    let path = dir.join(format!("{name}.json"));
    if !path.exists() {
        return Err(ArgoError::Provider(format!("{} not found", path.display())));
    }
    std::fs::remove_file(&path)
        .map_err(|e| ArgoError::Provider(format!("remove {}: {e}", path.display())))?;
    println!("deleted {}", path.display());
    Ok(())
}
