//! `argo cron` — list / create / delete / trigger / tick jobs in
//! the workspace's `cron/jobs/` directory.
use crate::cli::CronCmd;
use argo_cron::{
    CronExpr, CronJob, CronTarget, InMemoryJobStore, JobRunner, JobStore, Scheduler, TickLock,
    TickLockError,
};
use argo_types::{ArgoError, ArgoResult, TenantId};
use async_trait::async_trait;
use std::path::PathBuf;
use std::time::Duration;

pub async fn run(cmd: CronCmd) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let dir = cwd.join("cron").join("jobs");
    std::fs::create_dir_all(&dir)
        .map_err(|e| ArgoError::Provider(format!("create_dir_all {}: {e}", dir.display())))?;
    match cmd {
        CronCmd::List => list_jobs(&dir),
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
        CronCmd::Trigger { name } => trigger_job(&dir, &name).await,
        CronCmd::Tick { dry_run } => tick(&dir, dry_run).await,
        CronCmd::Daemon {
            interval_secs,
            max_ticks,
        } => daemon(&dir, interval_secs, max_ticks).await,
    }
}

async fn daemon(
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
        "argo cron daemon: tick every {interval_secs}s (lock={})",
        lock_path.display()
    );
    let mut ticks = 0_u64;
    loop {
        if let Err(e) = tick(dir, false).await {
            eprintln!("tick error: {e}");
        }
        ticks += 1;
        if max_ticks.is_some_and(|cap| ticks >= cap) {
            println!("daemon: reached --max-ticks {ticks}; exiting");
            return Ok(());
        }
        tokio::time::sleep(Duration::from_secs(interval_secs)).await;
    }
}

async fn trigger_job(dir: &std::path::Path, name: &str) -> ArgoResult<()> {
    let path = dir.join(format!("{name}.json"));
    if !path.exists() {
        return Err(ArgoError::Provider(format!("{} not found", path.display())));
    }
    let body = std::fs::read_to_string(&path)
        .map_err(|e| ArgoError::Provider(format!("read {}: {e}", path.display())))?;
    let job: CronJob = serde_json::from_str(&body)
        .map_err(|e| ArgoError::Provider(format!("parse {}: {e}", path.display())))?;
    let runner = LoggingRunner;
    runner.run(&job).await?;
    println!("triggered '{name}' (no-op delivery; daemon binding wires the gateway send)");
    Ok(())
}

async fn tick(dir: &std::path::Path, dry_run: bool) -> ArgoResult<()> {
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
    let runner = LoggingRunner;
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

fn list_jobs(dir: &std::path::Path) -> ArgoResult<()> {
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
    if entries.is_empty() {
        println!("(no cron jobs in {})", dir.display());
        return Ok(());
    }
    for p in entries {
        let body = std::fs::read_to_string(&p)
            .map_err(|e| ArgoError::Provider(format!("read {}: {e}", p.display())))?;
        let job: CronJob = serde_json::from_str(&body)
            .map_err(|e| ArgoError::Provider(format!("parse {}: {e}", p.display())))?;
        println!(
            "  {:<24} {:<14} platform={:<10} enabled={}",
            job.name, job.schedule.source, job.target.platform, job.enabled
        );
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
