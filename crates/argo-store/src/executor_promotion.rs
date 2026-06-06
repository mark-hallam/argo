//! Tenant-scoped per-action execution-graduation persistence
//! (UF Phase-B · B3, DEC-037).
//!
//! A hired teammate is draft-only at birth: every generated action is
//! `executor = "human"`. A **promotion** row graduates exactly one
//! `(tenant, worker, action_type)` off that human default onto a real
//! executor (MCP today), so dispatch can carry the action out for real
//! after approval. The grant is per-action and opt-in: the **absence**
//! of a row means "still draft-only", which is the v0 behaviour every
//! workspace ships with.
//!
//! This module is the store-only foundation. It does NOT decide whether
//! a promotion is *honoured* at dispatch time — that is gated, dormant
//! and default-OFF, by the CLI resolve path
//! (`ARGO_EXECUTION_GRADUATION`). Here we only durably record and read
//! the grant, always within a tenant boundary.
//!
//! # Example
//!
//! ```no_run
//! # async fn demo(pool: &sqlx::Pool<sqlx::Sqlite>) -> argo_types::ArgoResult<()> {
//! use argo_store::executor_promotion::{self, PromotionGrant};
//! executor_promotion::promote(
//!     pool,
//!     &PromotionGrant {
//!         tenant_id: "tenant-1".into(),
//!         worker_id: "worker-1".into(),
//!         action_type: "send_email".into(),
//!         executor_kind: "mcp".into(),
//!         mcp_server: Some("gmail".into()),
//!         mcp_tool: Some("drafts.send".into()),
//!         promoted_by: "principal:owner".into(),
//!     },
//! )
//! .await?;
//! let row = executor_promotion::get(pool, "tenant-1", "worker-1", "send_email").await?;
//! assert!(row.is_some());
//! # Ok(())
//! # }
//! ```

use argo_types::{ArgoError, ArgoResult};
use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite};

/// One persisted promotion row. The `(tenant_id, worker_id,
/// action_type)` triple is the primary key, so at most one promotion
/// exists per action per worker per tenant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromotionRecord {
    /// Tenant boundary for the worker.
    pub tenant_id: String,
    /// Worker whose action is graduated (string `WorkerId`).
    pub worker_id: String,
    /// Manifest `action_type` being graduated off the human default.
    pub action_type: String,
    /// Executor kind the action is promoted to. `mcp` is the only
    /// promotable kind in v1.
    pub executor_kind: String,
    /// MCP server name (required when `executor_kind = "mcp"`).
    pub mcp_server: Option<String>,
    /// MCP tool name within the server.
    pub mcp_tool: Option<String>,
    /// Principal id that granted the promotion.
    pub promoted_by: String,
    /// RFC3339 timestamp the promotion was granted.
    pub promoted_at: String,
}

/// Input to [`promote`] — the grant minus the server-stamped
/// `promoted_at`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromotionGrant {
    /// Tenant boundary for the worker.
    pub tenant_id: String,
    /// Worker whose action is graduated (string `WorkerId`).
    pub worker_id: String,
    /// Manifest `action_type` being graduated.
    pub action_type: String,
    /// Executor kind the action is promoted to (`mcp` in v1).
    pub executor_kind: String,
    /// MCP server name (required when `executor_kind = "mcp"`).
    pub mcp_server: Option<String>,
    /// MCP tool name within the server.
    pub mcp_tool: Option<String>,
    /// Principal id that granted the promotion.
    pub promoted_by: String,
}

/// The only executor kind a teammate may be graduated to in v1.
pub const PROMOTABLE_KIND_MCP: &str = "mcp";

/// Insert or refresh a promotion row for one
/// `(tenant_id, worker_id, action_type)`. Re-promoting the same action
/// overwrites the prior grant (new executor binding + `promoted_by` +
/// `promoted_at`). Returns the persisted [`PromotionRecord`].
///
/// `executor_kind` must be [`PROMOTABLE_KIND_MCP`]; `mcp_server` and
/// `mcp_tool` are required for that kind (the dispatch path cannot
/// resolve a server/tool otherwise). The caller is responsible for the
/// probation + RBAC preconditions — this layer only persists.
///
/// # Errors
///
/// Returns [`ArgoError::Schema`] when a required string is empty, the
/// kind is not promotable, or the MCP binding is incomplete; returns
/// [`ArgoError::Provider`] on SQLite failure.
pub async fn promote(pool: &Pool<Sqlite>, grant: &PromotionGrant) -> ArgoResult<PromotionRecord> {
    validate_required("tenant_id", &grant.tenant_id)?;
    validate_required("worker_id", &grant.worker_id)?;
    validate_required("action_type", &grant.action_type)?;
    validate_required("promoted_by", &grant.promoted_by)?;
    if grant.executor_kind != PROMOTABLE_KIND_MCP {
        return Err(ArgoError::Schema(format!(
            "executor_promotion: only '{PROMOTABLE_KIND_MCP}' is promotable in v1, \
             got '{}'",
            grant.executor_kind
        )));
    }
    let server = require_binding("mcp_server", grant.mcp_server.as_deref())?;
    let tool = require_binding("mcp_tool", grant.mcp_tool.as_deref())?;

    let promoted_at = now_rfc3339();
    sqlx::query(
        "INSERT INTO executor_promotion \
            (tenant_id, worker_id, action_type, executor_kind, mcp_server, mcp_tool, \
             promoted_by, promoted_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?) \
         ON CONFLICT (tenant_id, worker_id, action_type) DO UPDATE SET \
            executor_kind = excluded.executor_kind, \
            mcp_server = excluded.mcp_server, \
            mcp_tool = excluded.mcp_tool, \
            promoted_by = excluded.promoted_by, \
            promoted_at = excluded.promoted_at",
    )
    .bind(&grant.tenant_id)
    .bind(&grant.worker_id)
    .bind(&grant.action_type)
    .bind(&grant.executor_kind)
    .bind(server)
    .bind(tool)
    .bind(&grant.promoted_by)
    .bind(&promoted_at)
    .execute(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("executor_promotion promote: {e}")))?;

    get(pool, &grant.tenant_id, &grant.worker_id, &grant.action_type)
        .await?
        .ok_or_else(|| ArgoError::Invariant("executor_promotion upsert returned no row".into()))
}

