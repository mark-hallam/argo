//! Hash-chained `AuditSink` writer.
//!
//! `self_hash = sha256(parent_hash || kind || 0x00 || canonical_payload)`
//! per session. The first row in a session has `parent_hash = 0…0`.
//!
//! As of Stream `claude-perf-store-batched-writes`, concurrent
//! writes are coalesced through a per-sink
//! `argo_store::batched_writer::BatchedWriter`. The batch flush
//! computes the chain hashes in submit order per session, then
//! issues every INSERT inside one transaction. Subscribers and
//! worker-presence touches still fire post-commit, in submit order.
//!
//! Set `ARGO_STORE_BATCHED_WRITES=0` to fall back to the historical
//! one-row-per-transaction path (single-row chain semantics
//! unchanged).
use crate::event::AuditKind;
use argo_security::pii_detect;
use argo_store::batched_writer::{self, BatchedWriter};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, SessionId};
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use sqlx::{Pool, Sqlite};
use std::collections::HashMap;
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
///
/// As of Stream `claude-realtime-audit-stream`, the sink also
/// fans newly-written rows out via a `tokio::sync::broadcast`
/// channel so the gateway's `/audit/stream` endpoint can push
/// them to live subscribers without polling. The on-disk write
/// path is unchanged; the broadcast is best-effort (a slow
/// subscriber gets dropped, never the writer).
pub struct AuditSink {
    store: Arc<SessionStore>,
    broadcast: tokio::sync::broadcast::Sender<AuditEvent>,
    batched: Arc<BatchedWriter<AuditItem>>,
}

/// One row's worth of streaming-friendly metadata. Skips the
/// chain hash + payload-bytes-canonical to keep the wire shape
/// small; subscribers that need the full payload can re-fetch
/// from the audit table by `id`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuditEvent {
    /// UUID of the audit row.
    pub id: String,
    /// Optional session id (UUID string) the row belongs to.
    pub session_id: Option<String>,
    /// Wire label of the audit kind (e.g. `tool_call`).
    pub kind: String,
    /// JSON payload as written to the table.
    pub payload: serde_json::Value,
    /// RFC-3339 timestamp.
    pub created_at: String,
}

/// Default capacity for the broadcast channel. A slow subscriber
/// that falls more than this many rows behind is dropped per
/// the brief: "never block the audit-write path".
pub const AUDIT_BROADCAST_CAPACITY: usize = 256;

