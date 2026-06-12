//! Tenant-scoped capability-request persistence (connect-on-demand
//! backend, Stream `tg1-emit-capability-request-on-fenced-tool-deny`).
//!
//! When a teammate's run hits a tool/connector its capability fence
//! denies, the runtime persists ONE pending row per
//! `(tenant_id, worker_id, tool_or_connector)` here so the deny stops
//! being silent: the SPA's grant card reads pending rows via
//! `GET /api/v1/workers/<id>/capability-requests` and offers
//! connect-on-demand. This layer only records + lists; granting a
//! request (advancing `status`) is a later slice (tg3). Emission is
//! observe-only — recording a request never changes dispatch behavior.
//!
//! # Example
//!
//! ```no_run
//! # async fn demo(pool: &sqlx::Pool<sqlx::Sqlite>) -> argo_types::ArgoResult<()> {
//! let row = argo_store::capability_request::record_pending(
//!     pool,
//!     "tenant-1",
//!     "worker-1",
//!     "salesforce",
//!     "write",
//!     Some("run-1"),
//! )
//! .await?;
//! assert_eq!(row.status, "pending");
//! assert_eq!(
//!     argo_store::capability_request::list_pending(pool, "tenant-1", "worker-1")
//!         .await?
//!         .len(),
//!     1
//! );
//! # Ok(())
//! # }
//! ```

use argo_types::{ArgoError, ArgoResult};
use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite};
use uuid::Uuid;

/// One row from the `capability_request` table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityRequest {
    /// Stable request id (UUID string).
    pub id: String,
    /// Tenant boundary for the worker.
    pub tenant_id: String,
    /// Worker whose run hit the fence deny.
    pub worker_id: String,
    /// The tool or connector name the fence refused.
    pub tool_or_connector: String,
    /// Access the worker attempted: `read` or `write`.
    pub requested_access: String,
    /// The run that triggered the deny, when one is known.
    pub run_id: Option<String>,
    /// Lifecycle status: `pending` | `granted` | `dismissed`.
    pub status: String,
    /// RFC3339 timestamp when the request was first recorded.
    pub created_utc: String,
    /// RFC3339 timestamp of the most recent deny for this `(worker,
    /// tool)` — re-denies bump this rather than inserting a new row.
    pub last_seen_utc: String,
}

/// Record (or refresh) a pending capability request for one
/// `(tenant_id, worker_id, tool_or_connector)`. A re-deny of the same
/// tool bumps `last_seen_utc` (and the latest `requested_access` /
/// `run_id`) on the existing row instead of inserting a duplicate;
/// `status` and `created_utc` are preserved across re-denies.
///
/// # Errors
///
/// Returns [`ArgoError::Schema`] when a required string is empty or
/// `requested_access` is not `read`/`write`, and [`ArgoError::Provider`]
/// on SQLite failure.
pub async fn record_pending(
    pool: &Pool<Sqlite>,
    tenant_id: &str,
    worker_id: &str,
    tool_or_connector: &str,
    requested_access: &str,
    run_id: Option<&str>,
) -> ArgoResult<CapabilityRequest> {
    validate_required("tenant_id", tenant_id)?;
    validate_required("worker_id", worker_id)?;
    validate_required("tool_or_connector", tool_or_connector)?;
    validate_access(requested_access)?;

    let now = now_rfc3339();
    let id = Uuid::now_v7().to_string();
    sqlx::query(
        "INSERT INTO capability_request \
            (id, tenant_id, worker_id, tool_or_connector, requested_access, \
             run_id, status, created_utc, last_seen_utc) \
         VALUES (?, ?, ?, ?, ?, ?, 'pending', ?, ?) \
         ON CONFLICT (tenant_id, worker_id, tool_or_connector) DO UPDATE SET \
            requested_access = excluded.requested_access, \
            run_id = excluded.run_id, \
            last_seen_utc = excluded.last_seen_utc",
    )
    .bind(&id)
    .bind(tenant_id)
    .bind(worker_id)
    .bind(tool_or_connector)
    .bind(requested_access)
    .bind(run_id)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("capability_request record: {e}")))?;

    get(pool, tenant_id, worker_id, tool_or_connector)
        .await?
        .ok_or_else(|| ArgoError::Invariant("capability_request upsert returned no row".into()))
}

/// Return the single request row for one `(tenant, worker, tool)`, or
/// `None` when no request exists for that key.
///
/// # Errors
///
/// Returns [`ArgoError::Provider`] on SQLite failure.
pub async fn get(
    pool: &Pool<Sqlite>,
    tenant_id: &str,
    worker_id: &str,
    tool_or_connector: &str,
) -> ArgoResult<Option<CapabilityRequest>> {
    let row = sqlx::query_as::<_, RequestTuple>(
        "SELECT id, tenant_id, worker_id, tool_or_connector, requested_access, \
                run_id, status, created_utc, last_seen_utc \
         FROM capability_request \
         WHERE tenant_id = ? AND worker_id = ? AND tool_or_connector = ?",
    )
    .bind(tenant_id)
    .bind(worker_id)
    .bind(tool_or_connector)
    .fetch_optional(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("capability_request get: {e}")))?;
    Ok(row.map(tuple_to_record))
}

/// List pending requests for one `(tenant, worker)`, most-recently-seen
/// first — exactly what the SPA grant card polls.
///
/// # Errors
///
/// Returns [`ArgoError::Schema`] when a required string is empty and
/// [`ArgoError::Provider`] on SQLite failure.
pub async fn list_pending(
    pool: &Pool<Sqlite>,
    tenant_id: &str,
    worker_id: &str,
) -> ArgoResult<Vec<CapabilityRequest>> {
    validate_required("tenant_id", tenant_id)?;
    validate_required("worker_id", worker_id)?;

    let rows = sqlx::query_as::<_, RequestTuple>(
        "SELECT id, tenant_id, worker_id, tool_or_connector, requested_access, \
                run_id, status, created_utc, last_seen_utc \
         FROM capability_request \
         WHERE tenant_id = ? AND worker_id = ? AND status = 'pending' \
         ORDER BY last_seen_utc DESC, tool_or_connector ASC",
    )
    .bind(tenant_id)
    .bind(worker_id)
    .fetch_all(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("capability_request list_pending: {e}")))?;
    Ok(rows.into_iter().map(tuple_to_record).collect())
}

type RequestTuple = (
    String,
    String,
    String,
    String,
    String,
    Option<String>,
    String,
    String,
    String,
);

fn tuple_to_record(row: RequestTuple) -> CapabilityRequest {
    let (
        id,
        tenant_id,
        worker_id,
        tool_or_connector,
        requested_access,
        run_id,
        status,
        created_utc,
        last_seen_utc,
    ) = row;
    CapabilityRequest {
        id,
        tenant_id,
        worker_id,
        tool_or_connector,
        requested_access,
        run_id,
        status,
        created_utc,
        last_seen_utc,
    }
}

fn validate_required(name: &str, value: &str) -> ArgoResult<()> {
    if value.trim().is_empty() {
        return Err(ArgoError::Schema(format!(
            "capability_request: {name} must not be empty"
        )));
    }
    Ok(())
}

fn validate_access(access: &str) -> ArgoResult<()> {
    if access == "read" || access == "write" {
        Ok(())
    } else {
        Err(ArgoError::Schema(format!(
            "capability_request: requested_access must be 'read' or 'write', got '{access}'"
        )))
    }
}

fn now_rfc3339() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Micros, true)
}

#[cfg(test)]
#[path = "capability_request_tests.rs"]
mod tests;
