//! Storage for outstanding workspace-member invitations
//! (Stream `web-settings-members-invite-real-wire`).
//!
//! The `/settings/members` page lets owners and admins email an
//! invitation to a new member. Each invitation has its own SHA-256
//! token hash (distinct from the login-side `magic_link_tokens` table
//! so the 7-day invite window does not have to share a row shape with
//! the 15-minute login token). Accepted rows stay in the table so an
//! operator forensic-trace can answer "who invited Carol and when?".
//!
//! Schema lives in `migrations/127__workspace_member_invitation.sql`;
//! [`ensure_schema`] applies the DDL idempotently at every entry
//! point (same lazy-bootstrap pattern as
//! [`crate::user_account::ensure_schema`]).
//!
//! ```no_run
//! # async fn demo() -> argo_types::ArgoResult<()> {
//! use argo_store::workspace_member_invitation::{
//!     create_invitation, lookup_pending_by_token_hash,
//! };
//! use argo_store::SessionStore;
//! use argo_types::TenantId;
//! let store = SessionStore::open(":memory:").await?;
//! store.migrate().await?;
//! let tenant = TenantId::new();
//! let inv = create_invitation(
//!     &store, tenant, "alice@example.com", "user", &[], "deadbeef",
//!     "operator@example.com", 1700, 1700 + 7 * 24 * 3600,
//! ).await?;
//! assert!(lookup_pending_by_token_hash(&store, "deadbeef").await?.is_some());
//! # Ok(()) }
//! ```

use crate::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use sqlx::SqlitePool;
use uuid::Uuid;

/// Default invitation lifetime — 7 days from issue.
pub const INVITATION_TTL_SECS: i64 = 7 * 24 * 60 * 60;

/// One row of the `workspace_member_invitation` table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invitation {
    /// Opaque ULID/UUID identifier emitted in the audit payload.
    pub id: String,
    /// Tenant the invitee will join on accept.
    pub tenant_id: TenantId,
    /// Email the invite was sent to (canonical lowercase).
    pub email: String,
    /// Role label persisted with the invitation. Validated at insert
    /// time via the table's `CHECK` constraint — `'owner' | 'admin'
    /// | 'user' | 'guest'`.
    pub role: String,
    /// Cohort-scope list serialized as a JSON array. Empty for
    /// owner/admin (those tiers always see everything).
    pub cohort_scope: Vec<String>,
    /// SHA-256 hex of the random token emailed to the invitee. The
    /// raw token is never persisted.
    pub token_hash: String,
    /// Principal that sent the invite (`human:<email>` form).
    pub invited_by: String,
    /// Unix-seconds timestamp of insert.
    pub invited_at: i64,
    /// Unix-seconds timestamp after which `accept_invitation` returns
    /// `Expired`.
    pub expires_at: i64,
    /// Unix-seconds timestamp the invitation was consumed. `None`
    /// while pending.
    pub accepted_at: Option<i64>,
    /// Identifier of the user_account row created or matched at
    /// accept time. `None` while pending.
    pub accepted_by_user_id: Option<Uuid>,
}

const ENSURE_SQL: &str = include_str!("../migrations/127__workspace_member_invitation.sql");

/// Ensure the `workspace_member_invitation` table + supporting indexes
/// exist. Safe to call repeatedly; gated on `IF NOT EXISTS`.
pub async fn ensure_schema(pool: &SqlitePool) -> ArgoResult<()> {
    let stripped: String = ENSURE_SQL
        .lines()
        .filter(|line| !line.trim_start().starts_with("--"))
        .collect::<Vec<_>>()
        .join("\n");
    for stmt in stripped.split(';').map(str::trim).filter(|s| !s.is_empty()) {
        sqlx::query(stmt)
            .execute(pool)
            .await
            .map_err(|e| ArgoError::Provider(format!("workspace_member_invitation ensure: {e}")))?;
    }
    Ok(())
}

