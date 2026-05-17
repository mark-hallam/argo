use argo_audit::filter_dims::extract_filter_dims;
use argo_audit::{compute_chain_hash, AuditKind};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, SessionId};
use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

pub struct SeededAudit {
    pub sessions: Vec<SessionId>,
    pub since_micros: i64,
}

pub async fn seed_chains(
    store: &SessionStore,
    sessions: usize,
    rows_per_session: usize,
) -> ArgoResult<SeededAudit> {
    let tenant_id = "bench-tenant";
    let since_micros = Utc::now().timestamp_micros();
    let mut tx = store
        .pool()
        .begin()
        .await
        .map_err(|e| ArgoError::Provider(format!("bench seed tx: {e}")))?;
    let mut session_ids = Vec::with_capacity(sessions);
    for session_idx in 0..sessions {
        let session_id = SessionId::new();
        session_ids.push(session_id);
        let mut parent_hash = [0_u8; 32];
        for row_idx in 0..rows_per_session {
            let row_id = Uuid::now_v7();
            let created_at = since_micros + i64::try_from(row_idx).unwrap_or(i64::MAX);
            let payload = payload_for(tenant_id, session_idx, row_idx);
            let canonical = serde_json::to_string(&payload)
                .map_err(|e| ArgoError::Schema(format!("bench payload: {e}")))?;
            let kind = AuditKind::ToolCall.label();
            let self_hash = compute_chain_hash(&parent_hash, kind, &canonical);
            let dims = extract_filter_dims(&payload);
            sqlx::query(
                "INSERT INTO audit_record (id, session_id, kind, payload_json, \
                 parent_hash, self_hash, created_at, principal_id, tenant_id, \
                 worker_id, channel, action_type, outcome_status, approver_id, \
                 cost_band, data_classification) VALUES (?, ?, ?, ?, ?, ?, ?, \
                 NULL, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(row_id.as_bytes().as_slice())
            .bind(session_id.0.as_bytes().as_slice())
            .bind(kind)
            .bind(&canonical)
            .bind(parent_hash.as_slice())
            .bind(self_hash.as_slice())
            .bind(created_at)
            .bind(tenant_id)
            .bind(dims.worker_id.as_deref())
            .bind(dims.channel.as_deref())
            .bind(dims.action_type.as_deref())
            .bind(dims.outcome_status.as_deref())
            .bind(dims.approver_id.as_deref())
            .bind(dims.cost_band.as_deref())
            .bind(dims.data_classification.as_deref())
            .execute(&mut *tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("bench seed insert: {e}")))?;
            parent_hash = self_hash;
        }
    }
    tx.commit()
        .await
        .map_err(|e| ArgoError::Provider(format!("bench seed commit: {e}")))?;
    Ok(SeededAudit {
        sessions: session_ids,
        since_micros,
    })
}

fn payload_for(tenant_id: &str, session_idx: usize, row_idx: usize) -> serde_json::Value {
    json!({
        "tenant_id": tenant_id,
        "worker_id": format!("worker-{}", session_idx % 10),
        "worker_name": format!("Bench Worker {}", session_idx % 10),
        "channel": "web",
        "action_type": "tool_call",
        "outcome_status": if row_idx.is_multiple_of(20) { "failed" } else { "succeeded" },
        "approver_id": format!("approver-{}", row_idx % 4),
        "cost_usd": 0.001_f64,
        "data_classification": "internal",
        "verb": format!("bench audit row {row_idx}"),
    })
}
