//! SCIM 2.0 user-provisioning store (Mission `mission-scim-provisioning`).
//!
//! Backs the SCIM endpoints exposed by `argo-cli`'s gateway under
//! `/scim/v2/Users`. Schema is owned by `migrations/v63.rs`; callers
//! must run [`crate::SessionStore::migrate`] before constructing a
//! [`ScimUserStore`].
//!
//! Soft-delete pattern: `DELETE /scim/v2/Users/<id>` sets
//! `scim_user.deleted_at` and inserts a row in
//! `scim_provisioned_tombstones` (same transaction). The audit chain
//! can still resolve the user's `tenant_id` + `user_name` after a
//! delete without scanning every soft-deleted `scim_user` row.
//!
//! SQLite has no native `timestamptz` / `bool`, so the row encodes
//! timestamps as `INTEGER` (unix seconds, UTC) and `active` as
//! `INTEGER` (0/1). `emails` round-trips through the `emails_json`
//! `TEXT` column.
use crate::StoreResult;
use argo_types::{ArgoError, TenantId};
use chrono::{DateTime, TimeZone, Utc};
use sqlx::{Pool, Sqlite};
use uuid::Uuid;

/// One row from `scim_user`. Read shape only — writes go through
/// the [`ScimUserStore`] helpers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScimUserRow {
    /// Surrogate user id (UUID v7) — wire form of the SCIM `id` field.
    pub id: Uuid,
    /// Tenant the user belongs to. Cross-tenant lookups are filtered
    /// in the handler before this layer is reached.
    pub tenant_id: TenantId,
    /// IdP-supplied stable id (Okta `externalId`, Azure AD object id).
    pub external_id: Option<String>,
    /// SCIM `userName` — IdP-supplied login id (e.g. email).
    pub user_name: String,
    /// SCIM `displayName` — optional.
    pub display_name: Option<String>,
    /// SCIM `emails` — multi-valued; round-trips through `emails_json`.
    pub emails: Vec<String>,
    /// SCIM `active`. `false` keeps the row but prevents future auth.
    pub active: bool,
    /// Row creation timestamp (insert time).
    pub created_at: DateTime<Utc>,
    /// Last update timestamp (any PATCH/PUT).
    pub updated_at: DateTime<Utc>,
    /// Soft-delete marker — `Some(when)` after `DELETE`, `None`
    /// otherwise.
    pub deleted_at: Option<DateTime<Utc>>,
}

/// Input to [`ScimUserStore::insert`]. Constructed inside the gateway
/// handler after the SCIM body has been validated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewScimUser {
    /// Tenant scope. Resolved from the workspace before the insert.
    pub tenant_id: TenantId,
    /// IdP-supplied stable id.
    pub external_id: Option<String>,
    /// SCIM `userName`.
    pub user_name: String,
    /// SCIM `displayName`.
    pub display_name: Option<String>,
    /// SCIM `emails` array.
    pub emails: Vec<String>,
    /// SCIM `active`. Defaults to `true` at the handler when absent.
    pub active: bool,
    /// Caller-controlled creation timestamp (so tests pin time).
    pub created_at: DateTime<Utc>,
}

/// One row from `scim_provisioned_tombstones`. Index-only — the
/// canonical record stays in `scim_user` with `deleted_at` set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScimTombstoneRow {
    /// The deleted user's id (matches `scim_user.id`).
    pub id: Uuid,
    /// Tenant the user belonged to.
    pub tenant_id: TenantId,
    /// SCIM `userName` at the time of deletion.
    pub user_name: String,
    /// Wall-clock delete timestamp.
    pub deleted_at: DateTime<Utc>,
    /// Why the user was deprovisioned — `scim_delete` for an explicit
    /// `DELETE`, `scim_deactivate` reserved for future cascade logic.
    pub reason: String,
}

/// Thin wrapper around the workspace SQLite pool.
pub struct ScimUserStore<'a> {
    pool: &'a Pool<Sqlite>,
}

impl<'a> ScimUserStore<'a> {
    /// Construct over an already-migrated workspace pool.
    #[must_use]
    pub fn new(pool: &'a Pool<Sqlite>) -> Self {
        Self { pool }
    }

