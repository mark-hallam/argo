//! Message append, count, list, and FTS5 search impls —
//! extracted from `lib.rs` to keep that file under the 500-LOC
//! ceiling (Stream `claude-argo-store-lib-split`). The append
//! path also coalesces concurrent writers into a single
//! transaction via `crate::batched_writer::BatchedWriter`
//! (Stream `claude-perf-store-batched-writes`).
use crate::batched_writer;
use crate::models::{MessageRow, MessageSearchHit, MessageSearchHitRow};
use crate::{SessionStore, StoreResult};
use argo_security::pii_detect;
use argo_types::{ArgoError, ArgoResult, MessageRole, SessionId};
use chrono::Utc;
use sqlx::{Pool, Sqlite};
use std::collections::HashMap;
use uuid::Uuid;

/// One pending message row buffered by the append-path
/// `BatchedWriter`. Constructed by [`SessionStore::append_message`]
/// at submit time so the call returns the row's `Uuid` synchronously
/// while the actual SQL insert happens inside the next flush.
pub(crate) struct MessageItem {
    pub(crate) id: Uuid,
    pub(crate) session_id: SessionId,
    pub(crate) role: &'static str,
    pub(crate) content_json: String,
    pub(crate) created_at: i64,
}

impl SessionStore {
    /// Append a message to a session, auto-assigning sequence number.
    ///
    /// Concurrent appends are coalesced through a per-store
    /// `BatchedWriter` so multiple in-flight callers share one
    /// `BEGIN`/`COMMIT`. Set `ARGO_STORE_BATCHED_WRITES=0` to fall
    /// back to the historical per-row insert (used by tests that
    /// read a row immediately after appending it without first
    /// calling [`SessionStore::flush_pending_writes`]).
    pub async fn append_message(
        &self,
        session_id: SessionId,
        role: MessageRole,
        content_json: &str,
    ) -> StoreResult<Uuid> {
        let id = Uuid::now_v7();
        let created_at = Utc::now().timestamp_micros();
        let role_str = role_label(role);
        let scanned = pii_detect::scan_and_maybe_redact(content_json, "message_write").into_owned();

        if batched_writer::is_batching_enabled() {
            let item = MessageItem {
                id,
                session_id,
                role: role_str,
                content_json: scanned,
                created_at,
            };
            self.messages.submit(item).await?;
            return Ok(id);
        }

        // Fallback: per-row insert in its own transaction. Identical
        // to the historical path so tests / callers that opt out via
        // env knob see no behaviour change.
        insert_one(&self.pool, &MessageItem {
            id,
            session_id,
            role: role_str,
            content_json: scanned,
            created_at,
        })
        .await?;
        Ok(id)
    }

    /// Count messages in a session.
    pub async fn message_count(&self, session_id: SessionId) -> StoreResult<u64> {
        let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM message WHERE session_id = ?")
            .bind(session_id.0.as_bytes().as_slice())
            .fetch_one(&self.pool)
            .await
            .map_err(|e| ArgoError::Provider(format!("message_count: {e}")))?;
        Ok(u64::try_from(n).unwrap_or(0))
    }

    /// Stream a session's messages in `seq` order. Used by
    /// `argo session export` to dump sessions to JSONL.
    pub async fn list_messages(&self, session_id: SessionId) -> StoreResult<Vec<MessageRow>> {
        let rows = sqlx::query_as::<_, MessageRow>(
            "SELECT id, session_id, role, content_json, tool_calls_json, \
             tool_call_id, reasoning, provider_meta, created_at, seq \
             FROM message WHERE session_id = ? ORDER BY seq ASC",
        )
        .bind(session_id.0.as_bytes().as_slice())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("list_messages: {e}")))?;
        Ok(rows)
    }

    /// Full-text search the message log via FTS5. Returns the
    /// per-row `(session_id, seq, role, snippet)` for the top
    /// `limit` matches across all sessions, ordered by relevance.
    /// Used by `argo session search`.
    pub async fn search_messages(
        &self,
        query: &str,
        limit: u32,
    ) -> StoreResult<Vec<MessageSearchHit>> {
        let rows: Vec<MessageSearchHitRow> = sqlx::query_as::<_, MessageSearchHitRow>(
            "SELECT m.session_id, m.seq, m.role, \
             snippet(message_fts, 0, '[', ']', '…', 32) AS snippet, \
             m.created_at \
             FROM message_fts JOIN message m ON m.rowid = message_fts.rowid \
             WHERE message_fts MATCH ? ORDER BY rank LIMIT ?",
        )
        .bind(query)
        .bind(i64::from(limit))
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("search_messages: {e}")))?;
        Ok(rows.into_iter().map(MessageSearchHit::from).collect())
    }

    /// FTS5 search scoped to a single session. Same shape as
    /// [`Self::search_messages`] but adds a `session_id` filter.
    pub async fn search_messages_in_session(
        &self,
        session_id: SessionId,
        query: &str,
        limit: u32,
    ) -> StoreResult<Vec<MessageSearchHit>> {
        let rows: Vec<MessageSearchHitRow> = sqlx::query_as::<_, MessageSearchHitRow>(
            "SELECT m.session_id, m.seq, m.role, \
             snippet(message_fts, 0, '[', ']', '…', 32) AS snippet, \
             m.created_at \
             FROM message_fts JOIN message m ON m.rowid = message_fts.rowid \
             WHERE message_fts MATCH ? AND m.session_id = ? \
             ORDER BY rank LIMIT ?",
        )
        .bind(query)
        .bind(session_id.0.as_bytes().as_slice())
        .bind(i64::from(limit))
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("search_messages_in_session: {e}")))?;
        Ok(rows.into_iter().map(MessageSearchHit::from).collect())
    }
}

