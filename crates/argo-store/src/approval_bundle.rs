//! Server-defined approval bundles (Stream
//! `mission-approval-bundle-ui`). A bundle row groups N
//! `ActionRecord` ids (opaque strings owned by `worker_actions`)
//! under one operator decision; per-member transitions still happen
//! through the worker-actions state machine in the gateway route
//! handler. If 7 of 8 members are pending when the bundle is
//! approved, those 7 flip to `approved` and the 8th — already
//! decided individually — stays at its terminal state. Schema:
//! `migrations/v60.rs` (filename slot 71).
//!
//! # Example
//!
//! ```no_run
//! # use argo_store::approval_bundle::{
//! #     create_bundle, decide_bundle, BundleDecision, NewBundle,
//! # };
//! # use argo_types::TenantId;
//! # async fn run(pool: &sqlx::SqlitePool) -> argo_types::ArgoResult<()> {
//! let id = create_bundle(pool, &NewBundle {
//!     tenant_id: TenantId::new(),
//!     label: "8 social-media replies",
//!     member_approval_ids: vec!["a1".into(), "a2".into()],
//! }).await?;
//! decide_bundle(pool, &id, "operator-1", BundleDecision::Approve).await?;
//! # Ok(()) }
//! ```
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

use argo_types::{ArgoError, ArgoResult, TenantId};

/// Lifecycle states a bundle can occupy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BundleStatus {
    /// Awaiting an operator's collective decision.
    Pending,
    /// Operator approved the whole group.
    Approved,
    /// Operator refused the whole group.
    Refused,
}

impl BundleStatus {
    /// Lower-case label stored in the `status` column.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Approved => "approved",
            Self::Refused => "refused",
        }
    }

    /// Parse the column label back to a status; unknown → Pending.
    #[must_use]
    pub fn from_str_lossy(s: &str) -> Self {
        match s {
            "approved" => Self::Approved,
            "refused" => Self::Refused,
            _ => Self::Pending,
        }
    }
}

/// Operator decision recorded by [`decide_bundle`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BundleDecision {
    /// Approve every still-pending member.
    Approve,
    /// Refuse every still-pending member.
    Refuse,
}

impl BundleDecision {
    /// The terminal [`BundleStatus`] this decision lands the row in.
    #[must_use]
    pub fn status(self) -> BundleStatus {
        match self {
            Self::Approve => BundleStatus::Approved,
            Self::Refuse => BundleStatus::Refused,
        }
    }

    /// Lower-case label used in the `decision` column.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Approve => "approve",
            Self::Refuse => "refuse",
        }
    }
}

/// One persisted bundle row paired with its ordered member ids.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalBundle {
    /// UUIDv7 id; rendered as hex in JSON.
    pub id: String,
    /// Owning tenant.
    pub tenant_id: TenantId,
    /// Short human label — what the operator sees on the BundleCard.
    pub label: String,
    /// Created-at micros since epoch.
    pub created_at: i64,
    /// Lifecycle state.
    pub status: BundleStatus,
    /// Decided-at micros since epoch (Approved / Refused only).
    pub decided_at: Option<i64>,
    /// Free-text identifier for the operator who decided.
    pub decided_by: Option<String>,
    /// Final decision label (`approve` / `refuse`).
    pub decision: Option<String>,
    /// Member approval-ids in worker-supplied order.
    pub members: Vec<String>,
}

/// Insert payload for [`create_bundle`].
#[derive(Debug, Clone)]
pub struct NewBundle<'a> {
    /// Owning tenant.
    pub tenant_id: TenantId,
    /// Short human label.
    pub label: &'a str,
    /// Member approval-ids in the order the worker emitted them.
    pub member_approval_ids: Vec<String>,
}

/// Insert one bundle + its member rows. Empty members rejected.
pub async fn create_bundle(pool: &SqlitePool, new: &NewBundle<'_>) -> ArgoResult<String> {
    if new.member_approval_ids.is_empty() {
        return Err(ArgoError::Provider("empty bundle".into()));
    }
    let id = Uuid::now_v7().to_string();
    let id_bytes = parse_id_bytes(&id)?;
    let created_at = Utc::now().timestamp_micros();
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| ArgoError::Provider(format!("bundle tx: {e}")))?;
    sqlx::query(
        "INSERT INTO approval_bundle \
         (id, tenant_id, label, created_at, status) \
         VALUES (?, ?, ?, ?, 'pending')",
    )
    .bind(&id_bytes)
    .bind(new.tenant_id.0.as_bytes().as_slice())
    .bind(new.label)
    .bind(created_at)
    .execute(&mut *tx)
    .await
    .map_err(|e| ArgoError::Provider(format!("bundle insert: {e}")))?;
    for (idx, approval_id) in new.member_approval_ids.iter().enumerate() {
        sqlx::query(
            "INSERT INTO approval_bundle_member \
             (bundle_id, approval_id, member_idx) VALUES (?, ?, ?)",
        )
        .bind(&id_bytes)
        .bind(approval_id)
        .bind(i64::try_from(idx).unwrap_or(i64::MAX))
        .execute(&mut *tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("bundle member insert: {e}")))?;
    }
    tx.commit()
        .await
        .map_err(|e| ArgoError::Provider(format!("bundle commit: {e}")))?;
    Ok(id)
}

