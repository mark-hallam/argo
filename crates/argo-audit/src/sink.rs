//! Hash-chained `AuditSink` writer.
//!
//! `self_hash = sha256(parent_hash || kind || 0x00 || canonical_payload)`
//! per session. The first row in a session has `parent_hash = 0…0`.
use crate::event::AuditKind;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, SessionId};
use chrono::Utc;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use uuid::Uuid;

/// Hash function used by the chain. SHA-256 because we already use it
/// elsewhere in the codebase (skill bodies, evidence hashes, …).
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

/// Append-only sink that writes to the §7 `audit_record` table.
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

    /// Test/diagnostic helper: count rows for a session.
    pub async fn count_for(&self, session_id: SessionId) -> ArgoResult<i64> {
        sqlx::query_scalar("SELECT COUNT(*) FROM audit_record WHERE session_id = ?")
            .bind(session_id.0.as_bytes().as_slice())
            .fetch_one(self.store.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("audit count: {e}")))
    }

    /// Borrowed store (used by [`crate::verify::verify_session`]).
    #[must_use]
    pub fn store(&self) -> &SessionStore {
        &self.store
    }
}

/// Re-export so other crates can recompute the chain hash for tests.
#[doc(hidden)]
#[must_use]
pub fn compute_chain_hash(parent: &[u8; 32], kind: &str, payload: &str) -> [u8; 32] {
    chain_hash(parent, kind, payload)
}
