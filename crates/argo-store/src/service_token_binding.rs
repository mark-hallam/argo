//! Gateway service-token bindings — anchor a bearer token to a
//! `(principal_id, tenant_id)` pair so the gateway can derive
//! principal + tenant from the authenticated credential
//! (Mission `mission-audit-principal-tenant-from-token`, P0).
//!
//! Tokens are stored as the SHA-256 hex digest of the raw token
//! string; raw tokens never land in the database. Rows are opt-in:
//! a bearer with no binding row gets a `service:<fingerprint>`
//! synthetic principal and the workspace-default tenant in the
//! gateway path. Operator-facing CLI is `argo gateway token
//! bind|unbind|list` in `argo-cli`.
//!
//! Schema is owned by `migrations/124__service_token_binding.sql`;
//! the [`ensure_schema`] runtime helper applies the DDL idempotently
//! so callers do not need to run the slot applier separately (same
//! lazy-bootstrap pattern as `magic_link_tokens`).
//!
//! ```no_run
//! # async fn demo() -> argo_types::ArgoResult<()> {
//! use argo_store::service_token_binding::{insert_binding, lookup_binding};
//! use sqlx::sqlite::SqlitePoolOptions;
//! let pool = SqlitePoolOptions::new().connect("sqlite::memory:").await.unwrap();
//! insert_binding(
//!     &pool,
//!     "deadbeef",
//!     "human:mark@example.com",
//!     "00000000-0000-0000-0000-000000000001",
//!     Some("ci-runner"),
//!     "2026-05-20T00:00:00Z",
//! ).await?;
//! let row = lookup_binding(&pool, "deadbeef").await?;
//! assert!(row.is_some());
//! # Ok(()) }
//! ```

use argo_types::{ArgoError, ArgoResult};
use sqlx::SqlitePool;

const ENSURE_SQL: &str = include_str!("../migrations/124__service_token_binding.sql");

/// One bearer-token binding row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenBinding {
    /// SHA-256 hex digest of the raw bearer token.
    pub token_hash: String,
    /// Opaque principal identifier (`worker:<uuid>` / `human:<email>` /
    /// `system:<name>`). Validation lives in the gateway parser; this
    /// store treats it as a text label.
    pub principal_id: String,
    /// UUID-shaped tenant identifier. Stored as TEXT for SQLite
    /// portability; the gateway parses to `TenantId`.
    pub tenant_id: String,
    /// Optional operator-supplied label (e.g. `"ci-runner"`).
    pub label: Option<String>,
    /// ISO-8601 / RFC3339 timestamp captured at insert.
    pub created_at: String,
}

/// Apply the DDL idempotently. Safe to call repeatedly — every
/// statement is gated on `CREATE TABLE IF NOT EXISTS` /
/// `CREATE INDEX IF NOT EXISTS`. Mirrors `user_account::ensure_schema`'s
/// strip-comments-then-split pattern so the SQL comment header lines
/// don't reach the driver.
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
            .map_err(|e| ArgoError::Provider(format!("service_token_binding ensure: {e}")))?;
    }
    Ok(())
}

/// Insert a binding row. Caller hashes the raw token first.
/// Conflicts on `token_hash` return an `Invariant` error so callers
/// can present a clear "already bound" message.
pub async fn insert_binding(
    pool: &SqlitePool,
    token_hash: &str,
    principal_id: &str,
    tenant_id: &str,
    label: Option<&str>,
    created_at: &str,
) -> ArgoResult<()> {
    ensure_schema(pool).await?;
    let res = sqlx::query(
        "INSERT INTO service_token_binding \
         (token_hash, principal_id, tenant_id, label, created_at) \
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(token_hash)
    .bind(principal_id)
    .bind(tenant_id)
    .bind(label)
    .bind(created_at)
    .execute(pool)
    .await;
    match res {
        Ok(_) => Ok(()),
        Err(sqlx::Error::Database(db_err)) if is_unique_violation(&*db_err) => Err(
            ArgoError::Invariant(format!("token already bound: {token_hash}")),
        ),
        Err(e) => Err(ArgoError::Provider(format!(
            "service_token_binding insert: {e}"
        ))),
    }
}

fn is_unique_violation(err: &dyn sqlx::error::DatabaseError) -> bool {
    err.code().is_some_and(|c| c == "2067" || c == "1555")
}

/// Look up a binding by token hash. `None` when no row exists.
pub async fn lookup_binding(
    pool: &SqlitePool,
    token_hash: &str,
) -> ArgoResult<Option<TokenBinding>> {
    ensure_schema(pool).await?;
    let row: Option<(String, String, String, Option<String>, String)> = sqlx::query_as(
        "SELECT token_hash, principal_id, tenant_id, label, created_at \
         FROM service_token_binding WHERE token_hash = ?",
    )
    .bind(token_hash)
    .fetch_optional(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("service_token_binding lookup: {e}")))?;
    Ok(row.map(
        |(token_hash, principal_id, tenant_id, label, created_at)| TokenBinding {
            token_hash,
            principal_id,
            tenant_id,
            label,
            created_at,
        },
    ))
}

