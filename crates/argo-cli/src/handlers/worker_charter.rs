//! Stream `claude-worker-charter-storage` (mission claim 2).
//!
//! Per-instance charter (markdown body) and KPIs (free-form JSON)
//! live on the worker row alongside persona, role, channels, and
//! auto-approval policy. This handler ships `argo worker charter
//! set/get/clear` over those columns.

use crate::home::{db_path, ensure_dir, resolve_home};
use argo_identity::{
    IdentityStore, MAX_CHARTER_BYTES, MAX_KPIS_BYTES, WorkerId,
};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use uuid::Uuid;

/// `argo worker charter set <id> --tenant <t> --file <md> [--kpis <json>]`.
pub async fn set(
    tenant: &str,
    worker_id: &str,
    file: &Path,
    kpis_path: Option<&Path>,
    json: bool,
) -> ArgoResult<()> {
    let tenant_id = parse_tenant(tenant)?;
    let worker = parse_worker(worker_id)?;
    let charter_md = read_charter_file(file)?;
    let kpis = match kpis_path {
        Some(path) => Some(read_kpis_file(path)?),
        None => None,
    };
    let identities = open_identity_store().await?;
    identities
        .update_charter(tenant_id, worker, Some(&charter_md), kpis.as_ref())
        .await?;
    print_set_outcome(worker_id, &charter_md, kpis.as_ref(), json);
    Ok(())
}

/// `argo worker charter get <id> --tenant <t> [--md|--json]`.
pub async fn get(tenant: &str, worker_id: &str, md: bool, json: bool) -> ArgoResult<()> {
    let tenant_id = parse_tenant(tenant)?;
    let worker = parse_worker(worker_id)?;
    let identities = open_identity_store().await?;
    let row = identities
        .get_worker(tenant_id, worker)
        .await?
        .ok_or_else(|| {
            ArgoError::Provider(format!(
                "worker {worker_id} not found in tenant {tenant}"
            ))
        })?;
    if json {
        let payload = serde_json::json!({
            "ok": true,
            "worker_id": worker_id,
            "charter": row.charter,
            "kpis": row.kpis,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".into())
        );
    } else {
        // `md` is accepted for symmetry; markdown is the default
        // human-readable view either way.
        let _ = md;
        match row.charter.as_deref() {
            Some(body) => println!("{body}"),
            None => println!("(no charter set)"),
        }
        if let Some(kpis) = row.kpis.as_ref() {
            println!();
            println!("## KPIs");
            println!(
                "{}",
                serde_json::to_string_pretty(kpis).unwrap_or_else(|_| "{}".into())
            );
        }
    }
    Ok(())
}

/// `argo worker charter clear <id> --tenant <t>`.
pub async fn clear(tenant: &str, worker_id: &str, json: bool) -> ArgoResult<()> {
    let tenant_id = parse_tenant(tenant)?;
    let worker = parse_worker(worker_id)?;
    let identities = open_identity_store().await?;
    identities.update_charter(tenant_id, worker, None, None).await?;
    if json {
        let payload = serde_json::json!({
            "ok": true,
            "worker_id": worker_id,
            "cleared": true,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".into())
        );
    } else {
        println!("worker {worker_id}: charter + kpis cleared");
    }
    Ok(())
}

pub(crate) fn read_charter_file(path: &Path) -> ArgoResult<String> {
    let body = std::fs::read_to_string(path).map_err(|e| {
        ArgoError::Provider(format!(
            "could not read charter at {}: {e}",
            path.display()
        ))
    })?;
    enforce_charter_size(&body)?;
    Ok(body)
}

pub(crate) fn read_kpis_file(path: &Path) -> ArgoResult<Value> {
    let body = std::fs::read_to_string(path).map_err(|e| {
        ArgoError::Provider(format!("could not read kpis at {}: {e}", path.display()))
    })?;
    let value: Value = serde_json::from_str(&body).map_err(|e| {
        ArgoError::Provider(format!(
            "kpis JSON at {} is not valid JSON: {e}",
            path.display()
        ))
    })?;
    enforce_kpis_size(&value)?;
    Ok(value)
}

pub(crate) fn enforce_charter_size(body: &str) -> ArgoResult<()> {
    if body.len() > MAX_CHARTER_BYTES {
        return Err(ArgoError::Schema(format!(
            "charter exceeds {MAX_CHARTER_BYTES} bytes (got {})",
            body.len()
        )));
    }
    Ok(())
}

pub(crate) fn enforce_kpis_size(value: &Value) -> ArgoResult<()> {
    let serialised = serde_json::to_string(value)
        .map_err(|e| ArgoError::Provider(format!("serialise kpis: {e}")))?;
    if serialised.len() > MAX_KPIS_BYTES {
        return Err(ArgoError::Schema(format!(
            "kpis exceed {MAX_KPIS_BYTES} bytes (got {})",
            serialised.len()
        )));
    }
    Ok(())
}

fn parse_tenant(tenant: &str) -> ArgoResult<TenantId> {
    Uuid::parse_str(tenant)
        .map(TenantId)
        .map_err(|e| ArgoError::Schema(format!("invalid tenant id '{tenant}': {e}")))
}

fn parse_worker(worker_id: &str) -> ArgoResult<WorkerId> {
    WorkerId::from_str(worker_id)
        .map_err(|e| ArgoError::Schema(format!("invalid worker id '{worker_id}': {e}")))
}

async fn open_identity_store() -> ArgoResult<OwnedIdentityStore> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(db_path(&home)).await?;
    store.migrate().await?;
    Ok(OwnedIdentityStore { store, _home: home })
}

/// Owns the underlying `SessionStore` so the borrow that
/// [`IdentityStore`] holds stays valid for the whole CLI invocation.
pub(crate) struct OwnedIdentityStore {
    store: SessionStore,
    _home: PathBuf,
}

impl OwnedIdentityStore {
    pub(crate) async fn update_charter(
        &self,
        tenant_id: TenantId,
        worker_id: WorkerId,
        charter: Option<&str>,
        kpis: Option<&Value>,
    ) -> ArgoResult<()> {
        IdentityStore::new(&self.store)
            .update_charter(tenant_id, worker_id, charter, kpis)
            .await
    }

    pub(crate) async fn get_worker(
        &self,
        tenant_id: TenantId,
        worker_id: WorkerId,
    ) -> ArgoResult<Option<argo_identity::Worker>> {
        IdentityStore::new(&self.store).get_worker(tenant_id, worker_id).await
    }
}

fn print_set_outcome(worker_id: &str, charter: &str, kpis: Option<&Value>, json: bool) {
    if json {
        let payload = serde_json::json!({
            "ok": true,
            "worker_id": worker_id,
            "charter_bytes": charter.len(),
            "kpis_bytes": kpis
                .and_then(|v| serde_json::to_string(v).ok())
                .map(|s| s.len()),
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".into())
        );
    } else {
        println!("worker {worker_id}: charter updated ({} bytes)", charter.len());
        if let Some(value) = kpis {
            let bytes = serde_json::to_string(value).map(|s| s.len()).unwrap_or(0);
            println!("  kpis: {bytes} bytes");
        }
    }
}
