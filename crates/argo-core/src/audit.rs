//! Audit sink — append-only hash-chained log of agent-loop events
//! (§17 minimum). Phase 1 ships a sqlite-backed writer; full structured
//! event taxonomy (§17.1) lands with Agent N.
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, SessionId};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use uuid::Uuid;

/// Kind of event being audited. Phase 1 covers turn boundaries + tool
/// dispatch; each is hash-chained to its predecessor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditKind {
    /// `run_turn` started.
    TurnStart,
    /// Assistant produced a message (text or tool calls).
    AssistantMessage,
    /// A tool call was dispatched.
    ToolCall,
    /// A tool result came back.
    ToolResult,
    /// `run_turn` ended (with a final answer or halt).
    TurnEnd,
    /// A halt event (max iterations, error, etc.).
    Halt,
}

impl AuditKind {
    fn label(self) -> &'static str {
        match self {
            Self::TurnStart => "turn_start",
            Self::AssistantMessage => "assistant_message",
            Self::ToolCall => "tool_call",
            Self::ToolResult => "tool_result",
            Self::TurnEnd => "turn_end",
            Self::Halt => "halt",
        }
    }
}

/// Append-only hash-chained audit sink.
pub struct AuditSink {
    store: Arc<SessionStore>,
}

impl std::fmt::Debug for AuditSink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuditSink").finish()
    }
}

impl AuditSink {
    /// Build over an open session store.
    #[must_use]
    pub fn new(store: Arc<SessionStore>) -> Self {
        Self { store }
    }

    /// Write one event.
    ///
    /// Computes `self_hash = sha256(parent_hash || canonical_payload)`
    /// where `parent_hash` is the previous row's `self_hash` for the
    /// same session, or zeroes if no predecessor exists.
    pub async fn write(
        &self,
        session_id: Option<SessionId>,
        kind: AuditKind,
        payload: serde_json::Value,
    ) -> ArgoResult<()> {
        let id = Uuid::now_v7();
        let parent_hash = self.last_hash_for(session_id).await?;
        let canonical = serde_json::to_string(&payload)
            .map_err(|e| ArgoError::Schema(format!("audit payload serialize: {e}")))?;
        let self_hash = chain_hash(&parent_hash, kind.label(), &canonical);
        sqlx::query(
            "INSERT INTO audit_record (id, session_id, kind, payload_json, \
             parent_hash, self_hash, created_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id.as_bytes().as_slice())
        .bind(session_id.map(|s| s.0.as_bytes().to_vec()))
        .bind(kind.label())
        .bind(canonical)
        .bind(parent_hash.as_slice())
        .bind(self_hash.as_slice())
        .bind(Utc::now().timestamp_micros())
        .execute(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("audit insert: {e}")))?;
        Ok(())
    }

    async fn last_hash_for(&self, session_id: Option<SessionId>) -> ArgoResult<[u8; 32]> {
        let row: Option<Vec<u8>> = match session_id {
            Some(s) => sqlx::query_scalar(
                "SELECT self_hash FROM audit_record WHERE session_id = ? \
                 ORDER BY seq DESC LIMIT 1",
            )
            .bind(s.0.as_bytes().as_slice())
            .fetch_optional(self.store.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("audit last_hash: {e}")))?,
            None => sqlx::query_scalar(
                "SELECT self_hash FROM audit_record WHERE session_id IS NULL \
                 ORDER BY seq DESC LIMIT 1",
            )
            .fetch_optional(self.store.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("audit last_hash: {e}")))?,
        };
        let mut buf = [0u8; 32];
        if let Some(bytes) = row {
            if bytes.len() == 32 {
                buf.copy_from_slice(&bytes);
            }
        }
        Ok(buf)
    }

    /// Test helper: count rows for a session.
    pub async fn count_for(&self, session_id: SessionId) -> ArgoResult<i64> {
        sqlx::query_scalar("SELECT COUNT(*) FROM audit_record WHERE session_id = ?")
            .bind(session_id.0.as_bytes().as_slice())
            .fetch_one(self.store.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("audit count: {e}")))
    }
}

fn chain_hash(parent: &[u8; 32], kind: &str, payload: &str) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(parent);
    h.update(kind.as_bytes());
    h.update(b"\0");
    h.update(payload.as_bytes());
    let out = h.finalize();
    let mut buf = [0u8; 32];
    buf.copy_from_slice(&out);
    buf
}
