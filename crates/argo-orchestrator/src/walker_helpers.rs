//! Persistence + parsing helpers extracted from `walker.rs` to keep
//! that file under the 500-LOC ceiling (HARD RULE 6).
//!
//! Sibling module to `walker.rs`. The walker delegates three sub-
//! concerns here:
//!
//! - cross-tenant overdue-row decoding ([`find_overdue_handoffs`],
//!   `RawOverdue` → [`OverdueRow`]),
//! - payload-shape parsing ([`parse_chain_from_payload`],
//!   [`parse_index_from_payload`], [`stringify_step`]),
//! - chain-exhausted audit row writes
//!   ([`write_escalation_exhausted_audit`]).
//!
//! Pure refactor: no public-API or behaviour changes. All items are
//! `pub(crate)` (or private) so the surface stays inside the
//! orchestrator crate.
use crate::escalation::{EscalationChain, EscalationStep};
use crate::handoff::{Handoff, HandoffId, HandoffStatus};
use crate::session::SessionId;
use argo_audit::compute_chain_hash;
use argo_identity::WorkerId;
use argo_types::{ArgoError, ArgoResult, TenantId};
use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::SqlitePool;
use std::str::FromStr;

#[derive(sqlx::FromRow)]
struct RawOverdue {
    handoff_id: String,
    session_id: Vec<u8>,
    tenant_id: Vec<u8>,
    from_worker_id: String,
    to_field: String,
    expected_outcome: String,
    payload: String,
    created_at: i64,
    deadline: Option<i64>,
}

pub(crate) struct OverdueRow {
    pub(crate) handoff: Handoff,
    pub(crate) tenant_id: TenantId,
}

impl TryFrom<RawOverdue> for OverdueRow {
    type Error = ArgoError;
    fn try_from(r: RawOverdue) -> ArgoResult<Self> {
        let session_id = SessionId::from_string(
            uuid::Uuid::from_slice(&r.session_id)
                .map_err(|e| ArgoError::Schema(format!("session uuid: {e}")))?
                .to_string(),
        );
        let tenant_id = TenantId(
            uuid::Uuid::from_slice(&r.tenant_id)
                .map_err(|e| ArgoError::Schema(format!("tenant uuid: {e}")))?,
        );
        let from = WorkerId::from_str(&r.from_worker_id)
            .map_err(|e| ArgoError::Schema(format!("worker uuid: {e}")))?;
        let to = parse_step_from_persist_json(&r.to_field)?;
        let payload: Value = serde_json::from_str(&r.payload)
            .map_err(|e| ArgoError::Schema(format!("payload: {e}")))?;
        Ok(Self {
            handoff: Handoff {
                id: HandoffId::from_string(r.handoff_id),
                session_id,
                from,
                to,
                expected_outcome: r.expected_outcome,
                payload,
                status: HandoffStatus::Pending,
                created_at: chrono::DateTime::<Utc>::from_timestamp_micros(r.created_at)
                    .unwrap_or_else(Utc::now),
                deadline: r
                    .deadline
                    .and_then(chrono::DateTime::<Utc>::from_timestamp_micros),
            },
            tenant_id,
        })
    }
}

/// Run the cross-tenant overdue query and decode each row into an
/// [`OverdueRow`]. Lives here (not on [`crate::persist::OrchestratorStore`])
/// because the persist API is read-only and only exposes per-session /
/// per-worker lookups; the walker needs an "overdue across tenants"
/// scan that the typed surface doesn't cover.
pub(crate) async fn find_overdue_handoffs(
    pool: &SqlitePool,
    now: DateTime<Utc>,
) -> ArgoResult<Vec<OverdueRow>> {
    let rows = sqlx::query_as::<_, RawOverdue>(
        "SELECT h.id AS handoff_id, h.session_id, s.tenant_id, h.from_worker_id, \
         h.\"to\" AS to_field, h.expected_outcome, h.payload, h.created_at, h.deadline \
         FROM handoff h JOIN session s ON s.id = h.session_id \
         WHERE h.status = 'pending' AND h.deadline IS NOT NULL AND h.deadline < ? \
         ORDER BY h.deadline ASC",
    )
    .bind(now.timestamp_micros())
    .fetch_all(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("walker overdue query: {e}")))?;
    rows.into_iter().map(OverdueRow::try_from).collect()
}

