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
use uuid::Uuid;

use argo_types::{ArgoError, ArgoResult};

const WORKER_ARCHIVE_SCHEMA: &str = include_str!("../migrations/31__worker_archive.sql");
const AUDIT_TOMBSTONE_SCHEMA: &str =
    include_str!("../migrations/51__audit_worker_deleted_at.sql");

/// Default delete-grace window in days. Overridable per-process via
/// `ARGO_WORKER_DELETE_GRACE_DAYS`; the runtime parses any non-negative
/// integer (zero is allowed for ephemeral test envs).
pub const DEFAULT_DELETE_GRACE_DAYS: i64 = 30;

const MICROS_PER_DAY: i64 = 86_400_000_000;

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

/// Permanently purge an archived worker plus its per-tenant downstream
/// rows. Refuses while the worker is still live (call [`archive_worker`]
/// first) and refuses while the archive row is younger than the
/// configured grace window. The cascade and tombstone runs in one
/// transaction with the archive-row delete; partial state is impossible.
///
/// Cascade scope (per-tenant only — never crosses tenants):
/// * `memory_entry` rows tagged with this `worker_id` are DELETED.
/// * `audit_record` rows whose session points at this worker (via
///   `session.worker_id`) get the [`worker_deleted_at`] tombstone
///   stamped. Hashed columns (`parent_hash`, `self_hash`, `payload_json`)
///   are NOT touched, so chain tamper-evidence holds.
/// * The `worker_archive` row itself is DELETED.
///
/// For the compliance-driven force path that skips the grace gate,
/// see [`force_delete_worker`].
///
/// [`worker_deleted_at`]: ../../migrations/51__audit_worker_deleted_at.sql
pub async fn delete_worker(
    pool: &SqlitePool,
    worker_id: &str,
    tenant_id: &str,
) -> ArgoResult<()> {
    delete_with_grace(pool, worker_id, tenant_id, false).await
}

/// Compliance-flow purge that skips the grace window. `confirm` MUST be
/// `true`; the redundant flag is a defensive guard so misuse from a CLI
/// mis-routing or programmatic caller cannot accidentally bypass the
/// grace gate. Runs through the same SQL transaction as
/// [`delete_worker`], so the cascade contract is identical.
pub async fn force_delete_worker(
    pool: &SqlitePool,
    worker_id: &str,
    tenant_id: &str,
    confirm: bool,
) -> ArgoResult<()> {
    if !confirm {
        return Err(ArgoError::Schema(
            "force_delete_worker: confirm flag must be true".into(),
        ));
    }
    delete_with_grace(pool, worker_id, tenant_id, true).await
}

/// Resolve the active grace window. Reads
/// `ARGO_WORKER_DELETE_GRACE_DAYS` from the process env on every call
/// so an operator can flex the window without restarting the CLI.
/// Invalid / negative values fall back to [`DEFAULT_DELETE_GRACE_DAYS`].
pub fn delete_grace_days() -> i64 {
    std::env::var("ARGO_WORKER_DELETE_GRACE_DAYS")
        .ok()
        .and_then(|v| v.trim().parse::<i64>().ok())
        .filter(|n| *n >= 0)
        .unwrap_or(DEFAULT_DELETE_GRACE_DAYS)
}

