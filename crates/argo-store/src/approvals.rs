//! First-class approval requests for the auto-approval gate (Stream
//! `claude-approval-request-store`).
//!
//! The current auto-approval flow decides Allow/Ask/Deny inline and
//! captures the result in the audit ledger. There is no durable
//! record of *pending* requests — who proposed them, who can grant
//! them, when they should time out. This module is that record.
//!
//! The state machine is:
//!
//! ```text
//!     created            decide_request                 expire_overdue
//!  -> Pending --(Granted | Denied)--> terminal     -> Expired (terminal)
//! ```
//!
//! `decide_request` is idempotent: a second call on a non-Pending row
//! is a no-op (no rows updated, no error). `expire_overdue` is the
//! primitive a future timeout sweep will call to flip Pending rows
//! whose `deadline_at` has passed; it records the row's own
//! `default_decision` as the `decision` column for downstream
//! consumers.
//!
//! Schema is owned by `migrations/v44.rs`; callers must run
//! `SessionStore::migrate` before use.
//!
//! # Example
//!
//! ```no_run
//! # use argo_store::approvals::{
//! #     create_request, decide_request, list_pending, ApprovalDecision,
//! #     DefaultDecision, NewApprovalRequest,
//! # };
//! # use argo_types::TenantId;
//! # async fn run(pool: &sqlx::SqlitePool) -> argo_types::ArgoResult<()> {
//! let tenant = TenantId::new();
//! let id = create_request(
//!     pool,
//!     &NewApprovalRequest {
//!         action_id: vec![0u8; 16],
//!         tenant_id: tenant,
//!         requested_by: "operator-1",
//!         deadline_at: 1_700_000_000,
//!         default_decision: DefaultDecision::Deny,
//!         draft_content: None,
//!     },
//! )
//! .await?;
//!
//! decide_request(pool, &id, "approver-1", ApprovalDecision::Approve).await?;
//! let _ = list_pending(pool, tenant, 100).await?;
//! # Ok(()) }
//! ```
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use uuid::Uuid;

pub use argo_types::ApprovalDecision;
use argo_types::{ArgoError, ArgoResult, TenantId};

#[path = "approvals_types.rs"]
mod approvals_types;

pub use approvals_types::{ApprovalRequest, ApprovalStatus, DefaultDecision, NewApprovalRequest};

/// Insert one row with `status = Pending`. Returns the assigned id so
/// the caller can correlate it with the action being gated.
pub async fn create_request(pool: &SqlitePool, new: &NewApprovalRequest<'_>) -> ArgoResult<String> {
    let id = Uuid::now_v7().to_string();
    let id_bytes = Uuid::parse_str(&id)
        .map_err(|e| ArgoError::Provider(format!("approval id parse: {e}")))?
        .as_bytes()
        .to_vec();
    let requested_at = Utc::now().timestamp_micros();
    sqlx::query(
        "INSERT INTO approval_request \
         (id, action_id, tenant_id, requested_at, requested_by, deadline_at, \
          default_decision, status, draft_content) \
         VALUES (?, ?, ?, ?, ?, ?, ?, 'pending', ?)",
    )
    .bind(&id_bytes)
    .bind(&new.action_id)
    .bind(new.tenant_id.0.as_bytes().as_slice())
    .bind(requested_at)
    .bind(new.requested_by)
    .bind(new.deadline_at)
    .bind(new.default_decision.as_str())
    .bind(new.draft_content)
    .execute(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("approval insert: {e}")))?;
    Ok(id)
}

/// Fetch one row by id. Returns `None` if no row exists.
pub async fn get_request(pool: &SqlitePool, id: &str) -> ArgoResult<Option<ApprovalRequest>> {
    let id_bytes = parse_id_bytes(id)?;
    let row: Option<RowTuple> = sqlx::query_as(SELECT_BY_ID)
        .bind(&id_bytes)
        .fetch_optional(pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("approval get: {e}")))?;
    row.map(into_request).transpose()
}

