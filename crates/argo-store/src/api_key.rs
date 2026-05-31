//! First-class, tenant-scoped API-key store — the Rust home for the
//! `/settings/api-keys` SPA surface (Frontier-B depth, SPEC 2).
//!
//! A row is a *named, scoped* bearer credential: a label, the scopes it
//! may exercise, the cohorts it is bound to, and lifecycle metadata.
//! The **only secret material is `token_hash`** — the SHA-256 hex digest
//! of the raw bearer token. The raw token never lands in the database:
//! it is shown to the operator exactly once in the create response and
//! then forgotten. `prefix` is a masked preview (`sk_live_a4b8...c2e1`)
//! captured at create time and stored as plain metadata, so a list /
//! read never re-derives anything from the secret.
//!
//! This composes with — does **not** replace — [`crate::service_token_binding`]:
//! the gateway's create path writes a binding row keyed on the same
//! `token_hash` so the token resolves a principal + tenant at auth time,
//! while this table carries the operator-facing name / scope / cohort
//! model the binding table has no room for.
//!
//! The schema is owned by the versioned migration `v86`; callers do not
//! bootstrap it — it is applied by the standard migration runner like
//! every other `vNN` table. Every query is keyed by `tenant_id` so one
//! tenant can never read, revoke, or touch another tenant's key (core
//! invariant #2 — multi-tenant by construction).
//!
//! ```no_run
//! # async fn demo() -> argo_types::ArgoResult<()> {
//! use argo_store::api_key::{insert, list_for_tenant, ApiKeyInsert};
//! use sqlx::sqlite::SqlitePoolOptions;
//! // In production the `api_key` table is created by migration v86 via
//! // the standard runner (`SessionStore::migrate`); this snippet is
//! // compile-only (`no_run`).
//! let pool = SqlitePoolOptions::new().connect("sqlite::memory:").await.unwrap();
//! insert(&pool, &ApiKeyInsert {
//!     id: "key_1",
//!     tenant_id: "00000000-0000-0000-0000-000000000001",
//!     name: "Production CI",
//!     token_hash: "deadbeef",            // caller hashes the raw token first
//!     prefix: "sk_live_a4b8...c2e1",      // masked preview, NOT secret-derived on read
//!     scopes_json: "[\"read:workers\"]",
//!     cohort_ids_json: "[]",
//!     all_cohorts: true,
//!     created_at: "2026-06-01T00:00:00Z",
//!     expires_at: None,
//! }).await?;
//! let rows = list_for_tenant(&pool, "00000000-0000-0000-0000-000000000001").await?;
//! assert_eq!(rows.len(), 1);
//! # Ok(()) }
//! ```

use argo_types::{ArgoError, ArgoResult};
use sqlx::SqlitePool;

/// One API-key row as stored. `token_hash` is the SHA-256 hex digest of
/// the raw token — the only secret-derived column — and is intentionally
/// **not** projected onto callers that render the SPA roster; the list
/// read populates everything except the hash. The raw token is never
/// stored at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiKeyRow {
    /// Opaque key identifier (`key_<...>`). Primary key.
    pub id: String,
    /// UUID-shaped tenant identifier. Stored as TEXT for SQLite
    /// portability; the gateway parses to `TenantId`.
    pub tenant_id: String,
    /// Operator-supplied label (e.g. `"Production CI"`).
    pub name: String,
    /// 12-char masked preview captured at create time
    /// (`sk_live_a4b8...c2e1`). Plain metadata — never re-derived from
    /// the secret on read.
    pub prefix: String,
    /// JSON array of scope strings (the SPA's `ApiKeyScope[]`).
    pub scopes_json: String,
    /// JSON array of cohort-id strings the key is bound to.
    pub cohort_ids_json: String,
    /// When `true`, the key applies to every cohort (ignores
    /// `cohort_ids_json`).
    pub all_cohorts: bool,
    /// ISO-8601 / RFC3339 timestamp captured at insert.
    pub created_at: String,
    /// ISO-8601 timestamp of the most recent authenticated use, or
    /// `None` if never used.
    pub last_used_at: Option<String>,
    /// ISO-8601 expiry timestamp, or `None` for a non-expiring key.
    pub expires_at: Option<String>,
    /// `true` once the key has been revoked. Revoked keys stay in the
    /// table (audit trail) but are excluded from the live-key list.
    pub revoked: bool,
}

/// Field bundle for [`insert`]. The caller hashes the raw token into
/// `token_hash` and computes the masked `prefix` before calling — this
/// store never sees the raw secret.
#[derive(Debug, Clone)]
pub struct ApiKeyInsert<'a> {
    /// Opaque key identifier (`key_<...>`).
    pub id: &'a str,
    /// UUID-shaped tenant identifier.
    pub tenant_id: &'a str,
    /// Operator-supplied label.
    pub name: &'a str,
    /// SHA-256 hex digest of the raw token — the only secret material.
    pub token_hash: &'a str,
    /// 12-char masked preview captured at create time.
    pub prefix: &'a str,
    /// JSON array of scope strings.
    pub scopes_json: &'a str,
    /// JSON array of cohort-id strings.
    pub cohort_ids_json: &'a str,
    /// Whether the key applies to every cohort.
    pub all_cohorts: bool,
    /// ISO-8601 create timestamp.
    pub created_at: &'a str,
    /// Optional ISO-8601 expiry timestamp.
    pub expires_at: Option<&'a str>,
}

