//! Elevated-view grant store (Stream J16).
//!
//! Per-principal, per-category, time-bounded "show me the raw value"
//! override. The J2 redact-policy resolver unions a row's
//! `categories` set with the viewer's RBAC permissions before
//! deciding what to mask; the J16 HTTP endpoints
//! (`POST /api/v1/audit/elevated-view-request`, `GET .../grants`,
//! `DELETE .../grants/<id>`) sit on top of this layer.
//!
//! Schema is owned by `migrations/v59.rs`; callers must run
//! `SessionStore::migrate` before constructing an
//! [`ElevatedViewGrantStore`].
//!
//! SQLite has no native `timestamptz` / `text[]`, so timestamps live
//! as `INTEGER` (unix seconds, UTC) and `categories` lives as a
//! JSON-array `TEXT` column. The boundary encodes/decodes between
//! `Vec<String>` + `chrono::DateTime<Utc>` and the storage shape.
use crate::StoreResult;
use argo_types::{ArgoError, TenantId};
use chrono::{DateTime, TimeZone, Utc};
use sqlx::{Pool, Sqlite};
use uuid::Uuid;

/// One row from `elevated_view_grant`. Read shape only — writes go
/// through [`ElevatedViewGrantStore::insert`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElevatedViewGrantRow {
    /// Surrogate grant id (UUID v7).
    pub id: Uuid,
    /// Tenant the grant belongs to. Cross-tenant reads are 403'd
    /// in the gateway handler before this layer is reached.
    pub tenant_id: TenantId,
    /// Viewer principal the grant applies to. Wire form mirrors
    /// `audit_record.principal_id` (e.g. `human:mark@example.com`).
    pub principal_id: String,
    /// Wire labels for the categories this grant covers. Decoded
    /// from the `categories_json` column.
    pub categories: Vec<String>,
    /// Operator-supplied reason (≥ 20 chars at the handler).
    pub reason: String,
    /// Grant issue timestamp.
    pub granted_at: DateTime<Utc>,
    /// Wall-clock expiry. `expires_at <= now` rows are filtered
    /// out by [`ElevatedViewGrantStore::list_active_for_principal`].
    pub expires_at: DateTime<Utc>,
    /// `Some(when)` when the grant was revoked (self-revoke or
    /// operator-revoke); `None` for live grants. Revoked rows
    /// stay in the table for audit purposes.
    pub revoked_at: Option<DateTime<Utc>>,
}

/// Input to [`ElevatedViewGrantStore::insert`]. Constructed inside
/// the gateway handler after the body has been validated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewElevatedViewGrant {
    /// Tenant scope. Resolved from the workspace before the insert.
    pub tenant_id: TenantId,
    /// Viewer principal (wire form, same shape as `principal_id`).
    pub principal_id: String,
    /// Wire-label categories the grant unlocks.
    pub categories: Vec<String>,
    /// Operator reason (≥ 20 chars).
    pub reason: String,
    /// Issue timestamp (caller-controlled so tests can pin time).
    pub granted_at: DateTime<Utc>,
    /// Wall-clock expiry (`granted_at + duration_hours * 1h`).
    pub expires_at: DateTime<Utc>,
}

/// Thin wrapper around the workspace SQLite pool.
pub struct ElevatedViewGrantStore<'a> {
    pool: &'a Pool<Sqlite>,
}

impl<'a> ElevatedViewGrantStore<'a> {
    /// Construct over an already-migrated workspace pool.
    #[must_use]
    pub fn new(pool: &'a Pool<Sqlite>) -> Self {
        Self { pool }
    }