    /// Persist a new SCIM user. Returns the assigned UUID v7. The
    /// handler is responsible for checking `userName` uniqueness via
    /// [`Self::find_by_user_name`] first.
    ///
    /// # Errors
    /// Returns [`ArgoError::Provider`] on SQL or JSON-encoding failure.
    pub async fn insert(&self, new: NewScimUser) -> StoreResult<Uuid> {
        let id = Uuid::now_v7();
        let emails_json = serde_json::to_string(&new.emails)
            .map_err(|e| ArgoError::Provider(format!("encode emails: {e}")))?;
        sqlx::query(
            "INSERT INTO scim_user
                (id, tenant_id, external_id, user_name, display_name,
                 emails_json, active, created_at, updated_at, deleted_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, NULL)",
        )
        .bind(id.as_bytes().as_slice())
        .bind(new.tenant_id.0.as_bytes().as_slice())
        .bind(new.external_id.as_deref())
        .bind(&new.user_name)
        .bind(new.display_name.as_deref())
        .bind(&emails_json)
        .bind(i64::from(new.active))
        .bind(new.created_at.timestamp())
        .bind(new.created_at.timestamp())
        .execute(self.pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("scim_user insert: {e}")))?;
        Ok(id)
    }

    /// Fetch one user by id. Returns `None` when the row is absent
    /// AND when the row is soft-deleted (callers that need tombstoned
    /// rows use [`Self::get_including_deleted`]).
    ///
    /// # Errors
    /// Returns [`ArgoError::Provider`] on SQL or decode failure.
    pub async fn get(&self, id: Uuid) -> StoreResult<Option<ScimUserRow>> {
        let row: Option<RawRow> = sqlx::query_as(SELECT_BY_ID)
            .bind(id.as_bytes().as_slice())
            .fetch_optional(self.pool)
            .await
            .map_err(|e| ArgoError::Provider(format!("scim_user get: {e}")))?;
        row.filter(|r| r.9.is_none()).map(decode_row).transpose()
    }

    /// Fetch one user by id, including soft-deleted rows. Powers
    /// `is_active` checks from the SCIM auth gate.
    ///
    /// # Errors
    /// Returns [`ArgoError::Provider`] on SQL or decode failure.
    pub async fn get_including_deleted(&self, id: Uuid) -> StoreResult<Option<ScimUserRow>> {
        let row: Option<RawRow> = sqlx::query_as(SELECT_BY_ID)
            .bind(id.as_bytes().as_slice())
            .fetch_optional(self.pool)
            .await
            .map_err(|e| ArgoError::Provider(format!("scim_user get: {e}")))?;
        row.map(decode_row).transpose()
    }

    /// Look up a live user by `(tenant, user_name)`. Powers the SCIM
    /// `?filter=userName eq "..."` query and the POST-side uniqueness
    /// check. Soft-deleted rows are excluded.
    ///
    /// # Errors
    /// Returns [`ArgoError::Provider`] on SQL or decode failure.
    pub async fn find_by_user_name(
        &self,
        tenant_id: TenantId,
        user_name: &str,
    ) -> StoreResult<Option<ScimUserRow>> {
        let row: Option<RawRow> = sqlx::query_as(SELECT_BY_USER_NAME)
            .bind(tenant_id.0.as_bytes().as_slice())
            .bind(user_name)
            .fetch_optional(self.pool)
            .await
            .map_err(|e| ArgoError::Provider(format!("scim_user find: {e}")))?;
        row.map(decode_row).transpose()
    }

    /// Paginated list of live (non-deleted) users for a tenant,
    /// SCIM-style 1-indexed `start_index` + `count`. Sorted by
    /// `created_at` ascending so pagination is stable.
    ///
    /// # Errors
    /// Returns [`ArgoError::Provider`] on SQL or decode failure.
    pub async fn list(
        &self,
        tenant_id: TenantId,
        start_index: i64,
        count: i64,
    ) -> StoreResult<(Vec<ScimUserRow>, i64)> {
        let offset = (start_index - 1).max(0);
        let limit = count.max(0);
        let rows: Vec<RawRow> = sqlx::query_as(LIST_SQL)
            .bind(tenant_id.0.as_bytes().as_slice())
            .bind(limit)
            .bind(offset)
            .fetch_all(self.pool)
            .await
            .map_err(|e| ArgoError::Provider(format!("scim_user list: {e}")))?;
        let total: i64 = sqlx::query_scalar(COUNT_SQL)
            .bind(tenant_id.0.as_bytes().as_slice())
            .fetch_one(self.pool)
            .await
            .map_err(|e| ArgoError::Provider(format!("scim_user count: {e}")))?;
        let decoded: StoreResult<Vec<ScimUserRow>> = rows.into_iter().map(decode_row).collect();
        Ok((decoded?, total))
    }

    /// Update mutable SCIM fields (`display_name`, `emails`, `active`,
    /// `external_id`). Returns `true` on row change. Callers that want
    /// to enforce SCIM "no-op PATCH stays idempotent" semantics treat
    /// `false` as already-current and return 200.
    ///
    /// # Errors
    /// Returns [`ArgoError::Provider`] on SQL or JSON-encoding failure.
    pub async fn update(&self, id: Uuid, patch: ScimUserPatch) -> StoreResult<bool> {
        let emails_json = match patch.emails.as_ref() {
            Some(v) => Some(
                serde_json::to_string(v)
                    .map_err(|e| ArgoError::Provider(format!("encode emails: {e}")))?,
            ),
            None => None,
        };
        let result = sqlx::query(
            "UPDATE scim_user SET
                display_name = COALESCE(?, display_name),
                emails_json  = COALESCE(?, emails_json),
                external_id  = COALESCE(?, external_id),
                active       = COALESCE(?, active),
                updated_at   = ?
             WHERE id = ? AND deleted_at IS NULL",
        )
        .bind(patch.display_name.as_deref())
        .bind(emails_json.as_deref())
        .bind(patch.external_id.as_deref())
        .bind(patch.active.map(i64::from))
        .bind(patch.updated_at.timestamp())
        .bind(id.as_bytes().as_slice())
        .execute(self.pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("scim_user update: {e}")))?;
        Ok(result.rows_affected() > 0)
    }

    /// Soft-delete a user and insert the matching tombstone in the
    /// same transaction. Returns `true` when a row changed (idempotent
    /// re-DELETE returns `false`, which the handler turns into 204).
    ///
    /// # Errors
    /// Returns [`ArgoError::Provider`] on SQL execution failure at any
    /// step. The transaction rolls back; no partial state.
    pub async fn soft_delete(&self, id: Uuid, now: DateTime<Utc>) -> StoreResult<bool> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ArgoError::Provider(format!("scim_user delete tx: {e}")))?;
        let row: Option<RawRow> = sqlx::query_as(SELECT_BY_ID)
            .bind(id.as_bytes().as_slice())
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("scim_user delete lookup: {e}")))?;
        let Some(existing) = row else {
            return Ok(false);
        };
        if existing.9.is_some() {
            return Ok(false);
        }
        sqlx::query(
            "UPDATE scim_user SET deleted_at = ?, active = 0, updated_at = ? WHERE id = ?",
        )
        .bind(now.timestamp())
        .bind(now.timestamp())
        .bind(id.as_bytes().as_slice())
        .execute(&mut *tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("scim_user soft-delete: {e}")))?;
        sqlx::query(
            "INSERT INTO scim_provisioned_tombstones
                (id, tenant_id, user_name, deleted_at, reason)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(id.as_bytes().as_slice())
        .bind(existing.1.as_slice())
        .bind(&existing.3)
        .bind(now.timestamp())
        .bind("scim_delete")
        .execute(&mut *tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("scim tombstone insert: {e}")))?;
        tx.commit()
            .await
            .map_err(|e| ArgoError::Provider(format!("scim_user delete commit: {e}")))?;
        Ok(true)
    }

    /// Look up the tombstone row for a deleted user. Returns `None`
    /// when no row matches.
    ///
    /// # Errors
    /// Returns [`ArgoError::Provider`] on SQL or decode failure.
    pub async fn tombstone(&self, id: Uuid) -> StoreResult<Option<ScimTombstoneRow>> {
        let row: Option<RawTombstoneRow> = sqlx::query_as(
            "SELECT id, tenant_id, user_name, deleted_at, reason \
             FROM scim_provisioned_tombstones WHERE id = ?",
        )
        .bind(id.as_bytes().as_slice())
        .fetch_optional(self.pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("scim tombstone get: {e}")))?;
        row.map(decode_tombstone).transpose()
    }
}

