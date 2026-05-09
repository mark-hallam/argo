//! Read-side helpers for the `recovery_cache` table populated by the
//! `argo recover` LLM-assisted post-mortem (#901). The producer side
//! lives in `argo-cli::handlers::recover` (which inserts cache rows
//! after a successful LLM call). This module exposes pure CRUD so
//! `argo recover cache list/show/clear` can inspect and invalidate
//! entries without re-calling the model.
//!
//! Schema (`migrations/28__recovery_cache.sql`):
//! ```sql
//! CREATE TABLE recovery_cache (
//!     session_id TEXT NOT NULL,
//!     chain_hash TEXT NOT NULL,
//!     suggestions_json TEXT NOT NULL,
//!     failure_kind TEXT NOT NULL DEFAULT '',
//!     cached_at_micros INTEGER NOT NULL,
//!     PRIMARY KEY (session_id, chain_hash)
//! );
//! ```

use argo_types::{ArgoError, ArgoResult};
use sqlx::{Pool, Sqlite};

/// One cache row, lightweight projection used by `list` (no
/// suggestions JSON, just metadata).
///
/// # Examples
///
/// ```no_run
/// # use argo_store::RecoveryCacheEntry;
/// let entry = RecoveryCacheEntry {
///     session_id: "11111111-1111-1111-1111-111111111111".into(),
///     chain_hash: "sha256-abc".into(),
///     failure_kind: "tool_error".into(),
///     cached_at_micros: 1_700_000_000_000_000,
/// };
/// assert_eq!(entry.failure_kind, "tool_error");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryCacheEntry {
    /// Session UUID the recovery suggestion was computed for.
    pub session_id: String,
    /// Audit-chain hash that keyed the cache entry.
    pub chain_hash: String,
    /// Optional kind of the failing audit step (e.g. `"tool_error"`),
    /// stored to help operators triage at list time.
    pub failure_kind: String,
    /// `chrono::Utc::now().timestamp_micros()` at insert time.
    pub cached_at_micros: i64,
}

/// `argo recover cache list` projection — newest-first.
///
/// `since_micros = None` lists everything; otherwise only rows with
/// `cached_at_micros >= since_micros`.
///
/// # Examples
///
/// ```no_run
/// # use argo_store::{recovery_cache, SessionStore};
/// # async fn demo() -> argo_types::ArgoResult<()> {
/// let store = SessionStore::open("argo.db").await?;
/// let entries = recovery_cache::list(store.pool(), None, Some(50)).await?;
/// for e in entries {
///     println!("{} ({})", e.session_id, e.failure_kind);
/// }
/// # Ok(()) }
/// ```
pub async fn list(
    pool: &Pool<Sqlite>,
    since_micros: Option<i64>,
    limit: Option<u32>,
) -> ArgoResult<Vec<RecoveryCacheEntry>> {
    let limit = limit.unwrap_or(100).clamp(1, 1000) as i64;
    let rows = match since_micros {
        Some(since) => {
            sqlx::query_as::<_, (String, String, String, i64)>(
                "SELECT session_id, chain_hash, failure_kind, cached_at_micros \
                 FROM recovery_cache \
                 WHERE cached_at_micros >= ? \
                 ORDER BY cached_at_micros DESC \
                 LIMIT ?",
            )
            .bind(since)
            .bind(limit)
            .fetch_all(pool)
            .await
        }
        None => {
            sqlx::query_as::<_, (String, String, String, i64)>(
                "SELECT session_id, chain_hash, failure_kind, cached_at_micros \
                 FROM recovery_cache \
                 ORDER BY cached_at_micros DESC \
                 LIMIT ?",
            )
            .bind(limit)
            .fetch_all(pool)
            .await
        }
    }
    .map_err(|e| ArgoError::Provider(format!("list recovery_cache: {e}")))?;
    Ok(rows
        .into_iter()
        .map(|(session_id, chain_hash, failure_kind, cached_at_micros)| RecoveryCacheEntry {
            session_id,
            chain_hash,
            failure_kind,
            cached_at_micros,
        })
        .collect())
}