/// Map the `MessageRole` enum to the static role string the schema
/// expects. Pulled out so the batched-writer item can carry it as a
/// `&'static str` instead of allocating per row.
fn role_label(role: MessageRole) -> &'static str {
    match role {
        MessageRole::System => "system",
        MessageRole::User => "user",
        MessageRole::Assistant => "assistant",
        MessageRole::Tool => "tool",
    }
}

/// Single-row insert path. Used by both the env-knob opt-out branch
/// of [`SessionStore::append_message`] and (indirectly) the
/// batched flush path below — they share the SQL but the batched
/// path inlines its own loop to amortize the `next_seq` lookup.
async fn insert_one(pool: &Pool<Sqlite>, item: &MessageItem) -> StoreResult<()> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| ArgoError::Provider(format!("tx begin: {e}")))?;
    let next_seq: i64 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(seq), 0) + 1 FROM message WHERE session_id = ?",
    )
    .bind(item.session_id.0.as_bytes().as_slice())
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| ArgoError::Provider(format!("next_seq: {e}")))?;
    sqlx::query(
        "INSERT INTO message (id, session_id, role, content_json, created_at, seq)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(item.id.as_bytes().as_slice())
    .bind(item.session_id.0.as_bytes().as_slice())
    .bind(item.role)
    .bind(&item.content_json)
    .bind(item.created_at)
    .bind(next_seq)
    .execute(&mut *tx)
    .await
    .map_err(|e| ArgoError::Provider(format!("insert message: {e}")))?;
    tx.commit()
        .await
        .map_err(|e| ArgoError::Provider(format!("tx commit: {e}")))?;
    Ok(())
}

/// Flush-fn for the per-store message `BatchedWriter`. Groups items
/// by `session_id` so the per-session `seq` is computed with one
/// `MAX(seq)` lookup per session per batch (vs. per row), then
/// issues all inserts inside one `BEGIN`/`COMMIT`.
pub(crate) async fn flush_messages(
    pool: Pool<Sqlite>,
    batch: Vec<MessageItem>,
) -> ArgoResult<()> {
    if batch.is_empty() {
        return Ok(());
    }
    // Group by session_id so each session pays a single
    // `MAX(seq)` lookup per batch even when many sessions are
    // appending concurrently. `SessionId` does not implement
    // `Hash`/`Ord` itself, so key by the underlying 16-byte uuid
    // bytes — the sort order across sessions doesn't matter, only
    // that ordering *within* a session is preserved (Vec::push
    // does that).
    let mut by_session: HashMap<[u8; 16], Vec<MessageItem>> = HashMap::new();
    for item in batch {
        by_session
            .entry(*item.session_id.0.as_bytes())
            .or_default()
            .push(item);
    }
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| ArgoError::Provider(format!("tx begin: {e}")))?;
    for (session_blob, items) in by_session {
        let next_seq: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(seq), 0) + 1 FROM message WHERE session_id = ?",
        )
        .bind(session_blob.as_slice())
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("next_seq: {e}")))?;
        for (offset, item) in items.iter().enumerate() {
            let seq = next_seq + i64::try_from(offset).unwrap_or(0);
            sqlx::query(
                "INSERT INTO message (id, session_id, role, content_json, created_at, seq)
                 VALUES (?, ?, ?, ?, ?, ?)",
            )
            .bind(item.id.as_bytes().as_slice())
            .bind(item.session_id.0.as_bytes().as_slice())
            .bind(item.role)
            .bind(&item.content_json)
            .bind(item.created_at)
            .bind(seq)
            .execute(&mut *tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("insert message: {e}")))?;
        }
    }
    tx.commit()
        .await
        .map_err(|e| ArgoError::Provider(format!("tx commit: {e}")))?;
    Ok(())
}
