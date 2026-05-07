//! Worker archive (sleep / wake) store layer
//! (Stream `claude-worker-archive-rehydrate`, mission claim 1).
//!
//! `worker_archive` is a frozen-snapshot table. Putting a worker to
//! sleep stores a single row with the full serialised [`Worker`]
//! JSON plus light metadata (`archived_at`, `archived_by_principal`,
//! `reason`) and removes the live rows from `worker`, `persona`,
//! `role` (worker-scoped), and `duty` in one transaction. Audit and
//! handoff rows reference the [`WorkerId`] string and are
//! deliberately left in place so historical lineage survives.
//!
//! [`Worker`]: argo_identity::Worker
//! [`WorkerId`]: argo_identity::WorkerId
//!
//! The CLI handler at `argo-cli::handlers::worker_archive` owns the
//! serde dance (worker → JSON on archive; JSON → worker → re-insert
//! via [`IdentityStore`] on rehydrate). Keeping that here would
//! force `argo-store → argo-identity`, a circular dep.
//!
//! [`IdentityStore`]: argo_identity::IdentityStore

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};

use argo_types::{ArgoError, ArgoResult};

const WORKER_ARCHIVE_SCHEMA: &str = include_str!("../migrations/31__worker_archive.sql");

/// One archived worker snapshot. Mirrors the `worker_archive` table
/// columns 1:1.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchiveRow {
    /// Worker id this snapshot was taken for. Stable across archive
    /// → rehydrate.
    pub id: String,
    /// Tenant boundary.
    pub tenant_id: String,
    /// Micros since epoch when the row was written.
    pub archived_at: i64,
    /// Optional operator principal who initiated the archive.
    pub archived_by_principal: Option<String>,
    /// Optional free-text reason.
    pub reason: Option<String>,
    /// Frozen `Worker` JSON.
    pub frozen_json: String,
}

/// Apply the archive schema. Idempotent — safe to call from every
/// entry point so callers don't have to coordinate with the global
/// migration runner.
pub async fn ensure_schema(pool: &SqlitePool) -> ArgoResult<()> {
    for stmt in WORKER_ARCHIVE_SCHEMA.split_terminator(';') {
        let sql = stmt.trim();
        if sql.is_empty() {
            continue;
        }
        sqlx::query(sql)
            .execute(pool)
            .await
            .map_err(|e| ArgoError::Provider(format!("worker_archive schema: {e}")))?;
    }
    Ok(())
}

/// Snapshot one live worker into `worker_archive` and remove its
/// rows from `worker`, `persona`, `role` (worker-scoped), and
/// `duty`. All five writes commit atomically — partial state is
/// impossible.
///
/// Errors when no live worker matches `(worker_id, tenant_id)` or
/// when an archive row already exists for `worker_id`.
pub async fn archive_worker(
    pool: &SqlitePool,
    worker_id: &str,
    tenant_id: &str,
    archived_by_principal: Option<&str>,
    reason: Option<&str>,
    frozen_json: &str,
) -> ArgoResult<()> {
    ensure_schema(pool).await?;
    let now = Utc::now().timestamp_micros();

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| ArgoError::Provider(format!("worker_archive tx: {e}")))?;

    let live = sqlx::query_scalar::<_, String>("SELECT id FROM worker WHERE id = ? AND tenant_id = ?")
        .bind(worker_id)
        .bind(tenant_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("worker_archive lookup: {e}")))?;
    if live.is_none() {
        return Err(ArgoError::Provider(format!(
            "archive_worker: no live worker with id '{worker_id}' for tenant '{tenant_id}'"
        )));
    }

    let already: Option<String> =
        sqlx::query_scalar("SELECT id FROM worker_archive WHERE id = ?")
            .bind(worker_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("worker_archive existing: {e}")))?;
    if already.is_some() {
        return Err(ArgoError::Provider(format!(
            "archive_worker: worker '{worker_id}' is already archived"
        )));
    }

    sqlx::query(
        "INSERT INTO worker_archive
            (id, tenant_id, archived_at, archived_by_principal, reason, frozen_json)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(worker_id)
    .bind(tenant_id)
    .bind(now)
    .bind(archived_by_principal)
    .bind(reason)
    .bind(frozen_json)
    .execute(&mut *tx)
    .await
    .map_err(|e| ArgoError::Provider(format!("worker_archive insert: {e}")))?;

    sqlx::query("DELETE FROM duty WHERE worker_id = ?")
        .bind(worker_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("worker_archive delete duty: {e}")))?;
    sqlx::query("DELETE FROM role WHERE worker_id = ?")
        .bind(worker_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("worker_archive delete role: {e}")))?;
    sqlx::query("DELETE FROM persona WHERE worker_id = ?")
        .bind(worker_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("worker_archive delete persona: {e}")))?;
    sqlx::query("DELETE FROM worker WHERE id = ? AND tenant_id = ?")
        .bind(worker_id)
        .bind(tenant_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("worker_archive delete worker: {e}")))?;

    tx.commit()
        .await
        .map_err(|e| ArgoError::Provider(format!("worker_archive commit: {e}")))?;
    Ok(())
}

