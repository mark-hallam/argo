//! Message append, count, list, and FTS5 search impls —
//! extracted from `lib.rs` to keep that file under the 500-LOC
//! ceiling (Stream `claude-argo-store-lib-split`). No behaviour
//! change vs. the historical inline implementations; every
//! method retains its public signature.
use crate::models::{MessageRow, MessageSearchHit, MessageSearchHitRow};
use crate::{SessionStore, StoreResult};
use argo_security::pii_detect;
use argo_types::{ArgoError, MessageRole, SessionId};
use chrono::Utc;
use uuid::Uuid;

impl SessionStore {
    /// Append a message to a session, auto-assigning sequence number.
    pub async fn append_message(
        &self,
        session_id: SessionId,
        role: MessageRole,
        content_json: &str,
    ) -> StoreResult<Uuid> {
        let id = Uuid::now_v7();
        let created_at = Utc::now().timestamp_micros();
        let role_str = match role {
            MessageRole::System => "system",
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
            MessageRole::Tool => "tool",
        };

        let scanned = pii_detect::scan_and_maybe_redact(content_json, "message_write");

        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ArgoError::Provider(format!("tx begin: {e}")))?;

        let next_seq: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(seq), 0) + 1 FROM message WHERE session_id = ?",
        )
        .bind(session_id.0.as_bytes().as_slice())
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("next_seq: {e}")))?;

        sqlx::query(
            "INSERT INTO message (id, session_id, role, content_json, created_at, seq)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(id.as_bytes().as_slice())
        .bind(session_id.0.as_bytes().as_slice())
        .bind(role_str)
        .bind(scanned.as_ref())
        .bind(created_at)
        .bind(next_seq)
        .execute(&mut *tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("insert message: {e}")))?;

        tx.commit()
            .await
            .map_err(|e| ArgoError::Provider(format!("tx commit: {e}")))?;
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