/// Apply a grantor's decision to a Pending row. Idempotent: a second
/// call on a non-Pending row is a silent no-op (returns `false`); the
/// first successful call returns `true`. The `WHERE status = 'pending'`
/// clause is what makes this safe under concurrent grantor races.
pub async fn decide_request(
    pool: &SqlitePool,
    id: &str,
    decided_by: &str,
    decision: ApprovalDecision,
) -> ArgoResult<bool> {
    let id_bytes = parse_id_bytes(id)?;
    let now = Utc::now().timestamp_micros();
    let rows = sqlx::query(
        "UPDATE approval_request \
         SET status = ?, decided_at = ?, decided_by = ?, decision = ? \
         WHERE id = ? AND status = 'pending'",
    )
    .bind(decision.as_store_str())
    .bind(now)
    .bind(decided_by)
    .bind(decision.as_store_str())
    .bind(&id_bytes)
    .execute(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("approval decide: {e}")))?;
    Ok(rows.rows_affected() > 0)
}

/// Pending rows for a tenant, ordered oldest-first so a grantor UI
/// can drain the queue in arrival order. Capped at `limit`.
pub async fn list_pending(
    pool: &SqlitePool,
    tenant_id: TenantId,
    limit: u32,
) -> ArgoResult<Vec<ApprovalRequest>> {
    let rows: Vec<RowTuple> = sqlx::query_as(SELECT_PENDING_BY_TENANT)
        .bind(tenant_id.0.as_bytes().as_slice())
        .bind(i64::from(limit))
        .fetch_all(pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("approval list_pending: {e}")))?;
    rows.into_iter().map(into_request).collect()
}

/// Flip every Pending row whose `deadline_at < now` to Expired,
/// recording the row's own `default_decision` as the resolved
/// `decision`. `decided_by` is set to a `system:` sentinel so audit
/// readers can distinguish sweep-driven expiry from human decisions.
/// Returns the count of rows updated.
pub async fn expire_overdue(
    pool: &SqlitePool,
    now_micros: i64,
    decided_by: &str,
) -> ArgoResult<u64> {
    let rows = sqlx::query(
        "UPDATE approval_request \
         SET status = 'expired', \
             decided_at = ?, \
             decided_by = ?, \
             decision = default_decision \
         WHERE status = 'pending' AND deadline_at < ?",
    )
    .bind(now_micros)
    .bind(decided_by)
    .bind(now_micros)
    .execute(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("approval expire_overdue: {e}")))?;
    Ok(rows.rows_affected())
}

/// Pending rows whose deadline is approaching within `window`.
///
/// Rows already past the deadline are intentionally excluded; the
/// expiry sweeper owns that terminal transition.
pub async fn list_pending_near_deadline(
    pool: &SqlitePool,
    window: std::time::Duration,
    now: DateTime<Utc>,
) -> ArgoResult<Vec<ApprovalRequest>> {
    let now_micros = now.timestamp_micros();
    let window_micros = i64::try_from(window.as_micros()).unwrap_or(i64::MAX);
    let upper = now_micros.saturating_add(window_micros);
    let rows: Vec<RowTuple> = sqlx::query_as(SELECT_PENDING_NEAR_DEADLINE)
        .bind(now_micros)
        .bind(upper)
        .fetch_all(pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("approval list_pending_near_deadline: {e}")))?;
    rows.into_iter().map(into_request).collect()
}

/// Stamp a Pending approval row after a reminder has been sent.
///
/// Idempotent: a second call, an already-decided row, or an unknown id
/// returns `false` and leaves the database unchanged.
pub async fn mark_escalation_sent(pool: &SqlitePool, request_id: &str) -> ArgoResult<bool> {
    mark_escalation_sent_at(pool, request_id, Utc::now().timestamp_micros()).await
}