/// Partial-update input for [`ScimUserStore::update`]. Every field
/// is optional — a `None` leaves the column unchanged.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ScimUserPatch {
    /// New `displayName` (None = no change).
    pub display_name: Option<String>,
    /// New `emails` array (None = no change). An empty `Vec` clears
    /// the emails set explicitly.
    pub emails: Option<Vec<String>>,
    /// New `externalId` (None = no change).
    pub external_id: Option<String>,
    /// New `active` flag (None = no change). `Some(false)` is the
    /// SCIM deactivation path.
    pub active: Option<bool>,
    /// Caller-controlled update timestamp.
    pub updated_at: DateTime<Utc>,
}

type RawRow = (
    Vec<u8>,
    Vec<u8>,
    Option<String>,
    String,
    Option<String>,
    String,
    i64,
    i64,
    i64,
    Option<i64>,
);

type RawTombstoneRow = (Vec<u8>, Vec<u8>, String, i64, String);

const SELECT_BY_ID: &str =
    "SELECT id, tenant_id, external_id, user_name, display_name, \
            emails_json, active, created_at, updated_at, deleted_at \
     FROM scim_user WHERE id = ?";

const SELECT_BY_USER_NAME: &str =
    "SELECT id, tenant_id, external_id, user_name, display_name, \
            emails_json, active, created_at, updated_at, deleted_at \
     FROM scim_user \
     WHERE tenant_id = ? AND user_name = ? AND deleted_at IS NULL";