/// Parse the persisted `handoff."to"` JSON shape (mirrors
/// `persist::step_from_json` but lives here so the walker doesn't
/// import private symbols).
fn parse_step_from_persist_json(json: &str) -> ArgoResult<EscalationStep> {
    let v: Value = serde_json::from_str(json)
        .map_err(|e| ArgoError::Schema(format!("handoff.to JSON: {e}")))?;
    match v.get("kind").and_then(Value::as_str) {
        Some("worker") => {
            let raw = v
                .get("worker_id")
                .and_then(Value::as_str)
                .ok_or_else(|| ArgoError::Schema("handoff.to worker_id missing".into()))?;
            WorkerId::from_str(raw)
                .map(EscalationStep::Worker)
                .map_err(|e| ArgoError::Schema(format!("handoff.to worker_id: {e}")))
        }
        Some("human") => v
            .get("addr")
            .and_then(Value::as_str)
            .map(|s| EscalationStep::Human(s.to_string()))
            .ok_or_else(|| ArgoError::Schema("handoff.to addr missing".into())),
        _ => Err(ArgoError::Schema("handoff.to kind unknown".into())),
    }
}

pub(crate) fn parse_chain_from_payload(payload: &Value) -> Option<EscalationChain> {
    let arr = payload.get("escalation_chain")?.as_array()?;
    let strs: Vec<&str> = arr.iter().filter_map(Value::as_str).collect();
    Some(EscalationChain::from_strs(&strs))
}

pub(crate) fn parse_index_from_payload(payload: &Value) -> usize {
    payload
        .get("escalation_index")
        .and_then(Value::as_u64)
        .map(|n| n as usize)
        .unwrap_or(0)
}

pub(crate) fn stringify_step(step: &EscalationStep) -> String {
    match step {
        EscalationStep::Worker(id) => format!("worker:{id}"),
        EscalationStep::Human(addr) => format!("human:{addr}"),
    }
}

/// Insert an audit row labelled `escalation_exhausted` for the
/// supplied session. Mirrors what `AuditSink::write` would do but
/// avoids requiring a new variant on `AuditKind` (the brief
/// scopes the walker to `argo-orchestrator` + the gateway sweeper
/// — adding to `argo-audit` would expand the surface).
pub(crate) async fn write_escalation_exhausted_audit(
    pool: &SqlitePool,
    session_id: SessionId,
    payload: Value,
) -> ArgoResult<()> {
    let kind_label = "escalation_exhausted";
    let canonical = serde_json::to_string(&payload)
        .map_err(|e| ArgoError::Schema(format!("audit payload: {e}")))?;
    let session_blob = uuid::Uuid::parse_str(session_id.as_str())
        .map_err(|e| ArgoError::Schema(format!("session uuid: {e}")))?;
    let parent_hash = last_session_hash(pool, &session_blob).await?;
    let self_hash = compute_chain_hash(&parent_hash, kind_label, &canonical);
    let audit_id = uuid::Uuid::now_v7();
    sqlx::query(
        "INSERT INTO audit_record (id, session_id, kind, payload_json, parent_hash, \
         self_hash, created_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(audit_id.as_bytes().as_slice())
    .bind(session_blob.as_bytes().as_slice())
    .bind(kind_label)
    .bind(canonical)
    .bind(parent_hash.as_slice())
    .bind(self_hash.as_slice())
    .bind(Utc::now().timestamp_micros())
    .execute(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("audit insert: {e}")))?;
    Ok(())
}

async fn last_session_hash(pool: &SqlitePool, session: &uuid::Uuid) -> ArgoResult<[u8; 32]> {
    let row: Option<Vec<u8>> = sqlx::query_scalar(
        "SELECT self_hash FROM audit_record WHERE session_id = ? \
         ORDER BY seq DESC LIMIT 1",
    )
    .bind(session.as_bytes().as_slice())
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
