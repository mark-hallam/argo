//! `argo memory dream-now <worker>` — on-demand Dreams API summary.
//!
//! Walks the previous calendar month's recent rows for one worker
//! namespace, calls the Dreams API, persists the curated summary
//! as a memory row tagged `source=dreams_api`. Refuses if the
//! worker has any open anomaly events. Honours the per-worker
//! daily cap via `ARGO_DREAMS_DAILY_CAP_USD_MICROS`.
//!
//! Brief: `claude-dreams-memory-summarize` (V2.0 Integration A).
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_identity::{MemoryNs, MemoryScope, WorkerId};
use argo_memory::dreams_backend::{run_dream_for_worker, EnvDreamsCaller};
use argo_memory::MemoryStore;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use chrono::Utc;
use sqlx::Row;
use std::str::FromStr;
use std::sync::Arc;
use uuid::Uuid;

pub async fn run(worker: &str, scope: &str, json: bool) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let db = db_path(&home);
    let store = Arc::new(SessionStore::open(&db).await?);
    store.migrate().await?;

    let parsed_scope = parse_scope(scope)?;
    let (worker_id, tenant_id) = resolve_worker(&store, worker).await?;
    let memory = MemoryStore::new(Arc::clone(&store));

    let workspace_id = crate::home::cwd_workspace(&store).await?;
    let ns = MemoryNs::new(tenant_id, worker_id, parsed_scope);

    let caller = EnvDreamsCaller::from_env()?;
    let entry = run_dream_for_worker(
        &memory,
        workspace_id,
        &home,
        &ns,
        Utc::now(),
        &caller,
    )
    .await?;

    if json {
        let payload = serde_json::json!({
            "entry_id": entry.id.to_string(),
            "worker_id": worker_id.0.to_string(),
            "scope": scope,
            "summary": entry.body,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload)
                .map_err(|e| ArgoError::Provider(format!("dream-now json: {e}")))?
        );
    } else {
        println!("dreamt {entry_id} for worker {worker_id} (scope={scope})", entry_id = entry.id);
        println!("---");
        println!("{}", entry.body);
    }
    Ok(())
}

fn parse_scope(s: &str) -> ArgoResult<MemoryScope> {
    match s {
        "worker_self" | "worker-self" => Ok(MemoryScope::WorkerSelf),
        "role" => Ok(MemoryScope::Role),
        "shared_team" | "shared-team" => Ok(MemoryScope::SharedTeam),
        other => Err(ArgoError::Provider(format!(
            "memory scope must be worker_self|role|shared_team, got {other}"
        ))),
    }
}

async fn resolve_worker(
    store: &SessionStore,
    worker: &str,
) -> ArgoResult<(WorkerId, TenantId)> {
    let row = sqlx::query("SELECT id, tenant_id FROM worker WHERE id = ? LIMIT 1")
        .bind(worker)
        .fetch_optional(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("dream-now worker lookup: {e}")))?
        .ok_or_else(|| ArgoError::Provider(format!("worker not found: {worker}")))?;
    let id_str: String = row
        .try_get("id")
        .map_err(|e| ArgoError::Provider(format!("worker.id: {e}")))?;
    let tenant_str: String = row
        .try_get("tenant_id")
        .map_err(|e| ArgoError::Provider(format!("worker.tenant_id: {e}")))?;
    let id_uuid = Uuid::from_str(&id_str)
        .map_err(|e| ArgoError::Provider(format!("worker.id parse: {e}")))?;
    let tenant_uuid = Uuid::from_str(&tenant_str)
        .map_err(|e| ArgoError::Provider(format!("worker.tenant_id parse: {e}")))?;
    Ok((WorkerId(id_uuid), TenantId(tenant_uuid)))
}
