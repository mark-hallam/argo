//! First-class, tenant + worker-scoped store for a worker's open
//! conversation threads — the Rust home for the "Active conversations"
//! panel on the worker Memory tab (Frontier-B depth, worker-memory
//! surfaces).
//!
//! A row is one thread the worker is mid-conversation on: a counterparty
//! handle, a topic snippet, the timestamp of the last message, and an
//! optional audit drill-in target. The schema is owned by the versioned
//! migration `v87`; callers do not bootstrap it — it is applied by the
//! standard migration runner like every other `vNN` table.
//!
//! Every query is keyed by **both** `tenant_id` and `worker_id`, so one
//! tenant can never read another tenant's threads and the gateway read is
//! scoped to a single worker (core invariant #2 — multi-tenant by
//! construction). `worker_id` is the 16-byte [`crate::WorkerId`] blob,
//! matching the `memory_entry.worker_id` encoding the facts panel already
//! reads, so both halves of the Memory tab share one id convention.
//!
//! ```no_run
//! # async fn demo() -> argo_types::ArgoResult<()> {
//! use argo_store::conversation_thread::{insert, list_for_worker, ConversationThreadInsert};
//! use sqlx::sqlite::SqlitePoolOptions;
//! // In production the `worker_conversation_thread` table is created by
//! // migration v87 via the standard runner (`SessionStore::migrate`);
//! // this snippet is compile-only (`no_run`).
//! let pool = SqlitePoolOptions::new().connect("sqlite::memory:").await.unwrap();
//! let worker_id = [0u8; 16];
//! insert(&pool, &ConversationThreadInsert {
//!     id: "ct_1",
//!     tenant_id: "00000000-0000-0000-0000-000000000001",
//!     worker_id: &worker_id,
//!     counterparty_handle: "Diego Alvarez",
//!     topic: "Interview scheduling — Thu 10:30",
//!     last_message_at: 1_717_200_000_000_000,
//!     audit_href: Some("/audit/action/ct_1"),
//! }).await?;
//! let rows = list_for_worker(
//!     &pool, "00000000-0000-0000-0000-000000000001", &worker_id, 10,
//! ).await?;
//! assert_eq!(rows.len(), 1);
//! # Ok(()) }
//! ```

use argo_types::{ArgoError, ArgoResult};
use sqlx::SqlitePool;

/// One conversation-thread row as stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversationThreadRow {
    /// Opaque thread identifier (`ct_<...>`). Primary key.
    pub id: String,
    /// UUID-shaped tenant identifier. Stored as TEXT for SQLite
    /// portability; the gateway parses to `TenantId`.
    pub tenant_id: String,
    /// 16-byte worker-id blob (matches `memory_entry.worker_id`).
    pub worker_id: Vec<u8>,
    /// Counterparty display handle — PII-redacted at render.
    pub counterparty_handle: String,
    /// Short topic snippet — also redacted at render.
    pub topic: String,
    /// Unix-micros timestamp of the last message; newest-first sort key.
    pub last_message_at: i64,
    /// Optional audit drill-in target (e.g. `/audit/action/<id>`).
    pub audit_href: Option<String>,
}

/// Field bundle for [`insert`].
#[derive(Debug, Clone)]
pub struct ConversationThreadInsert<'a> {
    /// Opaque thread identifier (`ct_<...>`).
    pub id: &'a str,
    /// UUID-shaped tenant identifier.
    pub tenant_id: &'a str,
    /// 16-byte worker-id blob (matches `memory_entry.worker_id`).
    pub worker_id: &'a [u8],
    /// Counterparty display handle.
    pub counterparty_handle: &'a str,
    /// Short topic snippet.
    pub topic: &'a str,
    /// Unix-micros timestamp of the last message.
    pub last_message_at: i64,
    /// Optional audit drill-in target.
    pub audit_href: Option<&'a str>,
}

/// Insert one conversation-thread row. A primary-key collision on `id`
/// returns an [`ArgoError::Invariant`] so a caller can regenerate and
/// retry; all other database errors surface as [`ArgoError::Provider`].
pub async fn insert(pool: &SqlitePool, row: &ConversationThreadInsert<'_>) -> ArgoResult<()> {
    let res = sqlx::query(
        "INSERT INTO worker_conversation_thread \
         (id, tenant_id, worker_id, counterparty_handle, topic, last_message_at, audit_href) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(row.id)
    .bind(row.tenant_id)
    .bind(row.worker_id)
    .bind(row.counterparty_handle)
    .bind(row.topic)
    .bind(row.last_message_at)
    .bind(row.audit_href)
    .execute(pool)
    .await;
    match res {
        Ok(_) => Ok(()),
        Err(sqlx::Error::Database(db_err)) if is_unique_violation(&*db_err) => Err(
            ArgoError::Invariant(format!("worker_conversation_thread id collision: {}", row.id)),
        ),
        Err(e) => Err(ArgoError::Provider(format!(
            "worker_conversation_thread insert: {e}"
        ))),
    }
}

fn is_unique_violation(err: &dyn sqlx::error::DatabaseError) -> bool {
    err.code().is_some_and(|c| c == "2067" || c == "1555")
}

const SELECT_COLS: &str =
    "id, tenant_id, worker_id, counterparty_handle, topic, last_message_at, audit_href";

type RowTuple = (String, String, Vec<u8>, String, String, i64, Option<String>);

fn map_row(t: RowTuple) -> ConversationThreadRow {
    ConversationThreadRow {
        id: t.0,
        tenant_id: t.1,
        worker_id: t.2,
        counterparty_handle: t.3,
        topic: t.4,
        last_message_at: t.5,
        audit_href: t.6,
    }
}

/// List a worker's conversation threads, newest-first, capped at `limit`.
/// Scoped by `(tenant_id, worker_id)` so a caller can never read another
/// tenant's — or another worker's — threads. Returns an empty vec when the
/// worker has none (the panel renders its honest empty state).
pub async fn list_for_worker(
    pool: &SqlitePool,
    tenant_id: &str,
    worker_id: &[u8],
    limit: i64,
) -> ArgoResult<Vec<ConversationThreadRow>> {
    let rows: Vec<RowTuple> = sqlx::query_as(&format!(
        "SELECT {SELECT_COLS} FROM worker_conversation_thread \
         WHERE tenant_id = ? AND worker_id = ? \
         ORDER BY last_message_at DESC, id ASC LIMIT ?"
    ))
    .bind(tenant_id)
    .bind(worker_id)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("worker_conversation_thread list: {e}")))?;
    Ok(rows.into_iter().map(map_row).collect())
}

/// Count a worker's conversation threads under `tenant_id`. Used by the
/// seed's idempotency guard (skip the whole block if any row exists).
pub async fn count_for_worker(
    pool: &SqlitePool,
    tenant_id: &str,
    worker_id: &[u8],
) -> ArgoResult<i64> {
    sqlx::query_scalar(
        "SELECT COUNT(*) FROM worker_conversation_thread \
         WHERE tenant_id = ? AND worker_id = ?",
    )
    .bind(tenant_id)
    .bind(worker_id)
    .fetch_one(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("worker_conversation_thread count: {e}")))
}

#[cfg(test)]
#[path = "conversation_thread_tests.rs"]
mod tests;