/// Outcome of accepting an invitation via [`accept_invitation`].
///
/// `Ok` boxes [`Invitation`] so the unit variants do not pay the
/// large-struct alignment cost (`clippy::large_enum_variant`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AcceptOutcome {
    /// Invitation was pending and is now marked accepted. The boxed
    /// invitation reflects the row after the accept transition.
    Ok(Box<Invitation>),
    /// No invitation row matched the token hash.
    NotFound,
    /// Invitation row exists, `expires_at` is at/before `now`.
    Expired,
    /// Invitation row exists, `accepted_at` is already set.
    AlreadyAccepted,
}

/// Insert a new invitation row. Caller hashes the raw token (SHA-256,
/// hex-lowercase) and computes `expires_at` as
/// `invited_at + INVITATION_TTL_SECS` (or supplies a custom window).
#[allow(clippy::too_many_arguments)]
pub async fn create_invitation(
    store: &SessionStore,
    tenant_id: TenantId,
    email: &str,
    role: &str,
    cohort_scope: &[String],
    token_hash: &str,
    invited_by: &str,
    invited_at: i64,
    expires_at: i64,
) -> ArgoResult<Invitation> {
    ensure_schema(store.pool()).await?;
    let id = Uuid::now_v7().to_string();
    let cohort_json = serde_json::to_string(cohort_scope)
        .map_err(|e| ArgoError::Provider(format!("cohort_scope encode: {e}")))?;
    sqlx::query(
        "INSERT INTO workspace_member_invitation \
         (id, tenant_id, email, role, cohort_scope_json, token_hash, invited_by, invited_at, expires_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(tenant_id.0.as_bytes().as_slice())
    .bind(email)
    .bind(role)
    .bind(&cohort_json)
    .bind(token_hash)
    .bind(invited_by)
    .bind(invited_at)
    .bind(expires_at)
    .execute(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("workspace_member_invitation insert: {e}")))?;
    Ok(Invitation {
        id,
        tenant_id,
        email: email.to_string(),
        role: role.to_string(),
        cohort_scope: cohort_scope.to_vec(),
        token_hash: token_hash.to_string(),
        invited_by: invited_by.to_string(),
        invited_at,
        expires_at,
        accepted_at: None,
        accepted_by_user_id: None,
    })
}

/// Look up the pending invitation for a tenant + email pair. Returns
/// `None` when no pending row exists (rows are pending while
/// `accepted_at IS NULL`).
pub async fn lookup_pending_by_email(
    store: &SessionStore,
    tenant_id: TenantId,
    email: &str,
) -> ArgoResult<Option<Invitation>> {
    ensure_schema(store.pool()).await?;
    let row: Option<InvitationRow> = sqlx::query_as(
        "SELECT id, tenant_id, email, role, cohort_scope_json, token_hash, invited_by, \
                invited_at, expires_at, accepted_at, accepted_by_user_id \
         FROM workspace_member_invitation \
         WHERE tenant_id = ? AND email = ? AND accepted_at IS NULL",
    )
    .bind(tenant_id.0.as_bytes().as_slice())
    .bind(email)
    .fetch_optional(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("invitation lookup_pending_by_email: {e}")))?;
    row.map(invitation_from_row).transpose()
}

/// Look up an invitation by its SHA-256 token hash. Returns `None` when
/// no row matches.
pub async fn lookup_pending_by_token_hash(
    store: &SessionStore,
    token_hash: &str,
) -> ArgoResult<Option<Invitation>> {
    ensure_schema(store.pool()).await?;
    let row: Option<InvitationRow> = sqlx::query_as(
        "SELECT id, tenant_id, email, role, cohort_scope_json, token_hash, invited_by, \
                invited_at, expires_at, accepted_at, accepted_by_user_id \
         FROM workspace_member_invitation WHERE token_hash = ?",
    )
    .bind(token_hash)
    .fetch_optional(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("invitation lookup_by_token_hash: {e}")))?;
    row.map(invitation_from_row).transpose()
}

