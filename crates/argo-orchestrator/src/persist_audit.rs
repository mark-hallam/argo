//! Worker→worker handoff audit row writer
//! (Stream `claude-worker-handoff-record`).
//!
//! When `OrchestratorStore::update_handoff_status` walks a row from
//! `Pending` → `Acknowledged`, it appends a single audit row of kind
//! `worker_handoff` to the session's chain. Mission claim 3
//! ("workers cooperate; cooperation is observable") needs every
//! delegation to land in the audit timeline alongside everything
//! else — `cancel_handoff` already does this for cancellations
//! (`handoff_cancelled`), and this module is its acknowledgement
//! sibling.
//!
//! Idempotency is enforced by the caller: only a real
//! `Pending → Acknowledged` transition triggers a write. Re-acking
//! an already-acknowledged handoff is permitted by the state
//! machine but produces no extra row.
use crate::{EscalationStep, Handoff};
use argo_types::{ArgoError, ArgoResult};
use chrono::Utc;
use uuid::Uuid;

/// Stable kind label persisted in the `audit_record.kind` column for
/// worker→worker handoff acknowledgements.
pub const WORKER_HANDOFF_KIND: &str = "worker_handoff";

/// Append a single `worker_handoff` audit row tying the from-worker,
/// to-step, session, expected outcome, and a deterministic payload
/// hash into the session's chain.
///
/// Mirrors the shape of `write_handoff_cancelled_audit` in
/// `persist.rs`: bypasses the typed `AuditSink` because handoff
/// kinds are not in the [`argo_audit::AuditKind`] enum today
/// (`handoff_emitted` / `handoff_settled` / `handoff_cancelled` are
/// all string-literal kinds).
pub async fn audit_worker_handoff(
    pool: &sqlx::Pool<sqlx::Sqlite>,
    handoff: &Handoff,
) -> ArgoResult<()> {
    let canonical = canonical_payload(handoff)?;
    let sid = Uuid::parse_str(handoff.session_id.as_str())
        .map_err(|e| ArgoError::Schema(format!("handoff.session_id: {e}")))?;
    let parent = last_session_hash(pool, &sid).await?;
    let self_hash = argo_audit::compute_chain_hash(&parent, WORKER_HANDOFF_KIND, &canonical);
    let id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO audit_record (id, session_id, kind, payload_json, \
         parent_hash, self_hash, created_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id.as_bytes().as_slice())
    .bind(sid.as_bytes().as_slice())
    .bind(WORKER_HANDOFF_KIND)
    .bind(canonical)
    .bind(parent.as_slice())
    .bind(self_hash.as_slice())
    .bind(Utc::now().timestamp_micros())
    .execute(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("audit insert: {e}")))?;
    Ok(())
}

/// Build the canonical JSON payload for a `worker_handoff` audit row.
/// Public for tests; unit-tested separately for stability.
pub(crate) fn canonical_payload(handoff: &Handoff) -> ArgoResult<String> {
    let payload_hash = payload_hash(&handoff.payload);
    let (to_kind, to_worker_id, to_addr) = match &handoff.to {
        EscalationStep::Worker(id) => ("worker", Some(id.to_string()), None),
        EscalationStep::Human(addr) => ("human", None, Some(addr.clone())),
    };
    let value = serde_json::json!({
        "from_worker": handoff.from.to_string(),
        "to_kind": to_kind,
        "to_worker_id": to_worker_id,
        "to_addr": to_addr,
        "session_id": handoff.session_id.as_str(),
        "handoff_id": handoff.id.as_str(),
        "reason": handoff.expected_outcome,
        "payload_hash": payload_hash,
    });
    serde_json::to_string(&value)
        .map_err(|e| ArgoError::Schema(format!("worker_handoff payload: {e}")))
}

/// Deterministic hex-encoded sha256 of the canonical JSON
/// serialization of the handoff's opaque payload. Re-uses the audit
/// pipeline's existing chain-hash primitive
/// (`argo_audit::compute_chain_hash`) with a zero parent and a fixed
/// `"handoff.payload"` salt so two rows referencing the same payload
/// produce the same hash without leaking payload contents into the
/// audit timeline.
pub(crate) fn payload_hash(payload: &serde_json::Value) -> String {
    let canonical = serde_json::to_string(payload).unwrap_or_default();
    let bytes = argo_audit::compute_chain_hash(&[0u8; 32], "handoff.payload", &canonical);
    let mut out = String::with_capacity(64);
    use std::fmt::Write;
    for b in bytes {
        let _ = write!(out, "{:02x}", b);
    }
    out
}

/// Look up the most recent audit row's `self_hash` for the session.
/// Returns the zero hash when no rows exist yet (start of chain).
/// Duplicates `last_session_hash` in `persist.rs` to keep this
/// module self-contained per the brief's may-touch boundary.
async fn last_session_hash(
    pool: &sqlx::Pool<sqlx::Sqlite>,
    sid: &Uuid,
) -> ArgoResult<[u8; 32]> {
    let row: Option<Vec<u8>> = sqlx::query_scalar(
        "SELECT self_hash FROM audit_record WHERE session_id = ? \
         ORDER BY seq DESC LIMIT 1",
    )
    .bind(sid.as_bytes().as_slice())
    .fetch_optional(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("audit last_hash: {e}")))?;
    let mut buf = [0u8; 32];
    if let Some(bytes) = row {
        if bytes.len() == 32 {
            buf.copy_from_slice(&bytes);
        }
    }
    Ok(buf)
}

#[cfg(test)]
#[path = "persist_audit_tests.rs"]
mod tests;