/// One pending audit row buffered by the per-sink `BatchedWriter`.
struct AuditItem {
    id: Uuid,
    session_id: Option<SessionId>,
    kind: &'static str,
    payload: serde_json::Value,
    canonical: String,
    principal_id: Option<String>,
    created_at: DateTime<Utc>,
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
        let (tx, _rx) = tokio::sync::broadcast::channel(AUDIT_BROADCAST_CAPACITY);
        let pool = store.pool().clone();
        let store_for_flush = Arc::clone(&store);
        let broadcast_for_flush = tx.clone();
        let batched = Arc::new(BatchedWriter::new(
            move |batch| {
                let pool = pool.clone();
                let store = Arc::clone(&store_for_flush);
                let broadcast = broadcast_for_flush.clone();
                async move { flush_audit(pool, store, broadcast, batch).await }
            },
            batched_writer::batch_size_from_env(),
            batched_writer::batch_latency_from_env(),
        ));
        Self {
            store,
            broadcast: tx,
            batched,
        }
    }

    /// Subscribe to live audit events. Returns a receiver that
    /// yields every newly-written row from this point forward.
    /// Closing the sender (via `Drop`) terminates the receiver.
    /// Lagged subscribers see a `RecvError::Lagged(n)` they can
    /// surface to the operator (or just close the connection).
    #[must_use]
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<AuditEvent> {
        self.broadcast.subscribe()
    }

    /// Test/diagnostic helper: number of live broadcast subscribers.
    #[must_use]
    pub fn subscriber_count(&self) -> usize {
        self.broadcast.receiver_count()
    }

    /// Flush any in-flight batched audit rows. Used by tests that
    /// read the `audit_record` table immediately after writing
    /// without waiting for the latency window.
    pub async fn flush_pending(&self) -> ArgoResult<()> {
        self.batched.flush_now().await
    }

    /// Write one event.
    ///
    /// `principal_id` carries the resolved gateway caller (per
    /// `mvp-audit-principal-stamp`, the X-Argo-Principal header
    /// parsed at the bearer-auth gate). Pass `None` for
    /// system-internal writes (background jobs, CLI, tests) — the
    /// row's `principal_id` column lands NULL.
    pub async fn write(
        &self,
        session_id: Option<SessionId>,
        kind: AuditKind,
        payload: serde_json::Value,
        principal_id: Option<String>,
    ) -> ArgoResult<()> {
        let id = Uuid::now_v7();
        let raw = serde_json::to_string(&payload)
            .map_err(|e| ArgoError::Schema(format!("audit payload serialize: {e}")))?;
        // Soft-flag (or auto-redact) PII before hashing + persisting so
        // the chain stays consistent with what's on disk. Default is
        // log-only; redaction requires `ARGO_PII_AUTOREDACT=1`.
        let canonical = pii_detect::scan_and_maybe_redact(&raw, "audit_write").into_owned();
        let now = Utc::now();
        let item = AuditItem {
            id,
            session_id,
            kind: kind.label(),
            payload,
            canonical,
            principal_id,
            created_at: now,
        };
        if batched_writer::is_batching_enabled() {
            self.batched.submit(item).await
        } else {
            // Fallback: single-row path with the original
            // read-then-insert chain-hash semantics.
            let parent_hash = self.last_hash_for(session_id).await?;
            insert_one_with_chain(self.store.pool(), &item, &parent_hash).await?;
            after_commit_fanout(&self.store, &self.broadcast, std::slice::from_ref(&item)).await?;
            Ok(())
        }
    }

    async fn last_hash_for(&self, session_id: Option<SessionId>) -> ArgoResult<[u8; 32]> {
        last_hash_for(self.store.pool(), session_id).await
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

async fn last_hash_for(pool: &Pool<Sqlite>, session_id: Option<SessionId>) -> ArgoResult<[u8; 32]> {
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

async fn insert_one_with_chain(
    pool: &Pool<Sqlite>,
    item: &AuditItem,
    parent_hash: &[u8; 32],
) -> ArgoResult<()> {
    let self_hash = chain_hash(parent_hash, item.kind, &item.canonical);
    sqlx::query(
        "INSERT INTO audit_record (id, session_id, kind, payload_json, \
         parent_hash, self_hash, created_at, principal_id) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(item.id.as_bytes().as_slice())
    .bind(item.session_id.map(|s| s.0.as_bytes().to_vec()))
    .bind(item.kind)
    .bind(&item.canonical)
    .bind(parent_hash.as_slice())
    .bind(self_hash.as_slice())
    .bind(item.created_at.timestamp_micros())
    .bind(item.principal_id.as_deref())
    .execute(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("audit insert: {e}")))?;
    Ok(())
}

/// Flush-fn for the per-sink `BatchedWriter`. Computes chain
/// hashes in submit order per session_id, inserts every row in
/// one transaction, then runs the post-commit fanout (worker
/// presence + broadcast).
async fn flush_audit(
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
             parent_hash, self_hash, created_at, principal_id) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(item.id.as_bytes().as_slice())
        .bind(item.session_id.map(|s| s.0.as_bytes().to_vec()))
        .bind(item.kind)
        .bind(&item.canonical)
        .bind(parent_hashes[idx].as_slice())
        .bind(self_hashes[idx].as_slice())
        .bind(item.created_at.timestamp_micros())
        .bind(item.principal_id.as_deref())
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

async fn after_commit_fanout(
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
    Ok(())
}
