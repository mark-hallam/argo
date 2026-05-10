//! Worker identity create/show/list CLI.
//!
//! The persona / worker / role / duty tables (v12) and the
//! `worker.fs_root` column (v20) are now installed by the
//! versioned migration runner — this handler just calls
//! `SessionStore::migrate` via [`open_workspace_store`] and
//! relies on the ladder for schema state.
//!
//! Persistence + output types live in
//! [`super::worker_create_store`]; this module owns the CLI
//! orchestration, cron-job emission, and human-readable output.
use super::worker_create_store::{
    insert_worker, load_worker, open_workspace_store, DutyOut, WorkerOut,
};
use argo_cron::{CronExpr, CronJob, CronTarget};
use argo_types::{ArgoError, ArgoResult, TenantId};
use serde::Serialize;
use std::path::Path;

#[allow(clippy::too_many_arguments)]
pub async fn run(
    vertical: &str,
    persona: &str,
    charter: &str,
    builder: Option<&str>,
    voice_preamble: Option<&str>,
    duty_specs: &[String],
    fs_root: Option<&Path>,
    json: bool,
) -> ArgoResult<()> {
    let (store, tenant_id, cwd) = open_workspace_store().await?;
    let duties = parse_duties(duty_specs)?;
    let worker = insert_worker(
        &store,
        tenant_id,
        vertical,
        persona,
        charter,
        builder,
        voice_preamble,
        &duties,
        fs_root,
    )
    .await?;
    register_cron_duties(&cwd, tenant_id, &worker.id, &duties)?;
    print_worker(&worker, json)?;
    Ok(())
}

pub async fn show(worker_id: &str, json: bool) -> ArgoResult<()> {
    let (store, tenant_id, _cwd) = open_workspace_store().await?;
    let worker = load_worker(&store, tenant_id, worker_id).await?;
    print_worker(&worker, json)
}

pub async fn list(json: bool) -> ArgoResult<()> {
    let (store, tenant_id, _cwd) = open_workspace_store().await?;
    let ids: Vec<String> = sqlx::query_scalar(
        "SELECT id FROM worker WHERE tenant_id = ? ORDER BY created_at DESC, id ASC",
    )
    .bind(tenant_id.to_string())
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("list workers: {e}")))?;
    let mut workers = Vec::with_capacity(ids.len());
    for id in ids {
        workers.push(load_worker(&store, tenant_id, &id).await?);
    }
    if json {
        print_json(&workers)
    } else {
        if workers.is_empty() {
            println!("(no workers)");
            return Ok(());
        }
        println!("worker_id                  vertical        persona");
        for worker in workers {
            println!(
                "{:<26} {:<15} {}",
                short(&worker.id),
                worker.vertical_id,
                worker.persona.name
            );
        }
        Ok(())
    }
}

fn parse_duties(specs: &[String]) -> ArgoResult<Vec<DutyOut>> {
    specs
        .iter()
        .map(|spec| {
            let parts: Vec<&str> = spec.splitn(3, "::").collect();
            if parts.len() != 3 {
                return Err(ArgoError::Provider(
                    "duty must be '<cron>::<entry_action>::<description>'".into(),
                ));
            }
            CronExpr::parse(parts[0])
                .map_err(|e| ArgoError::Provider(format!("duty cron '{}': {e}", parts[0])))?;
            Ok(DutyOut {
                cron: parts[0].to_string(),
                entry_action: parts[1].to_string(),
                description: parts[2].to_string(),
            })
        })
        .collect()
}

fn register_cron_duties(
    cwd: &Path,
    tenant_id: TenantId,
    worker_id: &str,
    duties: &[DutyOut],
) -> ArgoResult<()> {
    if duties.is_empty() {
        return Ok(());
    }
    let dir = cwd.join("cron").join("jobs");
    std::fs::create_dir_all(&dir)
        .map_err(|e| ArgoError::Provider(format!("create_dir_all {}: {e}", dir.display())))?;
    for duty in duties {
        let cron = CronExpr::parse(&duty.cron)
            .map_err(|e| ArgoError::Provider(format!("duty cron '{}': {e}", duty.cron)))?;
        let name = format!("worker-{}-{}", short(worker_id), slug(&duty.entry_action));
        let job = CronJob::new(
            tenant_id,
            &name,
            cron,
            format!("Run worker {worker_id} duty {}: {}", duty.entry_action, duty.description),
            CronTarget {
                platform: "worker".into(),
                chat: worker_id.to_string(),
            },
        );
        let body = serde_json::to_string_pretty(&job)
            .map_err(|e| ArgoError::Provider(format!("serialize cron job: {e}")))?;
        let path = dir.join(format!("{name}.json"));
        std::fs::write(&path, body)
            .map_err(|e| ArgoError::Provider(format!("write {}: {e}", path.display())))?;
    }
    Ok(())
}

fn print_worker(worker: &WorkerOut, json: bool) -> ArgoResult<()> {
    if json {
        return print_json(worker);
    }
    println!("worker_id      {}", worker.id);
    println!("tenant_id      {}", worker.tenant_id);
    println!("vertical_id    {}", worker.vertical_id);
    println!("persona        {}", worker.persona.name);
    println!("voice          {}", worker.persona.voice_preamble);
    println!("charter        {}", worker.role.charter);
    if let Some(ref fs_root) = worker.fs_root {
        println!("fs_root        {}", fs_root);
    }
    if !worker.role.duties.is_empty() {
        println!("duties");
        for duty in &worker.role.duties {
            println!("  {} -> {} ({})", duty.cron, duty.entry_action, duty.description);
        }
    }
    Ok(())
}

fn print_json<T: Serialize>(value: &T) -> ArgoResult<()> {
    let body = serde_json::to_string_pretty(value)
        .map_err(|e| ArgoError::Provider(format!("serialize json: {e}")))?;
    println!("{body}");
    Ok(())
}

fn short(id: &str) -> String {
    if id.len() > 24 {
        id[..23].to_string()
    } else {
        id.to_string()
    }
}

fn slug(value: &str) -> String {
    value
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect()
}
