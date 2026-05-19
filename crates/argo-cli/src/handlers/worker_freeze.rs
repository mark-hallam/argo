//! `argo worker freeze/unfreeze` kill-switch CLI.

use argo_capability::CapabilityEngine;
use argo_store::worker_freeze::{self as freeze_store, FreezeRecord};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, Capability, Principal, TenantId};
use serde::Serialize;
use std::path::Path;

/// `argo worker freeze <id> --reason <text> [--json]`.
pub async fn freeze(worker_id: &str, reason: &str, json: bool) -> ArgoResult<()> {
    let (store, tenant_id, cwd) = super::worker_create_store::open_workspace_store().await?;
    let principal = local_principal();
    let row = freeze_with_role(
        &store, tenant_id, worker_id, reason, "owner", &cwd, &principal,
    )
    .await?;
    if json {
        print_json(&FreezeOut::from(row))
    } else {
        println!(
            "Worker {} frozen: {} (by {} at {})",
            row.worker_id, row.reason, row.frozen_by, row.frozen_at
        );
        Ok(())
    }
}

/// `argo worker unfreeze <id> [--json]`.
pub async fn unfreeze(worker_id: &str, json: bool) -> ArgoResult<()> {
    let (store, tenant_id, cwd) = super::worker_create_store::open_workspace_store().await?;
    let principal = local_principal();
    let unfrozen =
        unfreeze_with_role(&store, tenant_id, worker_id, "owner", &cwd, &principal).await?;
    let out = UnfreezeOut {
        ok: true,
        tenant_id: tenant_id.to_string(),
        worker_id: worker_id.to_string(),
        unfrozen,
        unfrozen_by: principal_label(&principal),
    };
    if json {
        print_json(&out)
    } else if unfrozen {
        println!("Worker {worker_id} unfrozen by {}", out.unfrozen_by);
        Ok(())
    } else {
        println!("Worker {worker_id} was not frozen");
        Ok(())
    }
}

async fn freeze_with_role(
    store: &SessionStore,
    tenant_id: TenantId,
    worker_id: &str,
    reason: &str,
    role_name: &str,
    cwd: &Path,
    principal: &Principal,
) -> ArgoResult<FreezeRecord> {
    require_worker_freeze(cwd, principal, role_name)?;
    let actor = principal_label(principal);
    freeze_store::freeze(
        store.pool(),
        &tenant_id.to_string(),
        worker_id,
        &actor,
        reason,
    )
    .await
}

async fn unfreeze_with_role(
    store: &SessionStore,
    tenant_id: TenantId,
    worker_id: &str,
    role_name: &str,
    cwd: &Path,
    principal: &Principal,
) -> ArgoResult<bool> {
    require_worker_freeze(cwd, principal, role_name)?;
    let actor = principal_label(principal);
    freeze_store::unfreeze(store.pool(), &tenant_id.to_string(), worker_id, &actor).await
}

fn require_worker_freeze(cwd: &Path, principal: &Principal, role_name: &str) -> ArgoResult<()> {
    let engine = CapabilityEngine::with_workspace_defaults(cwd);
    engine.bind(principal.clone(), role_name.to_string())?;
    engine.check(principal, &Capability::WorkerFreeze)
}

fn local_principal() -> Principal {
    let os_user = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "operator".into());
    Principal::Local { os_user }
}

fn principal_label(principal: &Principal) -> String {
    match principal {
        Principal::Local { os_user } => os_user.clone(),
        Principal::Gateway { adapter, identity } => format!("{adapter}:{identity}"),
        Principal::Acp { harness, identity } => format!("{harness}:{identity}"),
        Principal::Forge => "forge".into(),
    }
}

fn print_json<T: Serialize>(value: &T) -> ArgoResult<()> {
    let body = serde_json::to_string_pretty(value)
        .map_err(|e| ArgoError::Provider(format!("serialize worker freeze output: {e}")))?;
    println!("{body}");
    Ok(())
}

#[derive(Serialize)]
struct FreezeOut {
    ok: bool,
    tenant_id: String,
    worker_id: String,
    frozen_at: String,
    frozen_by: String,
    reason: String,
}

impl From<FreezeRecord> for FreezeOut {
    fn from(row: FreezeRecord) -> Self {
        Self {
            ok: true,
            tenant_id: row.tenant_id,
            worker_id: row.worker_id,
            frozen_at: row.frozen_at,
            frozen_by: row.frozen_by,
            reason: row.reason,
        }
    }
}

#[derive(Serialize)]
struct UnfreezeOut {
    ok: bool,
    tenant_id: String,
    worker_id: String,
    unfrozen: bool,
    unfrozen_by: String,
}

#[cfg(test)]
#[path = "worker_freeze_tests.rs"]
mod tests;