    /// Persist one grant. Returns the assigned UUID v7.
    ///
    /// # Errors
    ///
    /// Returns [`ArgoError::Provider`] on SQL execution failure or
    /// JSON encoding failure of the categories vector.
    pub async fn insert(&self, new: NewElevatedViewGrant) -> StoreResult<Uuid> {
        let id = Uuid::now_v7();
        let categories_json = serde_json::to_string(&new.categories)
            .map_err(|e| ArgoError::Provider(format!("encode categories: {e}")))?;
        sqlx::query(
            "INSERT INTO elevated_view_grant
                (id, tenant_id, principal_id, categories_json, reason,
                 granted_at, expires_at, revoked_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, NULL)",
        )
        .bind(id.as_bytes().as_slice())
        .bind(new.tenant_id.0.as_bytes().as_slice())
        .bind(&new.principal_id)
        .bind(&categories_json)
        .bind(&new.reason)
        .bind(new.granted_at.timestamp())
        .bind(new.expires_at.timestamp())
        .execute(self.pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("elevated_view_grant insert: {e}")))?;
        Ok(id)
    }

    /// Fetch one grant by id. `None` when the row is absent.
    pub async fn get(&self, id: Uuid) -> StoreResult<Option<ElevatedViewGrantRow>> {
        let row: Option<RawRow> = sqlx::query_as(SELECT_COLS)
            .bind(id.as_bytes().as_slice())
            .fetch_optional(self.pool)
            .await
            .map_err(|e| ArgoError::Provider(format!("elevated_view_grant get: {e}")))?;
        row.map(decode_row).transpose()
    }

    /// List grants for a principal, optionally filtering to live rows
    /// (not revoked AND not expired) only.
    ///
    /// `active_only == true` matches the J2 resolver's query:
    /// `WHERE principal_id = $1 AND revoked_at IS NULL AND expires_at > $now`.
    /// `active_only == false` returns every row for the principal so
    /// `GET /grants?active=false` can show expired ones.
    pub async fn list_for_principal(
        &self,
        principal_id: &str,
        active_only: bool,
        now: DateTime<Utc>,
    ) -> StoreResult<Vec<ElevatedViewGrantRow>> {
        let rows: Vec<RawRow> = if active_only {
            sqlx::query_as(LIST_ACTIVE_SQL)
                .bind(principal_id)
                .bind(now.timestamp())
                .fetch_all(self.pool)
                .await
        } else {
            sqlx::query_as(LIST_ALL_SQL)
                .bind(principal_id)
                .fetch_all(self.pool)
                .await
        }
        .map_err(|e| ArgoError::Provider(format!("elevated_view_grant list: {e}")))?;
        rows.into_iter().map(decode_row).collect()
    }

    /// Mark one grant revoked. Returns `true` when a row changed
    /// state (idempotency: revoking a revoked row returns `false`,
    /// the caller turns that into 204 anyway).
    pub async fn revoke(&self, id: Uuid, now: DateTime<Utc>) -> StoreResult<bool> {
        let result = sqlx::query(
            "UPDATE elevated_view_grant SET revoked_at = ? \
             WHERE id = ? AND revoked_at IS NULL",
        )
        .bind(now.timestamp())
        .bind(id.as_bytes().as_slice())
        .execute(self.pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("elevated_view_grant revoke: {e}")))?;
        Ok(result.rows_affected() > 0)
    }
}

/// Raw `query_as` shape — Vec<u8> for ids/tenants, i64 for timestamps,
/// i32 for the optional revoked_at flag. Decoded by [`decode_row`].
type RawRow = (
    Vec<u8>,
    Vec<u8>,
    String,
    String,
    String,
    i64,
    i64,
    Option<i64>,
);

const SELECT_COLS: &str =
    "SELECT id, tenant_id, principal_id, categories_json, reason, \
            granted_at, expires_at, revoked_at \
     FROM elevated_view_grant WHERE id = ?";

const LIST_ACTIVE_SQL: &str =
    "SELECT id, tenant_id, principal_id, categories_json, reason, \
            granted_at, expires_at, revoked_at \
     FROM elevated_view_grant \
     WHERE principal_id = ? AND revoked_at IS NULL AND expires_at > ? \
     ORDER BY granted_at DESC, id DESC";

const LIST_ALL_SQL: &str =
    "SELECT id, tenant_id, principal_id, categories_json, reason, \
            granted_at, expires_at, revoked_at \
     FROM elevated_view_grant \
     WHERE principal_id = ? \
     ORDER BY granted_at DESC, id DESC";

fn decode_row(row: RawRow) -> StoreResult<ElevatedViewGrantRow> {
    let (id_b, tid_b, principal_id, cats_json, reason, granted_at, expires_at, revoked_at) = row;
    let id = Uuid::from_slice(&id_b)
        .map_err(|e| ArgoError::Invariant(format!("decode grant id: {e}")))?;
    let tenant_id = Uuid::from_slice(&tid_b)
        .map(TenantId)
        .map_err(|e| ArgoError::Invariant(format!("decode grant tenant: {e}")))?;
    let categories: Vec<String> = serde_json::from_str(&cats_json)
        .map_err(|e| ArgoError::Invariant(format!("decode categories: {e}")))?;
    let granted_at = Utc
        .timestamp_opt(granted_at, 0)
        .single()
        .ok_or_else(|| ArgoError::Provider("decode granted_at out of range".into()))?;
    let expires_at = Utc
        .timestamp_opt(expires_at, 0)
        .single()
        .ok_or_else(|| ArgoError::Provider("decode expires_at out of range".into()))?;
    let revoked_at = match revoked_at {
        Some(t) => Some(
            Utc.timestamp_opt(t, 0)
                .single()
                .ok_or_else(|| ArgoError::Provider("decode revoked_at out of range".into()))?,
        ),
        None => None,
    };
    Ok(ElevatedViewGrantRow {
        id,
        tenant_id,
        principal_id,
        categories,
        reason,
        granted_at,
        expires_at,
        revoked_at,
    })
}
