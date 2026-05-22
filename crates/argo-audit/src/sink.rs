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
use crate::filter_dims::{extract_filter_dims, FilterDims};
use crate::sink_chain::{after_commit_fanout, chain_hash, flush_audit, insert_one_with_chain, last_hash_for};
use crate::sink_tenant::{stamp_tenant_id, DEFAULT_AUDIT_TENANT_ID};
use argo_security::pii_detect;
use argo_store::batched_writer::{self, BatchedWriter};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, SessionId};
use chrono::{DateTime, Utc};
use std::sync::Arc;
use uuid::Uuid;

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
pub(crate) struct AuditItem {
    pub(crate) id: Uuid,
    pub(crate) session_id: Option<SessionId>,
    pub(crate) kind: &'static str,
    pub(crate) payload: serde_json::Value,
    pub(crate) canonical: String,
    pub(crate) principal_id: Option<String>,
    pub(crate) tenant_id: String,
    pub(crate) created_at: DateTime<Utc>,
    /// Filter dimensions denormalized from `canonical` for the
    /// J14 audit-feed SQL filter path
    /// (`mission-enterprise-audit-full-text-search`).
    pub(crate) filter_dims: FilterDims,
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
    #[deprecated(
        note = "production must use new_with_tenant; new() retained for test fixtures only"
    )]
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
        // Source the denorm dims from the canonical JSON we're about
        // to store — keeps `verify_chain` denorm sanity check
        // straightforward (column == extract(payload_json_on_disk)).
        let canonical_value: serde_json::Value =
            serde_json::from_str(&canonical).unwrap_or(serde_json::Value::Null);
        let filter_dims = extract_filter_dims(&canonical_value);
        let item = AuditItem {
            id,
            session_id,
            kind: kind.label(),
            payload: stamped_payload,
            canonical,
            principal_id,
            tenant_id: tenant_id.to_string(),
            created_at: now,
            filter_dims,
        };
        if batched_writer::is_batching_enabled() {
            self.batched.submit(item).await
        } else {
            // Fallback: single-row path with the original
            // read-then-insert chain-hash semantics.
            let parent_hash = last_hash_for(self.store.pool(), session_id).await?;
            insert_one_with_chain(self.store.pool(), &item, &parent_hash).await?;
            after_commit_fanout(&self.store, &self.broadcast, std::slice::from_ref(&item)).await?;
            Ok(())
        }
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

    /// Cloned store handle. Used by [`crate::sink_tx`] for post-commit
    /// fanout that needs to touch `worker_presence`. Crate-private so
    /// external callers stick to [`Self::store`].
    pub(crate) fn store_arc(&self) -> Arc<SessionStore> {
        Arc::clone(&self.store)
    }

    /// Borrowed broadcast sender. Used by [`crate::sink_tx`] for the
    /// post-commit broadcast fan-out path. Crate-private — external
    /// callers subscribe via [`Self::subscribe`].
    pub(crate) fn broadcast_sender(&self) -> &tokio::sync::broadcast::Sender<AuditEvent> {
        &self.broadcast
    }
}

/// Re-export so other crates can recompute the chain hash for tests.
#[doc(hidden)]
#[must_use]
pub fn compute_chain_hash(parent: &[u8; 32], kind: &str, payload: &str) -> [u8; 32] {
    chain_hash(parent, kind, payload)
}