const LIST_SQL: &str =
    "SELECT id, tenant_id, external_id, user_name, display_name, \
            emails_json, active, created_at, updated_at, deleted_at \
     FROM scim_user \
     WHERE tenant_id = ? AND deleted_at IS NULL \
     ORDER BY created_at ASC, id ASC \
     LIMIT ? OFFSET ?";

const COUNT_SQL: &str =
    "SELECT COUNT(*) FROM scim_user WHERE tenant_id = ? AND deleted_at IS NULL";

fn decode_row(row: RawRow) -> StoreResult<ScimUserRow> {
    let (id_b, tid_b, external_id, user_name, display_name, emails_json, active, created, updated, deleted) =
        row;
    let id = Uuid::from_slice(&id_b)
        .map_err(|e| ArgoError::Provider(format!("decode scim_user id: {e}")))?;
    let tenant_id = Uuid::from_slice(&tid_b)
        .map(TenantId)
        .map_err(|e| ArgoError::Provider(format!("decode scim_user tenant: {e}")))?;
    let emails: Vec<String> = serde_json::from_str(&emails_json)
        .map_err(|e| ArgoError::Provider(format!("decode emails_json: {e}")))?;
    let created_at = ts(created, "created_at")?;
    let updated_at = ts(updated, "updated_at")?;
    let deleted_at = match deleted {
        Some(t) => Some(ts(t, "deleted_at")?),
        None => None,
    };
    Ok(ScimUserRow {
        id,
        tenant_id,
        external_id,
        user_name,
        display_name,
        emails,
        active: active != 0,
        created_at,
        updated_at,
        deleted_at,
    })
}

fn decode_tombstone(row: RawTombstoneRow) -> StoreResult<ScimTombstoneRow> {
    let (id_b, tid_b, user_name, deleted, reason) = row;
    let id = Uuid::from_slice(&id_b)
        .map_err(|e| ArgoError::Provider(format!("decode tombstone id: {e}")))?;
    let tenant_id = Uuid::from_slice(&tid_b)
        .map(TenantId)
        .map_err(|e| ArgoError::Provider(format!("decode tombstone tenant: {e}")))?;
    let deleted_at = ts(deleted, "deleted_at")?;
    Ok(ScimTombstoneRow {
        id,
        tenant_id,
        user_name,
        deleted_at,
        reason,
    })
}

fn ts(secs: i64, label: &str) -> StoreResult<DateTime<Utc>> {
    Utc.timestamp_opt(secs, 0)
        .single()
        .ok_or_else(|| ArgoError::Provider(format!("decode {label} out of range")))
}
