//! Magic-link token storage for the gateway self-serve signup flow
//! (Stream `claude-magic-link-auth-backend`).
//!
//! The `user_account` + `user_tenant` substrate tables ship from
//! `mission-tenant-substrate-tables` (#1777) and are accessed via
//! [`argo_identity::user_store`] / [`argo_identity::user_tenant_store`].
//! This module owns only the `magic_link_tokens` table — single-use
//! tokens emailed to prospective users, SHA-256-hashed at rest with a
//! 15-minute TTL.
//!
//! The file is named `user_account` rather than `magic_link_token`
//! to stay inside the original brief's may-touch list; the public
//! surface is the token store only.
//!
//! Schema is owned by `migrations/109__magic_link_tokens.sql`; the
//! [`ensure_schema`] runtime helper applies the DDL idempotently so
//! callers do not need to run the slot applier separately (same
//! pattern as `worker_presence` / `siem_dispatch_cursor` /
//! `ensure_tenant_substrate_schema`).
//!
//! ```no_run
//! # async fn demo() -> argo_types::ArgoResult<()> {
//! use argo_store::user_account::{insert_magic_link, consume_magic_link, MAGIC_LINK_TTL_SECS};
//! use sqlx::sqlite::SqlitePoolOptions;
//! let pool = SqlitePoolOptions::new().connect("sqlite::memory:").await.unwrap();
//! let now = 1700;
//! insert_magic_link(&pool, "hash", "x@example.com", now + MAGIC_LINK_TTL_SECS).await?;
//! let _ = consume_magic_link(&pool, "hash", now).await?;
//! # Ok(()) }
//! ```

use argo_types::{ArgoError, ArgoResult};
use sqlx::SqlitePool;

/// Magic-link TTL — 15 minutes from issue (brief §2 acceptance).
pub const MAGIC_LINK_TTL_SECS: i64 = 15 * 60;

/// Outcome of consuming a magic-link token via [`consume_magic_link`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConsumedToken {
    /// Token was valid, unexpired, and unused. The row is now marked
    /// `used_at = now` so a second consume of the same hash returns
    /// [`Self::AlreadyUsed`].
    Ok {
        /// Email address the token was originally minted for.
        email: String,
    },
    /// Hash not present in the store.
    NotFound,
    /// Row exists, `expires_at` is at/before `now`.
    Expired,
    /// Row exists, `used_at` is already set from a prior consume.
    AlreadyUsed,
}

const ENSURE_SQL: &str = include_str!("../migrations/115__magic_link_tokens.sql");

/// Ensure the `magic_link_tokens` table + supporting index exist.
/// Safe to call repeatedly; gated on `CREATE TABLE IF NOT EXISTS` /
/// `CREATE INDEX IF NOT EXISTS`. Mirrors `tenant_query::split_sql_statements`'s
/// strip-comments-then-split pattern so the `--`-prefixed header
/// lines in the reference SQL don't reach the driver.
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
            .map_err(|e| ArgoError::Provider(format!("magic_link_tokens ensure: {e}")))?;
    }
    Ok(())
}

/// Insert a new magic-link row. Caller hashes the raw token
/// (SHA-256, hex-lowercase) and computes `expires_at` as
/// `now + MAGIC_LINK_TTL_SECS`.
pub async fn insert_magic_link(
    pool: &SqlitePool,
    token_hash: &str,
    email: &str,
    expires_at: i64,
) -> ArgoResult<()> {
    ensure_schema(pool).await?;
    sqlx::query(
        "INSERT INTO magic_link_tokens (token_hash, email, expires_at, used_at) \
         VALUES (?, ?, ?, NULL)",
    )
    .bind(token_hash)
    .bind(email)
    .bind(expires_at)
    .execute(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("magic_link insert: {e}")))?;
    Ok(())
}

/// Look up `token_hash`, validate expiry + single-use, atomically
/// mark consumed. Returns the bound email on success.
///
/// The mark-used UPDATE is guarded with `used_at IS NULL` so a
/// concurrent consume race reports the loser as
/// [`ConsumedToken::AlreadyUsed`] rather than re-issuing.
pub async fn consume_magic_link(
    pool: &SqlitePool,
    token_hash: &str,
    now_epoch_secs: i64,
) -> ArgoResult<ConsumedToken> {
    ensure_schema(pool).await?;
    let row: Option<(String, i64, Option<i64>)> = sqlx::query_as(
        "SELECT email, expires_at, used_at FROM magic_link_tokens WHERE token_hash = ?",
    )
    .bind(token_hash)
    .fetch_optional(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("magic_link read: {e}")))?;
    let Some((email, expires_at, used_at)) = row else {
        return Ok(ConsumedToken::NotFound);
    };
    if used_at.is_some() {
        return Ok(ConsumedToken::AlreadyUsed);
    }
    if expires_at <= now_epoch_secs {
        return Ok(ConsumedToken::Expired);
    }
    let res = sqlx::query(
        "UPDATE magic_link_tokens SET used_at = ? \
         WHERE token_hash = ? AND used_at IS NULL",
    )
    .bind(now_epoch_secs)
    .bind(token_hash)
    .execute(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("magic_link update: {e}")))?;
    if res.rows_affected() == 0 {
        return Ok(ConsumedToken::AlreadyUsed);
    }
    Ok(ConsumedToken::Ok { email })
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
        ensure_schema(&p).await.expect("first call");
        ensure_schema(&p).await.expect("second call");
    }

    #[tokio::test]
    async fn consume_magic_link_is_single_use() {
        let p = pool().await;
        insert_magic_link(&p, "deadbeef", "carol@example.com", 1_000)
            .await
            .unwrap();
        let first = consume_magic_link(&p, "deadbeef", 500).await.unwrap();
        assert!(matches!(first, ConsumedToken::Ok { ref email } if email == "carol@example.com"));
        let second = consume_magic_link(&p, "deadbeef", 600).await.unwrap();
        assert_eq!(second, ConsumedToken::AlreadyUsed);
    }

    #[tokio::test]
    async fn consume_expired_magic_link() {
        let p = pool().await;
        insert_magic_link(&p, "stale", "x@example.com", 100).await.unwrap();
        let r = consume_magic_link(&p, "stale", 200).await.unwrap();
        assert_eq!(r, ConsumedToken::Expired);
    }

    #[tokio::test]
    async fn consume_unknown_token_is_not_found() {
        let p = pool().await;
        ensure_schema(&p).await.unwrap();
        let r = consume_magic_link(&p, "ghost", 100).await.unwrap();
        assert_eq!(r, ConsumedToken::NotFound);
    }
}