/// Fetch the promotion row for one `(tenant, worker, action_type)`, or
/// `None` when the action is still draft-only in that tenant.
///
/// The `tenant_id` is always bound, so a promotion can never resolve
/// across a tenant boundary.
///
/// # Errors
///
/// Returns [`ArgoError::Schema`] when a required string is empty and
/// [`ArgoError::Provider`] on SQLite failure.
pub async fn get(
    pool: &Pool<Sqlite>,
    tenant_id: &str,
    worker_id: &str,
    action_type: &str,
) -> ArgoResult<Option<PromotionRecord>> {
    validate_required("tenant_id", tenant_id)?;
    validate_required("worker_id", worker_id)?;
    validate_required("action_type", action_type)?;

    let row = sqlx::query_as::<_, PromotionTuple>(
        "SELECT tenant_id, worker_id, action_type, executor_kind, mcp_server, mcp_tool, \
                promoted_by, promoted_at \
         FROM executor_promotion \
         WHERE tenant_id = ? AND worker_id = ? AND action_type = ?",
    )
    .bind(tenant_id)
    .bind(worker_id)
    .bind(action_type)
    .fetch_optional(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("executor_promotion get: {e}")))?;
    Ok(row.map(tuple_to_record))
}

/// List every promotion for one worker, newest grant first. Tenant- and
/// worker-scoped — never returns another tenant's rows.
///
/// # Errors
///
/// Returns [`ArgoError::Schema`] when a required string is empty and
/// [`ArgoError::Provider`] on SQLite failure.
pub async fn list_for_worker(
    pool: &Pool<Sqlite>,
    tenant_id: &str,
    worker_id: &str,
) -> ArgoResult<Vec<PromotionRecord>> {
    validate_required("tenant_id", tenant_id)?;
    validate_required("worker_id", worker_id)?;

    let rows = sqlx::query_as::<_, PromotionTuple>(
        "SELECT tenant_id, worker_id, action_type, executor_kind, mcp_server, mcp_tool, \
                promoted_by, promoted_at \
         FROM executor_promotion \
         WHERE tenant_id = ? AND worker_id = ? \
         ORDER BY promoted_at DESC, action_type ASC",
    )
    .bind(tenant_id)
    .bind(worker_id)
    .fetch_all(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("executor_promotion list_for_worker: {e}")))?;
    Ok(rows.into_iter().map(tuple_to_record).collect())
}

/// Revoke (demote) a promoted action back to draft-only. Returns `true`
/// when a row was removed, `false` when none existed. The reverse of
/// [`promote`]; keeps the graduation reversible per DEC-036/037.
///
/// # Errors
///
/// Returns [`ArgoError::Schema`] when a required string is empty and
/// [`ArgoError::Provider`] on SQLite failure.
pub async fn revoke(
    pool: &Pool<Sqlite>,
    tenant_id: &str,
    worker_id: &str,
    action_type: &str,
) -> ArgoResult<bool> {
    validate_required("tenant_id", tenant_id)?;
    validate_required("worker_id", worker_id)?;
    validate_required("action_type", action_type)?;

    let res = sqlx::query(
        "DELETE FROM executor_promotion \
         WHERE tenant_id = ? AND worker_id = ? AND action_type = ?",
    )
    .bind(tenant_id)
    .bind(worker_id)
    .bind(action_type)
    .execute(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("executor_promotion revoke: {e}")))?;
    Ok(res.rows_affected() > 0)
}

type PromotionTuple = (
    String,
    String,
    String,
    String,
    Option<String>,
    Option<String>,
    String,
    String,
);

fn tuple_to_record(row: PromotionTuple) -> PromotionRecord {
    let (tenant_id, worker_id, action_type, executor_kind, mcp_server, mcp_tool, promoted_by, promoted_at) =
        row;
    PromotionRecord {
        tenant_id,
        worker_id,
        action_type,
        executor_kind,
        mcp_server,
        mcp_tool,
        promoted_by,
        promoted_at,
    }
}

fn validate_required(name: &str, value: &str) -> ArgoResult<()> {
    if value.trim().is_empty() {
        return Err(ArgoError::Schema(format!(
            "executor_promotion: {name} must not be empty"
        )));
    }
    Ok(())
}

fn require_binding<'a>(name: &str, value: Option<&'a str>) -> ArgoResult<&'a str> {
    match value {
        Some(v) if !v.trim().is_empty() => Ok(v),
        _ => Err(ArgoError::Schema(format!(
            "executor_promotion: {name} is required when executor_kind = 'mcp'"
        ))),
    }
}

fn now_rfc3339() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

#[cfg(test)]
#[path = "executor_promotion_tests.rs"]
mod tests;