/// Fetch the freshest cached suggestions JSON for `session_id`, if
/// any. Returns `None` when no row exists. Used by `argo recover
/// cache show` to print without re-calling the LLM.
///
/// # Examples
///
/// ```no_run
/// # use argo_store::{recovery_cache, SessionStore};
/// # async fn demo() -> argo_types::ArgoResult<()> {
/// let store = SessionStore::open("argo.db").await?;
/// if let Some((json, hash)) = recovery_cache::show(store.pool(), "abc").await? {
///     println!("chain={hash}\n{json}");
/// }
/// # Ok(()) }
/// ```
pub async fn show(
    pool: &Pool<Sqlite>,
    session_id: &str,
) -> ArgoResult<Option<(String, String)>> {
    let row: Option<(String, String)> = sqlx::query_as(
        "SELECT suggestions_json, chain_hash FROM recovery_cache \
         WHERE session_id = ? \
         ORDER BY cached_at_micros DESC \
         LIMIT 1",
    )
    .bind(session_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("show recovery_cache: {e}")))?;
    Ok(row)
}

/// Delete every cache entry for a single `session_id`. Returns the
/// number of rows removed (0 when the session had no cache).
///
/// # Examples
///
/// ```no_run
/// # use argo_store::{recovery_cache, SessionStore};
/// # async fn demo() -> argo_types::ArgoResult<()> {
/// let store = SessionStore::open("argo.db").await?;
/// let removed = recovery_cache::clear_one(store.pool(), "abc").await?;
/// println!("cleared {removed} entries");
/// # Ok(()) }
/// ```
pub async fn clear_one(pool: &Pool<Sqlite>, session_id: &str) -> ArgoResult<u64> {
    let res = sqlx::query("DELETE FROM recovery_cache WHERE session_id = ?")
        .bind(session_id)
        .execute(pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("clear recovery_cache: {e}")))?;
    Ok(res.rows_affected())
}

/// Truncate the entire `recovery_cache` table. Returns the number of
/// rows removed. Caller must enforce typed confirmation — this
/// function does no extra check.
///
/// # Examples
///
/// ```no_run
/// # use argo_store::{recovery_cache, SessionStore};
/// # async fn demo() -> argo_types::ArgoResult<()> {
/// let store = SessionStore::open("argo.db").await?;
/// let removed = recovery_cache::clear_all(store.pool()).await?;
/// println!("wiped {removed} rows");
/// # Ok(()) }
/// ```
pub async fn clear_all(pool: &Pool<Sqlite>) -> ArgoResult<u64> {
    let res = sqlx::query("DELETE FROM recovery_cache")
        .execute(pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("clear_all recovery_cache: {e}")))?;
    Ok(res.rows_affected())
}

