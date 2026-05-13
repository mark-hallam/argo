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

/// Default tenant id used by [`AuditSink::new`] when no explicit id
/// is supplied (Stream `mission-enterprise-cross-tenant-audit`). New
/// rows lose the `__legacy__` sentinel and self-name under one
/// stable workspace bucket; the audit feed's default tenant unions
/// this constant into its read scope so single-tenant deployments
/// behave identically to pre-tenant audits. Callers that know their
/// workspace tenant should use [`AuditSink::new_with_tenant`] (or
/// [`AuditSink::write_with_tenant`] for ad-hoc writes); `argo-config`
/// exposes `workspace_tenant_id` for the canonical resolution path.
pub const DEFAULT_AUDIT_TENANT_ID: &str = "ws-default";

/// Append-only sink that writes to the §7 `audit_record` table.
///
/// As of Stream `claude-realtime-audit-stream`, the sink also
/// fans newly-written rows out via a `tokio::sync::broadcast`
/// channel so the gateway's `/audit/stream` endpoint can push
/// them to live subscribers without polling. The on-disk write
/// path is unchanged; the broadcast is best-effort (a slow
/// subscriber gets dropped, never the writer).
///
/// Stream `mission-enterprise-cross-tenant-audit` adds a
/// `tenant_id` field stamped onto every new row's
/// `audit_record.tenant_id` column and `payload_json.tenant_id`.
/// [`AuditSink::new`] uses [`DEFAULT_AUDIT_TENANT_ID`];
/// [`AuditSink::new_with_tenant`] takes an explicit id derived
/// from the workspace (`argo_config::workspace_tenant_id`).
pub struct AuditSink {
    store: Arc<SessionStore>,
    broadcast: tokio::sync::broadcast::Sender<AuditEvent>,
    batched: Arc<BatchedWriter<AuditItem>>,
    tenant_id: String,
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
    tenant_id: String,
    created_at: DateTime<Utc>,
}

impl std::fmt::Debug for AuditSink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuditSink").finish()
    }
}

impl AuditSink {
    /// Build over an open session store using
    /// [`DEFAULT_AUDIT_TENANT_ID`] as the stamped tenant id. New code
    /// that knows its workspace tenant should prefer
    /// [`AuditSink::new_with_tenant`].
    #[must_use]
    pub fn new(store: Arc<SessionStore>) -> Self {
        Self::new_with_tenant(store, DEFAULT_AUDIT_TENANT_ID.to_string())
    }

    /// Build over an open session store, stamping every new row with
    /// the supplied `tenant_id` (Stream
    /// `mission-enterprise-cross-tenant-audit`). The id is written to
    /// both `audit_record.tenant_id` and `payload_json.tenant_id`;
    /// chain hash inputs are unchanged so historical `self_hash`
    /// values keep verifying.
    #[must_use]
    pub fn new_with_tenant(store: Arc<SessionStore>, tenant_id: String) -> Self {
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
            tenant_id,
        }
    }

    /// Tenant id baked into this sink at construction time.
    #[must_use]
    pub fn tenant_id(&self) -> &str {
        &self.tenant_id
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

    /// Write one event using the sink's bound tenant id.
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
        self.write_with_tenant(session_id, kind, payload, principal_id, &self.tenant_id)
            .await
    }

    /// Write one event under an explicit `tenant_id`, overriding the
    /// sink's default. Stream `mission-enterprise-cross-tenant-audit`
    /// — used by the gateway when stamping rows on behalf of a
    /// resolved-but-non-default tenant (e.g., admin sessions writing
    /// across tenants). The chain hash inputs are unchanged; only the
    /// denorm `tenant_id` column and `payload_json.tenant_id` field
    /// reflect the override.
    pub async fn write_with_tenant(
        &self,
        session_id: Option<SessionId>,
        kind: AuditKind,
        payload: serde_json::Value,
        principal_id: Option<String>,
        tenant_id: &str,
    ) -> ArgoResult<()> {
        let id = Uuid::now_v7();
        // Stamp tenant_id into the payload at the publish edge so the
        // denorm column and the payload-json always agree for new
        // rows. The hash is computed over the stamped payload — same
        // structure / same algorithm as before; historical rows
        // (without a tenant_id key) were hashed without one and still
        // verify because we don't rewrite their payload_json.
        let stamped_payload = stamp_tenant_id(payload, tenant_id);
        let raw = serde_json::to_string(&stamped_payload)
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
            payload: stamped_payload,
            canonical,
            principal_id,
            tenant_id: tenant_id.to_string(),
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

/// Stamp `tenant_id` into a payload at the publish edge (Stream
/// `mission-enterprise-cross-tenant-audit`). Objects gain a
/// top-level `"tenant_id"` key; non-object payloads are rewritten to
/// `{ "_": <original>, "tenant_id": <id> }` so the column-vs-payload
/// invariant holds for every row. An existing `tenant_id` key in an
/// object payload is overwritten — the sink is authoritative.
fn stamp_tenant_id(payload: serde_json::Value, tenant_id: &str) -> serde_json::Value {
    if let serde_json::Value::Object(mut map) = payload {
        map.insert(
            "tenant_id".to_string(),
            serde_json::Value::String(tenant_id.to_string()),
        );
        serde_json::Value::Object(map)
    } else {
        let mut map = serde_json::Map::new();
        map.insert("_".to_string(), payload);
        map.insert(
            "tenant_id".to_string(),
            serde_json::Value::String(tenant_id.to_string()),
        );
        serde_json::Value::Object(map)
    }
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
         parent_hash, self_hash, created_at, principal_id, tenant_id) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
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
             parent_hash, self_hash, created_at, principal_id, tenant_id) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
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