/// Fetch one bundle (with members) by id.
pub async fn get_bundle(pool: &SqlitePool, id: &str) -> ArgoResult<Option<ApprovalBundle>> {
    let id_bytes = parse_id_bytes(id)?;
    let row: Option<BundleRow> = sqlx::query_as(SELECT_BUNDLE_BY_ID)
        .bind(&id_bytes)
        .fetch_optional(pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("bundle get: {e}")))?;
    let Some(row) = row else { return Ok(None) };
    let members = fetch_members(pool, &id_bytes).await?;
    Ok(Some(into_bundle(row, members)?))
}

/// Pending bundles for a tenant, oldest-first, capped at `limit`.
pub async fn list_pending_bundles(
    pool: &SqlitePool,
    tenant_id: TenantId,
    limit: u32,
) -> ArgoResult<Vec<ApprovalBundle>> {
    let rows: Vec<BundleRow> = sqlx::query_as(SELECT_PENDING_BY_TENANT)
        .bind(tenant_id.0.as_bytes().as_slice())
        .bind(i64::from(limit))
        .fetch_all(pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("bundle list_pending: {e}")))?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let id_bytes = row.0.clone();
        let members = fetch_members(pool, &id_bytes).await?;
        out.push(into_bundle(row, members)?);
    }
    Ok(out)
}

/// Record the operator's collective decision. Idempotent: second
/// call on a non-Pending row is a no-op (`false`); first successful
/// call returns `true`. `WHERE status = 'pending'` keeps this safe
/// under concurrent operator races.
pub async fn decide_bundle(
    pool: &SqlitePool,
    id: &str,
    decided_by: &str,
    decision: BundleDecision,
) -> ArgoResult<bool> {
    let id_bytes = parse_id_bytes(id)?;
    let now = Utc::now().timestamp_micros();
    let rows = sqlx::query(
        "UPDATE approval_bundle \
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
    .map_err(|e| ArgoError::Provider(format!("bundle decide: {e}")))?;
    Ok(rows.rows_affected() > 0)
}

async fn fetch_members(pool: &SqlitePool, id_bytes: &[u8]) -> ArgoResult<Vec<String>> {
    let members: Vec<String> = sqlx::query_scalar(
        "SELECT approval_id FROM approval_bundle_member \
         WHERE bundle_id = ? ORDER BY member_idx ASC",
    )
    .bind(id_bytes)
    .fetch_all(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("bundle members fetch: {e}")))?;
    Ok(members)
}

fn parse_id_bytes(id: &str) -> ArgoResult<Vec<u8>> {
    Uuid::parse_str(id)
        .map(|u| u.as_bytes().to_vec())
        .map_err(|e| ArgoError::Provider(format!("bundle id parse: {e}")))
}

const SELECT_BUNDLE_BY_ID: &str = "SELECT id, tenant_id, label, created_at, status, \
                                   decided_at, decided_by, decision \
                                   FROM approval_bundle WHERE id = ?";

const SELECT_PENDING_BY_TENANT: &str = "SELECT id, tenant_id, label, created_at, status, \
                                        decided_at, decided_by, decision \
                                        FROM approval_bundle \
                                        WHERE tenant_id = ? AND status = 'pending' \
                                        ORDER BY created_at ASC LIMIT ?";

type BundleRow = (
    Vec<u8>,        // id
    Vec<u8>,        // tenant_id
    String,         // label
    i64,            // created_at
    String,         // status
    Option<i64>,    // decided_at
    Option<String>, // decided_by
    Option<String>, // decision
);

fn into_bundle(row: BundleRow, members: Vec<String>) -> ArgoResult<ApprovalBundle> {
    let id = Uuid::from_slice(&row.0)
        .map_err(|e| ArgoError::Provider(format!("bundle id from db: {e}")))?
        .to_string();
    let tenant_uuid = Uuid::from_slice(&row.1)
        .map_err(|e| ArgoError::Provider(format!("bundle tenant_id from db: {e}")))?;
    Ok(ApprovalBundle {
        id,
        tenant_id: TenantId(tenant_uuid),
        label: row.2,
        created_at: row.3,
        status: BundleStatus::from_str_lossy(&row.4),
        decided_at: row.5,
        decided_by: row.6,
        decision: row.7,
        members,
    })
}

#[cfg(test)]
#[path = "approval_bundle_tests.rs"]
mod tests;
