//! Tenant-scoped worker freeze persistence.
//!
//! This module is the store-only foundation for the PM kill switch. A
//! freeze is cooperative state: later runner, CLI, and gateway slices
//! decide how to consult it; this layer only toggles and reads the
//! `worker_freeze` table.
//!
//! # Example
//!
//! ```no_run
//! # async fn demo(pool: &sqlx::Pool<sqlx::Sqlite>) -> argo_types::ArgoResult<()> {
//! argo_store::worker_freeze::freeze(
//!     pool,
//!     "tenant-1",
//!     "worker-1",
//!     "principal:owner",
//!     "incident response",
//! )
//! .await?;
//! assert!(
//!     argo_store::worker_freeze::is_frozen(pool, "tenant-1", "worker-1")
//!         .await?
//!         .is_some()
//! );
//! # Ok(())
//! # }
//! ```

use argo_types::{ArgoError, ArgoResult};
use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite};

/// One row from the `worker_freeze` table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FreezeRecord {
    /// Tenant boundary for the worker.
    pub tenant_id: String,
    /// Worker whose cooperative execution is frozen.
    pub worker_id: String,
    /// RFC3339 timestamp when the current freeze was applied.
    pub frozen_at: String,
    /// Opaque actor string that applied the freeze.
    pub frozen_by: String,
    /// Operator-facing reason for the freeze.
    pub reason: String,
    /// RFC3339 timestamp when the worker was unfrozen, if inactive.
    pub unfrozen_at: Option<String>,
    /// Opaque actor string that lifted the freeze, if inactive.
    pub unfrozen_by: Option<String>,
}

/// Insert or refresh a freeze row for one `(tenant_id, worker_id)`.
/// Existing rows are toggled back to active and get a new
/// `frozen_at`, `frozen_by`, and `reason`.
///
/// # Errors
///
/// Returns [`ArgoError::Schema`] when a required string is empty and
/// [`ArgoError::Provider`] on SQLite failure.
pub async fn freeze(
    pool: &Pool<Sqlite>,
    tenant_id: &str,
    worker_id: &str,
    frozen_by: &str,
    reason: &str,
) -> ArgoResult<FreezeRecord> {
    validate_required("tenant_id", tenant_id)?;
    validate_required("worker_id", worker_id)?;
    validate_required("frozen_by", frozen_by)?;
    validate_required("reason", reason)?;

    let frozen_at = now_rfc3339();
    sqlx::query(
        "INSERT INTO worker_freeze \
            (tenant_id, worker_id, frozen_at, frozen_by, reason, unfrozen_at, unfrozen_by) \
         VALUES (?, ?, ?, ?, ?, NULL, NULL) \
         ON CONFLICT (tenant_id, worker_id) DO UPDATE SET \
            frozen_at = excluded.frozen_at, \
            frozen_by = excluded.frozen_by, \
            reason = excluded.reason, \
            unfrozen_at = NULL, \
            unfrozen_by = NULL",
    )
    .bind(tenant_id)
    .bind(worker_id)
    .bind(&frozen_at)
    .bind(frozen_by)
    .bind(reason)
    .execute(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("worker_freeze freeze: {e}")))?;

    is_frozen(pool, tenant_id, worker_id)
        .await?
        .ok_or_else(|| ArgoError::Invariant("worker_freeze upsert returned no active row".into()))
}

/// Mark one active freeze row as unfrozen. Returns `true` only when an
/// active row was changed; already-unfrozen or unknown workers return
/// `false`.
///
/// # Errors
///
/// Returns [`ArgoError::Schema`] when a required string is empty and
/// [`ArgoError::Provider`] on SQLite failure.
pub async fn unfreeze(
    pool: &Pool<Sqlite>,
    tenant_id: &str,
    worker_id: &str,
    unfrozen_by: &str,
) -> ArgoResult<bool> {
    validate_required("tenant_id", tenant_id)?;
    validate_required("worker_id", worker_id)?;
    validate_required("unfrozen_by", unfrozen_by)?;

    let unfrozen_at = now_rfc3339();
    let res = sqlx::query(
        "UPDATE worker_freeze \
         SET unfrozen_at = ?, unfrozen_by = ? \
         WHERE tenant_id = ? AND worker_id = ? AND unfrozen_at IS NULL",
    )
    .bind(&unfrozen_at)
    .bind(unfrozen_by)
    .bind(tenant_id)
    .bind(worker_id)
    .execute(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("worker_freeze unfreeze: {e}")))?;
    Ok(res.rows_affected() > 0)
}

/// Return the active freeze row for a worker, or `None` when the
/// worker is not currently frozen in that tenant.
///
/// # Errors
///
/// Returns [`ArgoError::Schema`] when a required string is empty and
/// [`ArgoError::Provider`] on SQLite failure.
pub async fn is_frozen(
    pool: &Pool<Sqlite>,
    tenant_id: &str,
    worker_id: &str,
) -> ArgoResult<Option<FreezeRecord>> {
    validate_required("tenant_id", tenant_id)?;
    validate_required("worker_id", worker_id)?;

    let row = sqlx::query_as::<_, FreezeTuple>(
        "SELECT tenant_id, worker_id, frozen_at, frozen_by, reason, unfrozen_at, unfrozen_by \
         FROM worker_freeze \
         WHERE tenant_id = ? AND worker_id = ? AND unfrozen_at IS NULL",
    )
    .bind(tenant_id)
    .bind(worker_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("worker_freeze is_frozen: {e}")))?;
    Ok(row.map(tuple_to_record))
}

/// List active freeze rows for one tenant, oldest freeze first.
///
/// # Errors
///
/// Returns [`ArgoError::Schema`] when `tenant_id` is empty and
/// [`ArgoError::Provider`] on SQLite failure.
pub async fn list_frozen(pool: &Pool<Sqlite>, tenant_id: &str) -> ArgoResult<Vec<FreezeRecord>> {
    validate_required("tenant_id", tenant_id)?;

    let rows = sqlx::query_as::<_, FreezeTuple>(
        "SELECT tenant_id, worker_id, frozen_at, frozen_by, reason, unfrozen_at, unfrozen_by \
         FROM worker_freeze \
         WHERE tenant_id = ? AND unfrozen_at IS NULL \
         ORDER BY frozen_at ASC, worker_id ASC",
    )
    .bind(tenant_id)
    .fetch_all(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("worker_freeze list_frozen: {e}")))?;
    Ok(rows.into_iter().map(tuple_to_record).collect())
}

type FreezeTuple = (
    String,
    String,
    String,
    String,
    String,
    Option<String>,
    Option<String>,
);

fn tuple_to_record(row: FreezeTuple) -> FreezeRecord {
    let (tenant_id, worker_id, frozen_at, frozen_by, reason, unfrozen_at, unfrozen_by) = row;
    FreezeRecord {
        tenant_id,
        worker_id,
        frozen_at,
        frozen_by,
        reason,
        unfrozen_at,
        unfrozen_by,
    }
}

fn validate_required(name: &str, value: &str) -> ArgoResult<()> {
    if value.trim().is_empty() {
        return Err(ArgoError::Schema(format!(
            "worker_freeze: {name} must not be empty"
        )));
    }
    Ok(())
}

fn now_rfc3339() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Micros, true)
}

#[cfg(test)]
#[path = "worker_freeze_tests.rs"]
mod tests;