/// Insert one API-key row. The caller hashes the raw token into
/// `token_hash` and computes `prefix` first. A collision on
/// `token_hash` returns an [`ArgoError::Invariant`] so callers can
/// re-generate and retry.
pub async fn insert(pool: &SqlitePool, row: &ApiKeyInsert<'_>) -> ArgoResult<()> {
    let res = sqlx::query(
        "INSERT INTO api_key \
         (id, tenant_id, name, token_hash, prefix, scopes_json, cohort_ids_json, \
          all_cohorts, created_at, last_used_at, expires_at, revoked) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, NULL, ?, 0)",
    )
    .bind(row.id)
    .bind(row.tenant_id)
    .bind(row.name)
    .bind(row.token_hash)
    .bind(row.prefix)
    .bind(row.scopes_json)
    .bind(row.cohort_ids_json)
    .bind(i64::from(row.all_cohorts))
    .bind(row.created_at)
    .bind(row.expires_at)
    .execute(pool)
    .await;
    match res {
        Ok(_) => Ok(()),
        Err(sqlx::Error::Database(db_err)) if is_unique_violation(&*db_err) => {
            Err(ArgoError::Invariant(format!(
                "api_key token_hash collision: {}",
                row.token_hash
            )))
        }
        Err(e) => Err(ArgoError::Provider(format!("api_key insert: {e}"))),
    }
}

fn is_unique_violation(err: &dyn sqlx::error::DatabaseError) -> bool {
    err.code().is_some_and(|c| c == "2067" || c == "1555")
}

const SELECT_COLS: &str = "id, tenant_id, name, prefix, scopes_json, cohort_ids_json, \
     all_cohorts, created_at, last_used_at, expires_at, revoked";

type RowTuple = (
    String,
    String,
    String,
    String,
    String,
    String,
    i64,
    String,
    Option<String>,
    Option<String>,
    i64,
);

fn map_row(t: RowTuple) -> ApiKeyRow {
    ApiKeyRow {
        id: t.0,
        tenant_id: t.1,
        name: t.2,
        prefix: t.3,
        scopes_json: t.4,
        cohort_ids_json: t.5,
        all_cohorts: t.6 != 0,
        created_at: t.7,
        last_used_at: t.8,
        expires_at: t.9,
        revoked: t.10 != 0,
    }
}

/// List every API key for `tenant_id`, newest-first. The `token_hash`
/// column is deliberately **not** projected — the roster carries
/// metadata only (masked `prefix`, scopes, lifecycle), never a
/// secret-derived value. Revoked keys are included (the SPA renders a
/// revoked badge); a tenant only ever sees its own rows.
pub async fn list_for_tenant(pool: &SqlitePool, tenant_id: &str) -> ArgoResult<Vec<ApiKeyRow>> {
    let rows: Vec<RowTuple> = sqlx::query_as(&format!(
        "SELECT {SELECT_COLS} FROM api_key \
         WHERE tenant_id = ? ORDER BY created_at DESC, id ASC"
    ))
    .bind(tenant_id)
    .fetch_all(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("api_key list: {e}")))?;
    Ok(rows.into_iter().map(map_row).collect())
}

/// Fetch one key by `(tenant_id, id)`. Returns `None` when no row
/// matches — including when the id exists under a *different* tenant, so
/// this read can never surface another tenant's key. `token_hash` is not
/// projected.
pub async fn get_for_tenant(
    pool: &SqlitePool,
    tenant_id: &str,
    id: &str,
) -> ArgoResult<Option<ApiKeyRow>> {
    let row: Option<RowTuple> = sqlx::query_as(&format!(
        "SELECT {SELECT_COLS} FROM api_key WHERE tenant_id = ? AND id = ?"
    ))
    .bind(tenant_id)
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("api_key get: {e}")))?;
    Ok(row.map(map_row))
}

/// Mark a key revoked. Scoped by `(tenant_id, id)` so a caller can never
/// revoke another tenant's key — a cross-tenant `(tenant, id)` pair
/// matches no row and returns `false`. Returns `true` when a row was
/// flipped, `false` when none matched. Idempotent: re-revoking an
/// already-revoked key still returns `true` (the row matched).
pub async fn mark_revoked(pool: &SqlitePool, tenant_id: &str, id: &str) -> ArgoResult<bool> {
    let res = sqlx::query("UPDATE api_key SET revoked = 1 WHERE tenant_id = ? AND id = ?")
        .bind(tenant_id)
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("api_key revoke: {e}")))?;
    Ok(res.rows_affected() > 0)
}

/// Stamp `last_used_at` on the key whose `token_hash` matches. Scoped by
/// `token_hash` (the lookup key the gateway already resolved), updating
/// every matching row. Returns the number of rows touched.
pub async fn touch_last_used(
    pool: &SqlitePool,
    token_hash: &str,
    used_at: &str,
) -> ArgoResult<u64> {
    let res = sqlx::query("UPDATE api_key SET last_used_at = ? WHERE token_hash = ?")
        .bind(used_at)
        .bind(token_hash)
        .execute(pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("api_key touch: {e}")))?;
    Ok(res.rows_affected())
}

#[cfg(test)]
#[path = "api_key_tests.rs"]
mod tests;
