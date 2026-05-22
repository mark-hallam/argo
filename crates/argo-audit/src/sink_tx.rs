//! Transactional audit write — shares the caller's sqlx transaction
//! so the `audit_record` insert commits atomically with the caller's
//! own mutation (workflow row, etc.). Bypasses the per-sink
//! [`argo_store::batched_writer::BatchedWriter`] deliberately —
//! batching owns its own transaction and cannot share with arbitrary
//! callers.
//!
//! Added under `mission-workflow-audit-write-transactional` to close
//! the P1 atomicity gap in the workflow-definition mutating gateway
//! handlers (`set_head`, `archive`, `rollback`). Same chain-hash,
//! PII-detection, tenant-stamping, and filter-dim algorithm as
//! [`AuditSink::write`] — only the executor (`&mut Transaction`
//! instead of `&Pool`) and the commit point (caller-owned instead of
//! sink-owned) differ.
//!
//! ## Usage
//!
//! ```ignore
//! let mut tx = pool.begin().await?;
//! // ... caller's own mutation against `&mut *tx` ...
//! let post = audit_sink
//!     .write_in_tx(&mut tx, session_id, kind, payload, principal_id)
//!     .await?;
//! tx.commit().await?;
//! audit_sink.after_commit_fanout(post).await.ok();
//! ```
//!
//! Drop without calling [`AuditSink::after_commit_fanout`] is allowed
//! — the row still lands on disk (it's in the caller's committed tx);
//! only the best-effort broadcast / worker-presence fan-out is
//! skipped. Production handlers should always invoke it; tests that
//! don't care can drop the guard.

use crate::event::AuditKind;
use crate::filter_dims::extract_filter_dims;
use crate::sink::{AuditItem, AuditSink};
use crate::sink_chain::{after_commit_fanout, chain_hash};
use crate::sink_tenant::stamp_tenant_id;
use argo_security::pii_detect;
use argo_types::{ArgoError, ArgoResult, SessionId};
use chrono::Utc;
use sqlx::{Sqlite, Transaction};
use uuid::Uuid;

/// Opaque guard carrying freshly-inserted audit items so the caller
/// can run the post-commit fan-out (broadcast + worker_presence)
/// AFTER they commit the surrounding transaction.
///
/// The struct is move-only and carries no `Drop` side-effects —
/// callers are responsible for invoking
/// [`AuditSink::after_commit_fanout`] explicitly. See the module
/// docs for the canonical pattern.
pub struct AuditPostCommit {
    pub(crate) items: Vec<AuditItem>,
}

impl AuditPostCommit {
    /// Number of audit rows in the guard.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// `true` if the guard carries no rows.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

impl AuditSink {
    /// Write one audit row INSIDE the caller's transaction. The
    /// insert commits with the caller's `tx.commit()`; if any other
    /// statement in the same tx (or the commit itself) fails, the
    /// audit row is rolled back atomically.
    ///
    /// Returns an [`AuditPostCommit`] guard the caller passes to
    /// [`Self::after_commit_fanout`] after `tx.commit()` to run the
    /// best-effort broadcast + worker-presence fan-out.
    ///
    /// # Errors
    ///
    /// Returns [`ArgoError::Provider`] on sqlx failure or
    /// [`ArgoError::Schema`] if the payload cannot be serialised.
    pub async fn write_in_tx<'c>(
        &self,
        tx: &mut Transaction<'c, Sqlite>,
        session_id: Option<SessionId>,
        kind: AuditKind,
        payload: serde_json::Value,
        principal_id: Option<String>,
    ) -> ArgoResult<AuditPostCommit> {
        let id = Uuid::now_v7();
        let stamped_payload = stamp_tenant_id(payload, self.tenant_id());
        let raw = serde_json::to_string(&stamped_payload)
            .map_err(|e| ArgoError::Schema(format!("audit payload serialize: {e}")))?;
        let canonical = pii_detect::scan_and_maybe_redact(&raw, "audit_write").into_owned();
        let canonical_value: serde_json::Value =
            serde_json::from_str(&canonical).unwrap_or(serde_json::Value::Null);
        let filter_dims = extract_filter_dims(&canonical_value);
        let created_at = Utc::now();

        let parent_hash = parent_hash_in_tx(tx, session_id).await?;
        let self_hash = chain_hash(&parent_hash, kind.label(), &canonical);

        sqlx::query(
            "INSERT INTO audit_record (id, session_id, kind, payload_json, \
             parent_hash, self_hash, created_at, principal_id, tenant_id, \
             worker_id, channel, action_type, outcome_status, \
             approver_id, cost_band, data_classification) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id.as_bytes().as_slice())
        .bind(session_id.map(|s| s.0.as_bytes().to_vec()))
        .bind(kind.label())
        .bind(&canonical)
        .bind(parent_hash.as_slice())
        .bind(self_hash.as_slice())
        .bind(created_at.timestamp_micros())
        .bind(principal_id.as_deref())
        .bind(self.tenant_id())
        .bind(filter_dims.worker_id.as_deref())
        .bind(filter_dims.channel.as_deref())
        .bind(filter_dims.action_type.as_deref())
        .bind(filter_dims.outcome_status.as_deref())
        .bind(filter_dims.approver_id.as_deref())
        .bind(filter_dims.cost_band.as_deref())
        .bind(filter_dims.data_classification.as_deref())
        .execute(&mut **tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("audit insert (in tx): {e}")))?;

        let item = AuditItem {
            id,
            session_id,
            kind: kind.label(),
            payload: stamped_payload,
            canonical,
            principal_id,
            tenant_id: self.tenant_id().to_string(),
            created_at,
            filter_dims,
        };
        Ok(AuditPostCommit { items: vec![item] })
    }

    /// Run the post-commit fan-out (broadcast + worker-presence
    /// touches) for rows written via [`Self::write_in_tx`]. Call
    /// AFTER `tx.commit()` succeeds. Errors here are best-effort —
    /// production callers should `.ok()` the result so a fan-out
    /// hiccup never turns a successful audit-and-mutation tx into a
    /// 5xx response.
    pub async fn after_commit_fanout(&self, post: AuditPostCommit) -> ArgoResult<()> {
        let store = self.store_arc();
        let broadcast = self.broadcast_sender();
        after_commit_fanout(&store, broadcast, &post.items).await
    }
}

async fn parent_hash_in_tx<'c>(
    tx: &mut Transaction<'c, Sqlite>,
    session_id: Option<SessionId>,
) -> ArgoResult<[u8; 32]> {
    let row: Option<Vec<u8>> = match session_id {
        Some(s) => sqlx::query_scalar(
            "SELECT self_hash FROM audit_record WHERE session_id = ? \
             ORDER BY seq DESC LIMIT 1",
        )
        .bind(s.0.as_bytes().as_slice())
        .fetch_optional(&mut **tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("audit last_hash (in tx): {e}")))?,
        None => sqlx::query_scalar(
            "SELECT self_hash FROM audit_record WHERE session_id IS NULL \
             ORDER BY seq DESC LIMIT 1",
        )
        .fetch_optional(&mut **tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("audit last_hash (in tx): {e}")))?,
    };
    let mut buf = [0u8; 32];
    if let Some(bytes) = row {
        if bytes.len() == 32 {
            buf.copy_from_slice(&bytes);
        }
    }
    Ok(buf)
}
