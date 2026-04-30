//! `argo cron` — list / create / delete / trigger jobs in the
//! workspace's `cron/jobs/` directory.
use crate::cli::CronCmd;
use argo_cron::{CronExpr, CronJob, CronTarget};
use argo_types::{ArgoError, ArgoResult, TenantId};
use std::path::PathBuf;

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
        CronCmd::Trigger { name } => {
            println!("argo cron trigger {name}: requires a running daemon (separate PR).");
            Ok(())
        }
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