/// Testable variant of [`mark_escalation_sent`] with caller-supplied
/// timestamp.
pub async fn mark_escalation_sent_at(
    pool: &SqlitePool,
    request_id: &str,
    now_micros: i64,
) -> ArgoResult<bool> {
    let id_bytes = parse_id_bytes(request_id)?;
    let rows = sqlx::query(
        "UPDATE approval_request \
         SET escalation_sent_at = ? \
         WHERE id = ? AND status = 'pending' AND escalation_sent_at IS NULL",
    )
    .bind(now_micros)
    .bind(&id_bytes)
    .execute(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("approval mark_escalation_sent: {e}")))?;
    Ok(rows.rows_affected() > 0)
}

fn parse_id_bytes(id: &str) -> ArgoResult<Vec<u8>> {
    Uuid::parse_str(id)
        .map(|u| u.as_bytes().to_vec())
        .map_err(|e| ArgoError::Provider(format!("approval id parse: {e}")))
}

const COLUMNS: &str = "id, action_id, tenant_id, requested_at, requested_by, \
                       deadline_at, default_decision, status, decided_at, \
                       decided_by, decision, escalation_sent_at, draft_content";

const SELECT_BY_ID: &str = "SELECT id, action_id, tenant_id, requested_at, requested_by, \
                            deadline_at, default_decision, status, decided_at, decided_by, \
                            decision, escalation_sent_at, draft_content \
                            FROM approval_request WHERE id = ?";

const SELECT_PENDING_BY_TENANT: &str =
    "SELECT id, action_id, tenant_id, requested_at, requested_by, \
     deadline_at, default_decision, status, decided_at, decided_by, decision, \
     escalation_sent_at, draft_content \
     FROM approval_request \
     WHERE tenant_id = ? AND status = 'pending' \
     ORDER BY requested_at ASC LIMIT ?";

const SELECT_PENDING_NEAR_DEADLINE: &str =
    "SELECT id, action_id, tenant_id, requested_at, requested_by, \
     deadline_at, default_decision, status, decided_at, decided_by, decision, \
     escalation_sent_at, draft_content \
     FROM approval_request \
     WHERE status = 'pending' \
       AND escalation_sent_at IS NULL \
       AND deadline_at > ? \
       AND deadline_at <= ? \
     ORDER BY deadline_at ASC, requested_at ASC";

#[allow(dead_code)]
const _ALL_COLUMNS_REFERENCE: &str = COLUMNS;

type RowTuple = (
    Vec<u8>,        // id
    Vec<u8>,        // action_id
    Vec<u8>,        // tenant_id
    i64,            // requested_at
    String,         // requested_by
    i64,            // deadline_at
    String,         // default_decision
    String,         // status
    Option<i64>,    // decided_at
    Option<String>, // decided_by
    Option<String>, // decision
    Option<i64>,    // escalation_sent_at
    Option<String>, // draft_content
);

fn into_request(t: RowTuple) -> ArgoResult<ApprovalRequest> {
    let id = Uuid::from_slice(&t.0)
        .map_err(|e| ArgoError::Provider(format!("approval id from db: {e}")))?
        .to_string();
    let tenant_uuid = Uuid::from_slice(&t.2)
        .map_err(|e| ArgoError::Provider(format!("approval tenant_id from db: {e}")))?;
    let tenant_id = TenantId(tenant_uuid);
    Ok(ApprovalRequest {
        id,
        action_id: t.1,
        tenant_id,
        requested_at: t.3,
        requested_by: t.4,
        deadline_at: t.5,
        default_decision: DefaultDecision::from_str_lossy(&t.6),
        status: ApprovalStatus::from_str_lossy(&t.7),
        decided_at: t.8,
        decided_by: t.9,
        decision: t.10,
        escalation_sent_at: t.11,
        draft_content: t.12,
    })
}

#[cfg(test)]
#[path = "approval_escalation_tests.rs"]
mod escalation_tests;
#[cfg(test)]
#[path = "approvals_tests.rs"]
mod tests;