/// Remove a binding by token hash. Returns `true` if a row was
/// deleted, `false` if no matching row existed.
pub async fn delete_binding(pool: &SqlitePool, token_hash: &str) -> ArgoResult<bool> {
    ensure_schema(pool).await?;
    let res = sqlx::query("DELETE FROM service_token_binding WHERE token_hash = ?")
        .bind(token_hash)
        .execute(pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("service_token_binding delete: {e}")))?;
    Ok(res.rows_affected() > 0)
}

/// List every binding row newest-first by `created_at`. The raw
/// token / full hash columns are projected so the caller can choose
/// what to render; the CLI surface masks the hash to a leading
/// fingerprint before printing.
pub async fn list_bindings(pool: &SqlitePool) -> ArgoResult<Vec<TokenBinding>> {
    ensure_schema(pool).await?;
    let rows: Vec<(String, String, String, Option<String>, String)> = sqlx::query_as(
        "SELECT token_hash, principal_id, tenant_id, label, created_at \
         FROM service_token_binding ORDER BY created_at DESC, token_hash ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("service_token_binding list: {e}")))?;
    Ok(rows
        .into_iter()
        .map(
            |(token_hash, principal_id, tenant_id, label, created_at)| TokenBinding {
                token_hash,
                principal_id,
                tenant_id,
                label,
                created_at,
            },
        )
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn pool() -> SqlitePool {
        SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("memory pool")
    }

    #[tokio::test]
    async fn ensure_schema_is_idempotent() {
        let p = pool().await;
        ensure_schema(&p).await.unwrap();
        ensure_schema(&p).await.unwrap();
    }

    #[tokio::test]
    async fn insert_then_lookup_roundtrip() {
        let p = pool().await;
        insert_binding(
            &p,
            "abc123",
            "human:mark@example.com",
            "00000000-0000-0000-0000-000000000001",
            Some("ci-runner"),
            "2026-05-20T00:00:00Z",
        )
        .await
        .unwrap();
        let row = lookup_binding(&p, "abc123").await.unwrap().unwrap();
        assert_eq!(row.principal_id, "human:mark@example.com");
        assert_eq!(row.tenant_id, "00000000-0000-0000-0000-000000000001");
        assert_eq!(row.label.as_deref(), Some("ci-runner"));
    }

    #[tokio::test]
    async fn lookup_returns_none_for_unknown_hash() {
        let p = pool().await;
        ensure_schema(&p).await.unwrap();
        assert!(lookup_binding(&p, "ghost").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn duplicate_insert_is_invariant_error() {
        let p = pool().await;
        insert_binding(&p, "dupe", "human:a", "t", None, "now")
            .await
            .unwrap();
        let err = insert_binding(&p, "dupe", "human:b", "t", None, "now")
            .await
            .unwrap_err();
        assert!(matches!(err, ArgoError::Invariant(_)), "{err:?}");
    }

    #[tokio::test]
    async fn delete_returns_true_only_when_row_existed() {
        let p = pool().await;
        insert_binding(&p, "doomed", "human:a", "t", None, "now")
            .await
            .unwrap();
        assert!(delete_binding(&p, "doomed").await.unwrap());
        assert!(!delete_binding(&p, "doomed").await.unwrap());
    }

    #[tokio::test]
    async fn list_returns_all_bindings_newest_first() {
        let p = pool().await;
        insert_binding(&p, "a", "human:a", "t1", None, "2026-05-19T00:00:00Z")
            .await
            .unwrap();
        insert_binding(&p, "b", "human:b", "t2", None, "2026-05-20T00:00:00Z")
            .await
            .unwrap();
        let rows = list_bindings(&p).await.unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].token_hash, "b");
        assert_eq!(rows[1].token_hash, "a");
    }
}