/// Delete entries with `cached_at_micros < before_micros`. Returns
/// the number of rows removed. The caller computes the cutoff from
/// `--older-than <duration>` flag.
///
/// # Examples
///
/// ```no_run
/// # use argo_store::{recovery_cache, SessionStore};
/// # async fn demo() -> argo_types::ArgoResult<()> {
/// let store = SessionStore::open("argo.db").await?;
/// let cutoff = chrono::Utc::now().timestamp_micros() - 7 * 86_400 * 1_000_000;
/// let removed = recovery_cache::clear_older_than(store.pool(), cutoff).await?;
/// println!("evicted {removed} rows older than 7 days");
/// # Ok(()) }
/// ```
pub async fn clear_older_than(pool: &Pool<Sqlite>, before_micros: i64) -> ArgoResult<u64> {
    let res = sqlx::query("DELETE FROM recovery_cache WHERE cached_at_micros < ?")
        .bind(before_micros)
        .execute(pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("evict old recovery_cache: {e}")))?;
    Ok(res.rows_affected())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SessionStore;
    use chrono::Utc;

    async fn open_with_table() -> SessionStore {
        // The recovery_cache table now lives in `migrations/v36.rs`;
        // SessionStore::migrate installs it as part of the v12..v40
        // ladder, no raw SQL replay required.
        let store = SessionStore::open(":memory:").await.unwrap();
        store.migrate().await.unwrap();
        store
    }

    async fn seed(store: &SessionStore, session_id: &str, chain_hash: &str, micros: i64) {
        sqlx::query(
            "INSERT OR REPLACE INTO recovery_cache \
             (session_id, chain_hash, suggestions_json, failure_kind, cached_at_micros) \
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(session_id)
        .bind(chain_hash)
        .bind(r#"{"likely_cause":"x","fixes":[]}"#)
        .bind("tool_error")
        .bind(micros)
        .execute(store.pool())
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn list_returns_newest_first_within_limit() {
        let store = open_with_table().await;
        let now = Utc::now().timestamp_micros();
        seed(&store, "s-1", "h-1", now - 3_000_000).await;
        seed(&store, "s-2", "h-2", now - 2_000_000).await;
        seed(&store, "s-3", "h-3", now - 1_000_000).await;
        let entries = list(store.pool(), None, Some(2)).await.unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].session_id, "s-3");
        assert_eq!(entries[1].session_id, "s-2");
    }

    #[tokio::test]
    async fn list_with_since_filter_drops_older_rows() {
        let store = open_with_table().await;
        let now = Utc::now().timestamp_micros();
        seed(&store, "old", "h-old", now - 10_000_000).await;
        seed(&store, "new", "h-new", now - 1_000_000).await;
        let entries = list(store.pool(), Some(now - 5_000_000), None).await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].session_id, "new");
    }

    #[tokio::test]
    async fn show_returns_freshest_for_session() {
        let store = open_with_table().await;
        let now = Utc::now().timestamp_micros();
        seed(&store, "s-1", "h-old", now - 5_000_000).await;
        seed(&store, "s-1", "h-new", now - 1_000_000).await;
        let row = show(store.pool(), "s-1").await.unwrap();
        let (_json, hash) = row.unwrap();
        assert_eq!(hash, "h-new");
    }

    #[tokio::test]
    async fn show_returns_none_for_unknown_session() {
        let store = open_with_table().await;
        let row = show(store.pool(), "missing").await.unwrap();
        assert!(row.is_none());
    }

    #[tokio::test]
    async fn clear_one_removes_only_named_session() {
        let store = open_with_table().await;
        let now = Utc::now().timestamp_micros();
        seed(&store, "keep", "h-keep", now).await;
        seed(&store, "drop", "h-1", now).await;
        seed(&store, "drop", "h-2", now).await;
        let removed = clear_one(store.pool(), "drop").await.unwrap();
        assert_eq!(removed, 2);
        let remaining = list(store.pool(), None, None).await.unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].session_id, "keep");
    }

    #[tokio::test]
    async fn clear_older_than_evicts_only_old_rows() {
        let store = open_with_table().await;
        let now = Utc::now().timestamp_micros();
        seed(&store, "young", "h-y", now).await;
        seed(&store, "old", "h-o", now - 30 * 24 * 3_600 * 1_000_000_i64).await;
        let cutoff = now - 7 * 24 * 3_600 * 1_000_000_i64;
        let removed = clear_older_than(store.pool(), cutoff).await.unwrap();
        assert_eq!(removed, 1);
        let remaining = list(store.pool(), None, None).await.unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].session_id, "young");
    }

    #[tokio::test]
    async fn clear_all_truncates_table() {
        let store = open_with_table().await;
        let now = Utc::now().timestamp_micros();
        seed(&store, "s-1", "h-1", now).await;
        seed(&store, "s-2", "h-2", now).await;
        let removed = clear_all(store.pool()).await.unwrap();
        assert_eq!(removed, 2);
        assert!(list(store.pool(), None, None).await.unwrap().is_empty());
    }
}
