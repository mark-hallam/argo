//! Chain-hash arithmetic, batched insert/flush, and post-commit
//! fan-out helpers for [`crate::sink::AuditSink`].
//!
//! Extracted from `sink.rs` under the
//! `codex-split-files-450-500-wave` brief to keep the parent file
//! below the 420-LOC headroom target. The exported helpers are
//! `pub(crate)` and are re-imported from `sink.rs`; no public
//! surface change.
use crate::sink::{AuditEvent, AuditItem};
use crate::sink_tx::parent_hash_in_tx;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, SessionId};
use sha2::{Digest, Sha256};
use sqlx::{Pool, Sqlite, Transaction};
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

/// Insert one chain-hashed row INSIDE an existing transaction. The caller
/// opens the tx with `BEGIN IMMEDIATE` and reads the parent hash via
/// [`crate::sink_tx::parent_hash_in_tx`] under that SAME lock, so the
/// read-then-insert is atomic and two concurrent appends can never share a
/// parent_hash (chain fork). There is intentionally NO bare-pool variant —
/// the old `insert_one_with_chain`/`last_hash_for` pool helpers were the last
/// non-serialized append path and have been removed (security re-review of
/// #2805).
pub(crate) async fn insert_one_with_chain_in_tx<'c>(
    tx: &mut Transaction<'c, Sqlite>,
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
    .execute(&mut **tx)
    .await
    .map_err(|e| ArgoError::Provider(format!("audit insert: {e}")))?;
    Ok(())
}

/// Flush-fn for the per-sink `BatchedWriter`. Computes chain
/// hashes in submit order per session_id, inserts every row in
/// one transaction, then runs the post-commit fanout (worker
/// presence + broadcast).
///
/// Serialization: the writer opens a `BEGIN IMMEDIATE` transaction so
/// the SQLite write lock is acquired BEFORE the per-bucket parent-hash
/// read. The read happens inside that transaction (via
/// [`crate::sink_tx::parent_hash_in_tx`]), so a concurrent append
/// (canonical or a formerly side-door path that now takes its own
/// `BEGIN IMMEDIATE`) cannot slip a row in between this writer's read
/// and its insert. `busy_timeout` (5s, set in `wal_tuning`) makes the
/// second writer wait for the lock rather than fork the chain.
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

    // Acquire the write lock up front (`BEGIN IMMEDIATE`), THEN read
    // each bucket's chain tail inside the transaction so the
    // read-modify-write is atomic against concurrent appends.
    let mut tx = pool
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(|e| ArgoError::Provider(format!("audit tx begin: {e}")))?;

    let mut parent_hashes = vec![[0u8; 32]; batch.len()];
    let mut self_hashes = vec![[0u8; 32]; batch.len()];
    for indices in buckets.values() {
        let first = batch[indices[0]].session_id;
        let mut parent = parent_hash_in_tx(&mut tx, first).await?;
        for &idx in indices {
            let item = &batch[idx];
            let s = chain_hash(&parent, item.kind, &item.canonical);
            parent_hashes[idx] = parent;
            self_hashes[idx] = s;
            parent = s;
        }
    }

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
            argo_store::worker_presence::touch_presence_for_tenant(
                store.pool(),
                &item.tenant_id,
                &worker_id,
            )
            .await?;
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
