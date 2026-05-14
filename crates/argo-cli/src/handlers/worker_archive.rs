//! `argo worker archive/rehydrate/archive list` (Stream
//! `claude-worker-archive-rehydrate`, mission claim 1).
//!
//! See `argo_store::worker_archive` for the schema and SQL contract.
//! This handler owns the [`Worker`] ↔ JSON serde dance and the
//! [`IdentityStore`] re-insert on rehydrate.
//!
//! [`Worker`]: argo_identity::Worker
//! [`IdentityStore`]: argo_identity::IdentityStore

use crate::home::{db_path, ensure_dir, resolve_home};
use argo_identity::{IdentityStore, Worker, WorkerId};
use argo_store::worker_archive::{self, ArchiveRow};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use chrono::{DateTime, Utc};
use serde_json::json;
use std::path::PathBuf;
use std::str::FromStr;
use uuid::Uuid;

/// `argo worker archive <id> --tenant <t> [--reason <txt>] [--principal <p>] [--json]`.
pub async fn archive(
    tenant: &str,
    worker_id: &str,
    reason: Option<&str>,
    principal: Option<&str>,
    json: bool,
) -> ArgoResult<()> {
    let tenant_id = parse_tenant(tenant)?;
    let parsed = parse_worker(worker_id)?;
    let owned = open_identity_store().await?;
    let identities = IdentityStore::new(&owned.store);
    let worker = identities.get_worker(tenant_id, parsed).await?.ok_or_else(|| {
        ArgoError::Provider(format!(
            "worker {worker_id} not found in tenant {tenant}"
        ))
    })?;

    let frozen = serde_json::to_string(&worker)
        .map_err(|e| ArgoError::Provider(format!("freeze worker: {e}")))?;
    let resolved_principal = principal
        .map(str::to_string)
        .or_else(|| std::env::var("USER").ok());

    worker_archive::archive_worker(
        owned.store.pool(),
        &worker.id.to_string(),
        &worker.tenant_id.to_string(),
        resolved_principal.as_deref(),
        reason,
        &frozen,
    )
    .await?;

    if json {
        let payload = json!({
            "ok": true,
            "worker_id": worker.id.to_string(),
            "tenant_id": worker.tenant_id.to_string(),
            "archived_by_principal": resolved_principal,
            "reason": reason,
            "frozen_bytes": frozen.len(),
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".into())
        );
    } else {
        let reason_line = reason
            .map(|r| format!(" ({r})"))
            .unwrap_or_default();
        println!(
            "worker {} archived{}",
            worker.id,
            reason_line
        );
    }
    Ok(())
}

/// `argo worker rehydrate <id> --tenant <t> [--json]`.
pub async fn rehydrate(tenant: &str, worker_id: &str, json: bool) -> ArgoResult<()> {
    let tenant_id = parse_tenant(tenant)?;
    let parsed = parse_worker(worker_id)?;
    let owned = open_identity_store().await?;

    let frozen = worker_archive::rehydrate_worker(
        owned.store.pool(),
        &parsed.to_string(),
        &tenant_id.to_string(),
    )
    .await?;

    let mut worker: Worker = serde_json::from_str(&frozen)
        .map_err(|e| ArgoError::Provider(format!("thaw worker: {e}")))?;
    if worker.id != parsed || worker.tenant_id != tenant_id {
        return Err(ArgoError::Provider(format!(
            "frozen Worker ids drifted (expected {parsed}/{tenant_id}, got {}/{})",
            worker.id, worker.tenant_id
        )));
    }

    let identities = IdentityStore::new(&owned.store);
    if let Err(err) = identities.create_worker(&worker).await {
        // Recreating from snapshot failed AFTER the archive row was
        // already deleted. Restore the archive so the operator can
        // retry without losing data — matches the "no silent loss"
        // contract called out in the brief.
        worker.id = parsed;
        let restore = serde_json::to_string(&worker)
            .map_err(|e| ArgoError::Provider(format!("re-freeze worker: {e}")))?;
        let _ = sqlx::query(
            "INSERT INTO worker_archive
                (id, tenant_id, archived_at, archived_by_principal, reason, frozen_json)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(parsed.to_string())
        .bind(tenant_id.to_string())
        .bind(Utc::now().timestamp_micros())
        .bind(Option::<String>::None)
        .bind(Some("rehydrate failed; restored automatically"))
        .bind(restore)
        .execute(owned.store.pool())
        .await;
        return Err(err);
    }

    if json {
        let payload = json!({
            "ok": true,
            "worker_id": worker.id.to_string(),
            "tenant_id": worker.tenant_id.to_string(),
            "rehydrated": true,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".into())
        );
    } else {
        println!("worker {} rehydrated", worker.id);
    }
    Ok(())
}

/// `argo worker delete <id> --tenant <t> [--force] [--json]`. Calls
/// through to `worker_archive::delete_worker` (or `force_delete_worker`
/// when `--force` is set) — see that module for the cascade contract.
pub async fn delete(
    tenant: &str,
    worker_id: &str,
    force: bool,
    json: bool,
) -> ArgoResult<()> {
    let tenant_id = parse_tenant(tenant)?;
    let parsed = parse_worker(worker_id)?;
    let owned = open_identity_store().await?;

    if force {
        worker_archive::force_delete_worker(
            owned.store.pool(),
            &parsed.to_string(),
            &tenant_id.to_string(),
            true,
        )
        .await?;
    } else {
        worker_archive::delete_worker(
            owned.store.pool(),
            &parsed.to_string(),
            &tenant_id.to_string(),
        )
        .await?;
    }

    if json {
        let payload = json!({
            "ok": true,
            "worker_id": parsed.to_string(),
            "tenant_id": tenant_id.to_string(),
            "deleted": true,
            "force": force,
            "grace_days": worker_archive::delete_grace_days(),
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".into())
        );
    } else if force {
        println!("worker {parsed} deleted (force)");
    } else {
        println!("worker {parsed} deleted");
    }
    Ok(())
}

/// `argo worker archive list --tenant <t> [--json]`.
pub async fn list(tenant: &str, json: bool) -> ArgoResult<()> {
    let tenant_id = parse_tenant(tenant)?;
    let owned = open_identity_store().await?;
    let rows = worker_archive::list_archived(
        owned.store.pool(),
        &tenant_id.to_string(),
    )
    .await?;
    if json {
        let payload = json!({
            "ok": true,
            "tenant_id": tenant_id.to_string(),
            "count": rows.len(),
            "rows": rows
                .iter()
                .map(|r| {
                    json!({
                        "worker_id": r.id,
                        "archived_at": r.archived_at,
                        "archived_at_rfc3339": format_iso(r.archived_at),
                        "archived_by_principal": r.archived_by_principal,
                        "reason": r.reason,
                    })
                })
                .collect::<Vec<_>>(),
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".into())
        );
    } else if rows.is_empty() {
        println!("(no archived workers)");
    } else {
        print_table(&rows);
    }
    Ok(())
}

fn print_table(rows: &[ArchiveRow]) {
    println!("{:<36}  {:<25}  {:<20}  reason", "worker_id", "archived_at", "principal");
    for row in rows {
        let ts = format_iso(row.archived_at);
        let principal = row.archived_by_principal.as_deref().unwrap_or("");
        let reason = row.reason.as_deref().unwrap_or("");
        println!("{:<36}  {ts:<25}  {principal:<20}  {reason}", row.id);
    }
}

fn format_iso(micros: i64) -> String {
    DateTime::<Utc>::from_timestamp_micros(micros)
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_else(|| micros.to_string())
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

struct OwnedIdentityStore {
    store: SessionStore,
    _home: PathBuf,
}