async fn delete_with_grace(
    pool: &SqlitePool,
    worker_id: &str,
    tenant_id: &str,
    force: bool,
) -> ArgoResult<()> {
    ensure_schema(pool).await?;
    ensure_audit_tombstone_column(pool).await?;

    let worker_uuid = Uuid::parse_str(worker_id).map_err(|e| {
        ArgoError::Schema(format!(
            "delete_worker: invalid worker_id '{worker_id}': {e}"
        ))
    })?;
    let tenant_uuid = Uuid::parse_str(tenant_id).map_err(|e| {
        ArgoError::Schema(format!(
            "delete_worker: invalid tenant_id '{tenant_id}': {e}"
        ))
    })?;
    let worker_blob = worker_uuid.as_bytes().to_vec();
    let tenant_blob = tenant_uuid.as_bytes().to_vec();
    let now = Utc::now().timestamp_micros();

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| ArgoError::Provider(format!("delete_worker tx: {e}")))?;

    let live: Option<String> = sqlx::query_scalar(
        "SELECT id FROM worker WHERE id = ? AND tenant_id = ?",
    )
    .bind(worker_id)
    .bind(tenant_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| ArgoError::Provider(format!("delete_worker live lookup: {e}")))?;
    if live.is_some() {
        return Err(ArgoError::Schema(format!(
            "delete_worker: worker '{worker_id}' is still live in tenant '{tenant_id}'; \
             archive it first"
        )));
    }

    let archived_at: Option<i64> = sqlx::query_scalar(
        "SELECT archived_at FROM worker_archive WHERE id = ? AND tenant_id = ?",
    )
    .bind(worker_id)
    .bind(tenant_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| ArgoError::Provider(format!("delete_worker archive lookup: {e}")))?;
    let archived_at = archived_at.ok_or_else(|| {
        ArgoError::Provider(format!(
            "delete_worker: no archive row for '{worker_id}' in tenant '{tenant_id}'"
        ))
    })?;

    if !force {
        let grace_days = delete_grace_days();
        let grace_micros = grace_days.saturating_mul(MICROS_PER_DAY);
        let elapsed = now.saturating_sub(archived_at);
        if elapsed < grace_micros {
            let remaining_micros = grace_micros - elapsed;
            // Always present at least 1 day so the operator never sees
            // a confusing "0 days remaining" inside an active window.
            let remaining_days = ((remaining_micros + MICROS_PER_DAY - 1) / MICROS_PER_DAY).max(1);
            return Err(ArgoError::Provider(format!(
                "delete_worker: archive '{worker_id}' is inside the {grace_days}-day grace \
                 window; {remaining_days} day(s) remaining. Use force_delete_worker for \
                 compliance-driven purges."
            )));
        }
    }

    sqlx::query("DELETE FROM memory_entry WHERE worker_id = ? AND tenant_id = ?")
        .bind(&worker_blob)
        .bind(&tenant_blob)
        .execute(&mut *tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("delete_worker cascade memory: {e}")))?;

    sqlx::query(
        "UPDATE audit_record SET worker_deleted_at = ? \
         WHERE worker_deleted_at IS NULL \
           AND session_id IN (SELECT id FROM session WHERE worker_id = ?)",
    )
    .bind(now)
    .bind(&worker_blob)
    .execute(&mut *tx)
    .await
    .map_err(|e| ArgoError::Provider(format!("delete_worker tombstone audit: {e}")))?;

    sqlx::query("DELETE FROM worker_archive WHERE id = ? AND tenant_id = ?")
        .bind(worker_id)
        .bind(tenant_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("delete_worker drop archive: {e}")))?;

    tx.commit()
        .await
        .map_err(|e| ArgoError::Provider(format!("delete_worker commit: {e}")))?;
    Ok(())
}

/// Idempotently apply the `audit_record.worker_deleted_at` additive
/// migration. SQLite has no `ALTER TABLE … ADD COLUMN IF NOT EXISTS`,
/// so we probe via `PRAGMA table_info(audit_record)` first — mirrors
/// `v45::up` (additive `worker.archived_at`).
async fn ensure_audit_tombstone_column(pool: &SqlitePool) -> ArgoResult<()> {
    let cols = sqlx::query("PRAGMA table_info(audit_record)")
        .fetch_all(pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("audit_record table_info: {e}")))?;
    let exists = cols.iter().any(|row| {
        let name: String = row.get("name");
        name == "worker_deleted_at"
    });
    if exists {
        return Ok(());
    }
    for stmt in AUDIT_TOMBSTONE_SCHEMA.split_terminator(';') {
        let sql = stmt.trim();
        if sql.is_empty() {
            continue;
        }
        sqlx::query(sql)
            .execute(pool)
            .await
            .map_err(|e| ArgoError::Provider(format!("audit_worker_deleted_at: {e}")))?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "worker_archive_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "worker_archive_delete_tests.rs"]
mod delete_tests;
