//! On-disk CRUD for cron-job JSON files under `cron/jobs/`.
//!
//! Sibling of [`super::cron`]; extracted to keep the parent module
//! below the 500-LOC ceiling. Visibility is `pub(super)` so the
//! parent dispatcher in `cron.rs` can call these but no other
//! handler module can.
use argo_cron::{CronExpr, CronJob, CronTarget};
use argo_types::{ArgoError, ArgoResult, TenantId};
use std::path::PathBuf;

pub(super) fn list_jobs(dir: &std::path::Path, json: bool, format: &str) -> ArgoResult<()> {
    let mut entries: Vec<PathBuf> = Vec::new();
    for entry in std::fs::read_dir(dir)
        .map_err(|e| ArgoError::Io(format!("read_dir {}: {e}", dir.display())))?
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

pub(super) fn create_job(
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
        .map_err(|e| ArgoError::Io(format!("write {}: {e}", path.display())))?;
    println!("created {} (schedule = '{}')", path.display(), schedule);
    Ok(())
}

pub(super) fn delete_job(dir: &std::path::Path, name: &str) -> ArgoResult<()> {
    let path = dir.join(format!("{name}.json"));
    if !path.exists() {
        return Err(ArgoError::Provider(format!("{} not found", path.display())));
    }
    std::fs::remove_file(&path)
        .map_err(|e| ArgoError::Io(format!("remove {}: {e}", path.display())))?;
    println!("deleted {}", path.display());
    Ok(())
}
