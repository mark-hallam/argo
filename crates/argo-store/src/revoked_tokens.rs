//! Session-JWT revocation list backing the
//! `middleware_session_verify` cookie-auth gate
//! (mission-revocation-with-verify-middleware-v1x).
//!
//! The `__Host-session` cookie JWT carries a uuid-v7 `jti` claim. When
//! an operator hits `POST /api/v1/auth/logout`, the handler decodes
//! the cookie, calls [`revoke`] with the `jti` + tenant binding, and
//! returns a cookie-clear response. Every subsequent authenticated
//! request runs the verify-middleware which calls [`is_revoked`]; a
//! `true` answer 401s the request regardless of the cookie's
//! signature + `exp` validity.
//!
//! Schema is owned by `migrations/128__revoked_tokens.sql`; the
//! [`ensure_schema`] runtime helper applies the DDL idempotently so
//! callers do not need to run the slot applier separately — same
//! lazy-bootstrap pattern as `user_account` (slot 115) and
//! `data_export_job_query` (slot 116).
//!
//! ```no_run
//! # async fn demo() -> argo_types::ArgoResult<()> {
//! use argo_store::revoked_tokens::{revoke, is_revoked};
//! use sqlx::sqlite::SqlitePoolOptions;
//! let pool = SqlitePoolOptions::new().connect("sqlite::memory:").await.unwrap();
//! revoke(&pool, "01HW...jti", "tenant-a", Some("user-1"), Some("logout"), 1_700).await?;
//! assert!(is_revoked(&pool, "01HW...jti").await?);
//! # Ok(()) }
//! ```

use argo_types::{ArgoError, ArgoResult};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

const ENSURE_SQL: &str = include_str!("../migrations/128__revoked_tokens.sql");

/// One row of the `revoked_tokens` table — emitted by
/// [`list_for_tenant`] so the operator audit-pane can surface who
/// revoked what and when.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RevokedToken {
    /// Uuid v7 minted into the cookie JWT's `jti` claim.
    pub jti: String,
    /// Tenant the original session was bound to.
    pub tenant_id: String,
    /// Epoch seconds the revocation row was inserted.
    pub revoked_at: i64,
    /// Optional principal (user_id or operator) that triggered the revoke.
    pub principal: Option<String>,
    /// Optional free-form reason (`"logout"`, `"admin-revoke"`, …).
    pub reason: Option<String>,
}

/// Ensure the `revoked_tokens` table + supporting indexes exist.
/// Safe to call repeatedly; the DDL is gated on
/// `CREATE TABLE IF NOT EXISTS` / `CREATE INDEX IF NOT EXISTS`.
/// Strips `--` comment lines before splitting on `;` to match the
/// [`crate::user_account::ensure_schema`] pattern.
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
            .map_err(|e| ArgoError::Provider(format!("revoked_tokens ensure: {e}")))?;
    }
    Ok(())
}

/// Insert (or no-op upsert) a revocation row for `jti`. Double-revoke
/// of the same `jti` is idempotent: the second call leaves the
/// original `revoked_at` / `principal` / `reason` intact (ON CONFLICT
/// DO NOTHING) so audit ordering matches the first logout.
pub async fn revoke(
    pool: &SqlitePool,
    jti: &str,
    tenant_id: &str,
    principal: Option<&str>,
    reason: Option<&str>,
    revoked_at: i64,
) -> ArgoResult<()> {
    ensure_schema(pool).await?;
    sqlx::query(
        "INSERT INTO revoked_tokens (jti, tenant_id, revoked_at, principal, reason) \
         VALUES (?, ?, ?, ?, ?) \
         ON CONFLICT(jti) DO NOTHING",
    )
    .bind(jti)
    .bind(tenant_id)
    .bind(revoked_at)
    .bind(principal)
    .bind(reason)
    .execute(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("revoked_tokens insert: {e}")))?;
    Ok(())
}

/// Returns `true` when `jti` is present in `revoked_tokens`. The
/// verify-middleware calls this on every authenticated request that
/// presents a `__Host-session` cookie; a `true` answer drives the
/// 401 reject path. Calls [`ensure_schema`] eagerly so first boot in
/// a fresh DB does not falsely report "not revoked" via a SQLite
/// `no such table` error.
pub async fn is_revoked(pool: &SqlitePool, jti: &str) -> ArgoResult<bool> {
    ensure_schema(pool).await?;
    let row: Option<(i64,)> = sqlx::query_as("SELECT 1 FROM revoked_tokens WHERE jti = ?")
        .bind(jti)
        .fetch_optional(pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("revoked_tokens read: {e}")))?;
    Ok(row.is_some())
}

/// List revocation rows bound to `tenant_id`, ORDER BY revoked_at
/// DESC. Caller is the operator-facing audit pane / `argo session
/// revocations` CLI surface.
pub async fn list_for_tenant(
    pool: &SqlitePool,
    tenant_id: &str,
) -> ArgoResult<Vec<RevokedToken>> {
    ensure_schema(pool).await?;
    let rows: Vec<RevokedTokenRow> = sqlx::query_as(
        "SELECT jti, tenant_id, revoked_at, principal, reason \
         FROM revoked_tokens WHERE tenant_id = ? ORDER BY revoked_at DESC",
    )
    .bind(tenant_id)
    .fetch_all(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("revoked_tokens list: {e}")))?;
    Ok(rows.into_iter().map(RevokedTokenRow::into_revoked).collect())
}

/// Internal sqlx decode shim — sqlx-derived `FromRow` keeps the
/// 5-column tuple type out of [`list_for_tenant`]'s signature so
/// clippy's `type_complexity` lint stays happy without splitting the
/// function across modules.
#[derive(sqlx::FromRow)]
struct RevokedTokenRow {
    jti: String,
    tenant_id: String,
    revoked_at: i64,
    principal: Option<String>,
    reason: Option<String>,
}

impl RevokedTokenRow {
    fn into_revoked(self) -> RevokedToken {
        RevokedToken {
            jti: self.jti,
            tenant_id: self.tenant_id,
            revoked_at: self.revoked_at,
            principal: self.principal,
            reason: self.reason,
        }
    }
}

#[cfg(test)]
#[path = "revoked_tokens_tests.rs"]
mod tests;
