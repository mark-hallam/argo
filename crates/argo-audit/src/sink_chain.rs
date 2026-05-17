//! Chain-hash arithmetic, batched insert/flush, and post-commit
//! fan-out helpers for [`crate::sink::AuditSink`].
//!
//! Extracted from `sink.rs` under the
//! `codex-split-files-450-500-wave` brief to keep the parent file
//! below the 420-LOC headroom target. The exported helpers are
//! `pub(crate)` and are re-imported from `sink.rs`; no public
//! surface change.
use crate::sink::{AuditEvent, AuditItem};
use argo_types::{ArgoError, ArgoResult, SessionId};
use argo_store::SessionStore;
use sha2::{Digest, Sha256};
use sqlx::{Pool, Sqlite};
use std::collections::HashMap;
use std::sync::Arc;

/// Hash function used by the chain. SHA-256 because we already use it
/// elsewhere in the codebase (skill bodies, evidence hashes, …).
pub(crate) fn chain_hash(parent: &[u8; 32], kind: &str, payload: &str) -> [u8; 32] {
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

/// Pull every UUID-shaped worker id out of a freshly-written audit
/// payload (Stream `claude-worker-presence-tracking`). Recognised
/// keys: top-level `worker_id`, `from_worker`, `from_worker_id`,
/// `to_worker_id`, plus the same names nested under `payload`/`to`.
/// Empty strings, JSON-null, and parse failures are skipped — the
/// caller treats this as best-effort metadata extraction.
fn worker_ids_in_payload(payload: &serde_json::Value) -> Vec<String> {
    const POINTERS: [&str; 8] = [
        "/worker_id",
        "/from_worker",
        "/from_worker_id",
        "/to_worker_id",
        "/payload/worker_id",
        "/payload/from_worker",
        "/payload/from_worker_id",
        "/to/worker_id",
    ];
    let mut out = Vec::new();
    for pointer in POINTERS {
        if let Some(raw) = payload.pointer(pointer).and_then(|v| v.as_str()) {
            if !raw.is_empty() && !out.iter().any(|seen: &String| seen == raw) {
                out.push(raw.to_string());
            }
        }
    }
    out
}

pub(crate) async fn last_hash_for(
    pool: &Pool<Sqlite>,
    session_id: Option<SessionId>,
) -> ArgoResult<[u8; 32]> {
    let row: Option<Vec<u8>> = match session_id {
        Some(s) => sqlx::query_scalar(
            "SELECT self_hash FROM audit_record WHERE session_id = ? \
             ORDER BY seq DESC LIMIT 1",
        )
        .bind(s.0.as_bytes().as_slice())
        .fetch_optional(pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("audit last_hash: {e}")))?,
        None => sqlx::query_scalar(
            "SELECT self_hash FROM audit_record WHERE session_id IS NULL \
             ORDER BY seq DESC LIMIT 1",
        )
        .fetch_optional(pool)
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

pub(crate) async fn insert_one_with_chain(
    pool: &Pool<Sqlite>,
    item: &AuditItem,
    parent_hash: &[u8; 32],
) -> ArgoResult<()> {
    let self_hash = chain_hash(parent_hash, item.kind, &item.canonical);
    sqlx::query(
        "INSERT INTO audit_record (id, session_id, kind, payload_json, \
         parent_hash, self_hash, created_at, principal_id, tenant_id, \
         worker_id, channel, action_type, outcome_status, \
         approver_id, cost_band, data_classification) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(item.id.as_bytes().as_slice())
    .bind(item.session_id.map(|s| s.0.as_bytes().to_vec()))
    .bind(item.kind)
    .bind(&item.canonical)
    .bind(parent_hash.as_slice())
    .bind(self_hash.as_slice())
    .bind(item.created_at.timestamp_micros())
    .bind(item.principal_id.as_deref())
    .bind(&item.tenant_id)
    .bind(item.filter_dims.worker_id.as_deref())
    .bind(item.filter_dims.channel.as_deref())
    .bind(item.filter_dims.action_type.as_deref())
    .bind(item.filter_dims.outcome_status.as_deref())
    .bind(item.filter_dims.approver_id.as_deref())
    .bind(item.filter_dims.cost_band.as_deref())
    .bind(item.filter_dims.data_classification.as_deref())
    .execute(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("audit insert: {e}")))?;
    Ok(())
}

/// Flush-fn for the per-sink `BatchedWriter`. Computes chain
/// hashes in submit order per session_id, inserts every row in
/// one transaction, then runs the post-commit fanout (worker
/// presence + broadcast).
pub(crate) async fn flush_audit(
    pool: Pool<Sqlite>,
    store: Arc<SessionStore>,
    broadcast: tokio::sync::broadcast::Sender<AuditEvent>,
    batch: Vec<AuditItem>,
) -> ArgoResult<()> {
    if batch.is_empty() {
        return Ok(());
    }

    // Group indices by session bucket. `None` is the cross-session
    // "no session" chain. We track index-into-`batch` per session
    // so each row's hash inputs are computed in submit order.
    let mut buckets: HashMap<Option<[u8; 16]>, Vec<usize>> = HashMap::new();
    for (idx, item) in batch.iter().enumerate() {
        let key = item.session_id.map(|s| *s.0.as_bytes());
        buckets.entry(key).or_default().push(idx);
    }

    let mut parent_hashes = vec![[0u8; 32]; batch.len()];
    let mut self_hashes = vec![[0u8; 32]; batch.len()];
    for indices in buckets.values() {
        let first = batch[indices[0]].session_id;
        let mut parent = last_hash_for(&pool, first).await?;
        for &idx in indices {
            let item = &batch[idx];
            let s = chain_hash(&parent, item.kind, &item.canonical);
            parent_hashes[idx] = parent;
            self_hashes[idx] = s;
            parent = s;
        }
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| ArgoError::Provider(format!("audit tx begin: {e}")))?;
    for (idx, item) in batch.iter().enumerate() {
        sqlx::query(
            "INSERT INTO audit_record (id, session_id, kind, payload_json, \
             parent_hash, self_hash, created_at, principal_id, tenant_id, \
             worker_id, channel, action_type, outcome_status, \
             approver_id, cost_band, data_classification) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(item.id.as_bytes().as_slice())
        .bind(item.session_id.map(|s| s.0.as_bytes().to_vec()))
        .bind(item.kind)
        .bind(&item.canonical)
        .bind(parent_hashes[idx].as_slice())
        .bind(self_hashes[idx].as_slice())
        .bind(item.created_at.timestamp_micros())
        .bind(item.principal_id.as_deref())
        .bind(&item.tenant_id)
        .bind(item.filter_dims.worker_id.as_deref())
        .bind(item.filter_dims.channel.as_deref())
        .bind(item.filter_dims.action_type.as_deref())
        .bind(item.filter_dims.outcome_status.as_deref())
        .bind(item.filter_dims.approver_id.as_deref())
        .bind(item.filter_dims.cost_band.as_deref())
        .bind(item.filter_dims.data_classification.as_deref())
        .execute(&mut *tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("audit insert: {e}")))?;
    }
    tx.commit()
        .await
        .map_err(|e| ArgoError::Provider(format!("audit tx commit: {e}")))?;

    after_commit_fanout(&store, &broadcast, &batch).await?;
    Ok(())
}

pub(crate) async fn after_commit_fanout(
    store: &Arc<SessionStore>,
    broadcast: &tokio::sync::broadcast::Sender<AuditEvent>,
    items: &[AuditItem],
) -> ArgoResult<()> {
    for item in items {
        for worker_id in worker_ids_in_payload(&item.payload) {
            argo_store::worker_presence::touch_presence(store.pool(), &worker_id).await?;
        }
        // Best-effort broadcast — `send` errors only when there
        // are no live receivers, which is fine; subscribers will
        // get rows written after they connect.
        let _ = broadcast.send(AuditEvent {
            id: item.id.to_string(),
            session_id: item.session_id.map(|s| s.0.to_string()),
            kind: item.kind.to_string(),
            payload: item.payload.clone(),
            created_at: item.created_at.to_rfc3339(),
        });
    }
    // Best-effort `audit_chain_tail_seq` gauge for `/openmetrics`. A
    // failure here logs at debug and moves on rather than blocking the
    // audit-write path.
    if let Ok(Some(seq)) = sqlx::query_scalar::<_, Option<i64>>("SELECT MAX(seq) FROM audit_record")
        .fetch_one(store.pool())
        .await
    {
        argo_telemetry::prometheus_registry::register_scalar_gauge(
            "audit_chain_tail_seq",
            "Latest sequence number observed by the audit sink after a successful append",
        )
        .set(seq);
    }
    Ok(())
}