/// Atomically consume an invitation. The `UPDATE` is guarded with
/// `accepted_at IS NULL` so a concurrent accept race reports the loser
/// as [`AcceptOutcome::AlreadyAccepted`]. The caller is responsible
/// for the follow-on `user_tenant` insert.
pub async fn accept_invitation(
    store: &SessionStore,
    token_hash: &str,
    user_id: Uuid,
    now_epoch_secs: i64,
) -> ArgoResult<AcceptOutcome> {
    ensure_schema(store.pool()).await?;
    let Some(invitation) = lookup_pending_by_token_hash(store, token_hash).await? else {
        return Ok(AcceptOutcome::NotFound);
    };
    if invitation.accepted_at.is_some() {
        return Ok(AcceptOutcome::AlreadyAccepted);
    }
    if invitation.expires_at <= now_epoch_secs {
        return Ok(AcceptOutcome::Expired);
    }
    let res = sqlx::query(
        "UPDATE workspace_member_invitation SET accepted_at = ?, accepted_by_user_id = ? \
         WHERE token_hash = ? AND accepted_at IS NULL",
    )
    .bind(now_epoch_secs)
    .bind(user_id.as_bytes().as_slice())
    .bind(token_hash)
    .execute(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("invitation accept: {e}")))?;
    if res.rows_affected() == 0 {
        return Ok(AcceptOutcome::AlreadyAccepted);
    }
    Ok(AcceptOutcome::Ok(Box::new(Invitation {
        accepted_at: Some(now_epoch_secs),
        accepted_by_user_id: Some(user_id),
        ..invitation
    })))
}

/// All pending invitations for a tenant, newest first. Used by the
/// `/settings/members` page to render the pending-invitation rows
/// alongside accepted members.
pub async fn list_pending_for_tenant(
    store: &SessionStore,
    tenant_id: TenantId,
) -> ArgoResult<Vec<Invitation>> {
    ensure_schema(store.pool()).await?;
    let rows: Vec<InvitationRow> = sqlx::query_as(
        "SELECT id, tenant_id, email, role, cohort_scope_json, token_hash, invited_by, \
                invited_at, expires_at, accepted_at, accepted_by_user_id \
         FROM workspace_member_invitation \
         WHERE tenant_id = ? AND accepted_at IS NULL \
         ORDER BY invited_at DESC",
    )
    .bind(tenant_id.0.as_bytes().as_slice())
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("invitation list_pending: {e}")))?;
    rows.into_iter().map(invitation_from_row).collect()
}

type InvitationRow = (
    String,
    Vec<u8>,
    String,
    String,
    String,
    String,
    String,
    i64,
    i64,
    Option<i64>,
    Option<Vec<u8>>,
);

fn invitation_from_row(row: InvitationRow) -> ArgoResult<Invitation> {
    let (
        id,
        tenant_id,
        email,
        role,
        cohort_scope_json,
        token_hash,
        invited_by,
        invited_at,
        expires_at,
        accepted_at,
        accepted_by_user_id,
    ) = row;
    let tenant_uuid = Uuid::from_slice(&tenant_id)
        .map_err(|e| ArgoError::Provider(format!("invitation tenant_id decode: {e}")))?;
    let accepted_by_uuid = accepted_by_user_id
        .map(|bytes| {
            Uuid::from_slice(&bytes)
                .map_err(|e| ArgoError::Provider(format!("invitation user_id decode: {e}")))
        })
        .transpose()?;
    let cohort_scope: Vec<String> = serde_json::from_str(&cohort_scope_json)
        .map_err(|e| ArgoError::Provider(format!("invitation cohort_scope decode: {e}")))?;
    Ok(Invitation {
        id,
        tenant_id: TenantId(tenant_uuid),
        email,
        role,
        cohort_scope,
        token_hash,
        invited_by,
        invited_at,
        expires_at,
        accepted_at,
        accepted_by_user_id: accepted_by_uuid,
    })
}

#[cfg(test)]
#[path = "workspace_member_invitation_tests.rs"]
mod tests;
