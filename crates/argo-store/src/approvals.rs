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
//!     },
//! )
//! .await?;
//!
//! decide_request(pool, &id, "approver-1", ApprovalDecision::Granted).await?;
//! let _ = list_pending(pool, tenant, 100).await?;
//! # Ok(()) }
//! ```
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

use argo_types::{ArgoError, ArgoResult, TenantId};

/// Lifecycle states an [`ApprovalRequest`] can occupy. Stored as a
/// lower-case string in the `status` column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalStatus {
    /// Awaiting a human grant/deny decision.
    Pending,
    /// A grantor approved the request.
    Granted,
    /// A grantor denied the request.
    Denied,
    /// `expire_overdue` flipped the row past its deadline; the
    /// recorded decision matches `default_decision`.
    Expired,
}

impl ApprovalStatus {
    /// Lower-case label used in the `status` column.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Granted => "granted",
            Self::Denied => "denied",
            Self::Expired => "expired",
        }
    }

    /// Parse the column label back to a status. Unknown values fall
    /// back to [`ApprovalStatus::Pending`] so a forward-rolled DB
    /// doesn't crash older readers.
    #[must_use]
    pub fn from_str_lossy(s: &str) -> Self {
        match s {
            "granted" => Self::Granted,
            "denied" => Self::Denied,
            "expired" => Self::Expired,
            _ => Self::Pending,
        }
    }
}

/// Default decision recorded at request-creation time. The future
/// timeout sweep applies one of these to a Pending row whose
/// deadline has passed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DefaultDecision {
    /// Treat the unattended request as approved.
    Allow,
    /// Treat the unattended request as still requiring a human;
    /// the sweep records `expired` but downstream consumers may
    /// re-prompt.
    Ask,
    /// Treat the unattended request as denied (the safe default).
    Deny,
}

impl DefaultDecision {
    /// Lower-case label used in the `default_decision` column.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::Ask => "ask",
            Self::Deny => "deny",
        }
    }

    /// Parse the column label back to a default decision. Unknown
    /// values fall back to [`DefaultDecision::Deny`] (safe default).
    #[must_use]
    pub fn from_str_lossy(s: &str) -> Self {
        match s {
            "allow" => Self::Allow,
            "ask" => Self::Ask,
            _ => Self::Deny,
        }
    }
}

/// Decision recorded by [`decide_request`] — a human grantor's
/// resolution of a Pending request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalDecision {
    /// Grantor approved the request.
    Granted,
    /// Grantor denied the request.
    Denied,
}

impl ApprovalDecision {
    /// The terminal [`ApprovalStatus`] this decision lands the row in.
    #[must_use]
    pub fn status(self) -> ApprovalStatus {
        match self {
            Self::Granted => ApprovalStatus::Granted,
            Self::Denied => ApprovalStatus::Denied,
        }
    }

    /// Lower-case label used in the `decision` column.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Granted => "granted",
            Self::Denied => "denied",
        }
    }
}

/// One persisted approval-request row. Matches `approval_request`
/// 1:1 so callers can `SELECT *` and round-trip cleanly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalRequest {
    /// UUIDv7 id (BLOB in SQLite, hex-encoded in JSON).
    pub id: String,
    /// Action this request is gating. Opaque to the store.
    pub action_id: Vec<u8>,
    /// Owning tenant.
    pub tenant_id: TenantId,
    /// Requested-at micros since epoch.
    pub requested_at: i64,
    /// Free-text identifier for the requester (e.g. operator email).
    pub requested_by: String,
    /// Deadline-at micros since epoch — the timeout sweep flips rows
    /// with `deadline_at < now` AND `status = Pending` to Expired.
    pub deadline_at: i64,
    /// The decision the timeout sweep applies on expiry.
    pub default_decision: DefaultDecision,
    /// Lifecycle state.
    pub status: ApprovalStatus,
    /// Decided-at micros since epoch (Granted/Denied/Expired only).
    pub decided_at: Option<i64>,
    /// Free-text identifier for the grantor (Granted/Denied only;
    /// Expired sets this to a sentinel like "system:timeout-sweep").
    pub decided_by: Option<String>,
    /// Final decision label (`granted`/`denied` for human decisions;
    /// the `default_decision` label for Expired rows).
    pub decision: Option<String>,
    /// Reminder/escalation sent-at micros since epoch. `None` means a
    /// pending row is still eligible for the no-reply reminder loop.
    pub escalation_sent_at: Option<i64>,
}

/// Insert payload for [`create_request`]. Lets callers populate every
/// column without dragging the auto-managed `id`, `requested_at`,
/// `status`, and decision-tracking columns through their type
/// signatures.
#[derive(Debug, Clone)]
pub struct NewApprovalRequest<'a> {
    /// Action this request is gating.
    pub action_id: Vec<u8>,
    /// Owning tenant.
    pub tenant_id: TenantId,
    /// Free-text identifier for the requester.
    pub requested_by: &'a str,
    /// Deadline-at micros since epoch.
    pub deadline_at: i64,
    /// The decision the timeout sweep applies on expiry.
    pub default_decision: DefaultDecision,
}

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
          default_decision, status) \
         VALUES (?, ?, ?, ?, ?, ?, ?, 'pending')",
    )
    .bind(&id_bytes)
    .bind(&new.action_id)
    .bind(new.tenant_id.0.as_bytes().as_slice())
    .bind(requested_at)
    .bind(new.requested_by)
    .bind(new.deadline_at)
    .bind(new.default_decision.as_str())
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
    .bind(decision.status().as_str())
    .bind(now)
    .bind(decided_by)
    .bind(decision.as_str())
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
                       decided_by, decision, escalation_sent_at";

const SELECT_BY_ID: &str = "SELECT id, action_id, tenant_id, requested_at, requested_by, \
                            deadline_at, default_decision, status, decided_at, decided_by, \
                            decision, escalation_sent_at FROM approval_request WHERE id = ?";

const SELECT_PENDING_BY_TENANT: &str =
    "SELECT id, action_id, tenant_id, requested_at, requested_by, \
     deadline_at, default_decision, status, decided_at, decided_by, decision, escalation_sent_at \
     FROM approval_request \
     WHERE tenant_id = ? AND status = 'pending' \
     ORDER BY requested_at ASC LIMIT ?";

const SELECT_PENDING_NEAR_DEADLINE: &str =
    "SELECT id, action_id, tenant_id, requested_at, requested_by, \
     deadline_at, default_decision, status, decided_at, decided_by, decision, escalation_sent_at \
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
    })
}

#[cfg(test)]
#[path = "approval_escalation_tests.rs"]
mod escalation_tests;
#[cfg(test)]
#[path = "approvals_tests.rs"]
mod tests;