/// Read the frozen snapshot for `worker_id` and remove the archive
/// row in one transaction. Returns the JSON body so the caller can
/// deserialise + re-insert via [`IdentityStore`].
///
/// Errors when no archive row exists, or when a live worker row
/// already occupies that id (would violate the `worker.id` PK on
/// re-insert).
///
/// [`IdentityStore`]: argo_identity::IdentityStore
pub async fn rehydrate_worker(
    pool: &SqlitePool,
    worker_id: &str,
    tenant_id: &str,
) -> ArgoResult<String> {
    ensure_schema(pool).await?;

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| ArgoError::Provider(format!("rehydrate tx: {e}")))?;

    let live: Option<String> = sqlx::query_scalar("SELECT id FROM worker WHERE id = ?")
        .bind(worker_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("rehydrate lookup live: {e}")))?;
    if live.is_some() {
        return Err(ArgoError::Provider(format!(
            "rehydrate_worker: live worker '{worker_id}' already exists"
        )));
    }

    let frozen: Option<String> = sqlx::query_scalar(
        "SELECT frozen_json FROM worker_archive WHERE id = ? AND tenant_id = ?",
    )
    .bind(worker_id)
    .bind(tenant_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| ArgoError::Provider(format!("rehydrate lookup archive: {e}")))?;
    let frozen = frozen.ok_or_else(|| {
        ArgoError::Provider(format!(
            "rehydrate_worker: no archive row for '{worker_id}' in tenant '{tenant_id}'"
        ))
    })?;

    sqlx::query("DELETE FROM worker_archive WHERE id = ? AND tenant_id = ?")
        .bind(worker_id)
        .bind(tenant_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("rehydrate delete archive: {e}")))?;

    tx.commit()
        .await
        .map_err(|e| ArgoError::Provider(format!("rehydrate commit: {e}")))?;
    Ok(frozen)
}

/// List archived workers for one tenant, newest-first.
pub async fn list_archived(
    pool: &SqlitePool,
    tenant_id: &str,
) -> ArgoResult<Vec<ArchiveRow>> {
    ensure_schema(pool).await?;
    let rows = sqlx::query(
        "SELECT id, tenant_id, archived_at, archived_by_principal, reason, frozen_json
         FROM worker_archive
         WHERE tenant_id = ?
         ORDER BY archived_at DESC, id ASC",
    )
    .bind(tenant_id)
    .fetch_all(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("worker_archive list: {e}")))?;
    Ok(rows
        .into_iter()
        .map(|row| ArchiveRow {
            id: row.get("id"),
            tenant_id: row.get("tenant_id"),
            archived_at: row.get("archived_at"),
            archived_by_principal: row.get("archived_by_principal"),
            reason: row.get("reason"),
            frozen_json: row.get("frozen_json"),
        })
        .collect())
}

#[cfg(test)]
#[path = "worker_archive_tests.rs"]
mod tests;
