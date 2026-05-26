//! `GET /api/v1/audit/<actionId>` — full AuditDetail (Stream J15).
//! Read-only over `audit_record`; the 8-step decision chain,
//! provenance, output, and external-ref pre-scan are all composed
//! from `payload_json` (v1 audit schema doesn't break the fields
//! out as columns). Single tenant per workspace today, so
//! "cross-tenant" 404 is structurally guaranteed by the unknown-id
//! path. J2 redaction is applied to summary, inputs, decision-chain
//! bodies, and the output body before serialisation.
//!
//! Helper modules:
//! - [`super::routes_audit_detail_render`] — row → JSON renderers (summary,
//!   inputs, decision chain, provenance, output, screenshots).
//! - [`super::routes_audit_detail_refs`] — external-reference scanners
//!   (Stripe, Slack, GitHub, Linear, generic URL substring matches).

use super::rate_limit;
use super::role_gate::{enforce_role, Action};
use super::routes_audit_detail_render::{build_detail, payload_str};
use super::routes_auth_principal::principal_id_for_audit;
use super::store_unavailable;
use argo_audit::{AuditKind, AuditSink, DEFAULT_AUDIT_TENANT_ID};
use argo_redact::RedactionPolicy;
use argo_store::SessionStore;
use serde_json::{json, Value};
use std::sync::Arc;

const LEGACY_AUDIT_TENANT_ID: &str = "__legacy__";

/// `GET /api/v1/audit/<actionId>` — see module docs.
pub(super) async fn detail_response(
    req: &str,
    action_id: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    if let Some(deny) = enforce_role(req, Action::AuditDetailRead) {
        return deny;
    }
    let Some(store) = store else {
        return store_unavailable();
    };
    let id_bytes = match hex::decode(action_id) {
        Ok(b) if b.len() == 16 => b,
        _ => return not_found(),
    };
    let tenant_id = request_tenant_id(req);
    let row = match fetch_row(store, &id_bytes, &tenant_id).await {
        Ok(Some(r)) => r,
        Ok(None) => return not_found(),
        Err(e) => return server_error(&format!("audit detail fetch: {e}")),
    };
    let payload: Value = serde_json::from_str(&row.payload_json).unwrap_or(Value::Null);
    let policy = RedactionPolicy::default();
    let delegation_chain = match build_delegation_chain(store, &tenant_id, &row, &payload).await {
        Ok(chain) => chain,
        Err(e) => return server_error(&format!("audit delegation chain fetch: {e}")),
    };
    let mut detail = build_detail(action_id, &row, &payload, &policy);
    if let Value::Object(obj) = &mut detail {
        obj.insert("delegation_chain".into(), delegation_chain);
    }
    let body = json!({
        "detail": detail,
    });
    let audit_tenant =
        payload_str(&payload, "tenant_id").unwrap_or_else(|| rate_limit::extract_tenant(req));
    emit_view_audit(store, req, &audit_tenant, action_id).await;
    ("200 OK", body.to_string())
}

async fn build_delegation_chain(
    store: &Arc<SessionStore>,
    tenant_id: &str,
    row: &DetailRow,
    payload: &Value,
) -> Result<Value, sqlx::Error> {
    let Some(handoff_id) = handoff_id(payload) else {
        return Ok(Value::Array(Vec::new()));
    };
    let rows = fetch_handoff_rows(store, tenant_id, &handoff_id).await?;
    if rows.is_empty() {
        return Ok(Value::Array(vec![delegation_step(
            row, payload, "executed",
        )]));
    }
    let last_id = rows.last().map(|r| r.id.as_slice());
    Ok(Value::Array(
        rows.iter()
            .enumerate()
            .map(|(idx, r)| {
                let row_payload: Value =
                    serde_json::from_str(&r.payload_json).unwrap_or(Value::Null);
                let role = delegation_role(idx, r.id.as_slice(), last_id);
                delegation_step(r, &row_payload, role)
            })
            .collect(),
    ))
}

async fn fetch_handoff_rows(
    store: &Arc<SessionStore>,
    tenant_id: &str,
    handoff_id: &str,
) -> Result<Vec<DetailRow>, sqlx::Error> {
    sqlx::query_as::<_, DetailRow>(
        "SELECT id, session_id, kind, payload_json, self_hash, created_at \
         FROM audit_record \
         WHERE (tenant_id = ? OR (? = ? AND tenant_id = ?)) \
           AND (json_extract(payload_json, '$.current_handoff_id') = ? \
                OR json_extract(payload_json, '$.handoff_id') = ? \
                OR json_extract(payload_json, '$.handoffId') = ?) \
         ORDER BY created_at ASC, seq ASC \
         LIMIT 10",
    )
    .bind(tenant_id)
    .bind(tenant_id)
    .bind(DEFAULT_AUDIT_TENANT_ID)
    .bind(LEGACY_AUDIT_TENANT_ID)
    .bind(handoff_id)
    .bind(handoff_id)
    .bind(handoff_id)
    .fetch_all(store.pool())
    .await
}

fn handoff_id(payload: &Value) -> Option<String> {
    payload_str(payload, "current_handoff_id")
        .or_else(|| payload_str(payload, "handoff_id"))
        .or_else(|| payload_str(payload, "handoffId"))
}

fn delegation_role(idx: usize, id: &[u8], last_id: Option<&[u8]>) -> &'static str {
    if idx == 0 {
        "originator"
    } else if Some(id) == last_id {
        "executed"
    } else {
        "delegated_to"
    }
}

fn delegation_step(row: &DetailRow, payload: &Value, role: &str) -> Value {
    let worker_id = payload_str(payload, "worker_id").unwrap_or_default();
    let worker_name = payload_str(payload, "worker_name").unwrap_or_else(|| {
        if worker_id.is_empty() {
            "unknown".into()
        } else {
            worker_id.clone()
        }
    });
    let timestamp = chrono::DateTime::<chrono::Utc>::from_timestamp_micros(row.created_at)
        .map_or_else(|| row.created_at.to_string(), |dt| dt.to_rfc3339());
    json!({
        "worker_id": worker_id,
        "worker_name": worker_name,
        "action_id": hex::encode(&row.id),
        "role": role,
        "ts": timestamp,
    })
}

#[derive(sqlx::FromRow)]
pub(super) struct DetailRow {
    pub(super) id: Vec<u8>,
    pub(super) session_id: Option<Vec<u8>>,
    pub(super) kind: String,
    pub(super) payload_json: String,
    pub(super) self_hash: Vec<u8>,
    pub(super) created_at: i64,
}

async fn fetch_row(
    store: &Arc<SessionStore>,
    id_bytes: &[u8],
    tenant_id: &str,
) -> Result<Option<DetailRow>, sqlx::Error> {
    sqlx::query_as::<_, DetailRow>(
        "SELECT id, session_id, kind, payload_json, self_hash, created_at \
         FROM audit_record \
         WHERE id = ? \
           AND (tenant_id = ? OR (? = ? AND tenant_id = ?)) \
         LIMIT 1",
    )
    .bind(id_bytes)
    .bind(tenant_id)
    .bind(tenant_id)
    .bind(DEFAULT_AUDIT_TENANT_ID)
    .bind(LEGACY_AUDIT_TENANT_ID)
    .fetch_optional(store.pool())
    .await
}

fn request_tenant_id(req: &str) -> String {
    header_value(req, "x-argo-tenant-id").unwrap_or_else(|| DEFAULT_AUDIT_TENANT_ID.to_string())
}

fn header_value(req: &str, name_lower: &str) -> Option<String> {
    let headers = req
        .split_once("\r\n\r\n")
        .map_or(req, |(headers, _body)| headers);
    headers.lines().skip(1).find_map(|line| {
        let (name, value) = line.split_once(':')?;
        name.trim()
            .eq_ignore_ascii_case(name_lower)
            .then(|| value.trim().to_string())
            .filter(|s| !s.is_empty())
    })
}

async fn emit_view_audit(store: &Arc<SessionStore>, req: &str, tenant_id: &str, action_id: &str) {
    let payload = json!({
        "endpoint": "audit.detail.viewed",
        "viewed_action_id": action_id,
    });
    let sink = AuditSink::new_with_tenant(Arc::clone(store), tenant_id.to_string());
    if let Err(e) = sink
        .write(
            None,
            AuditKind::HttpRequest,
            payload,
            principal_id_for_audit(req),
        )
        .await
    {
        tracing::warn!(
            target: "gateway::audit_detail",
            error = %e,
            "audit.detail.viewed write failed",
        );
    }
}

fn not_found() -> (&'static str, String) {
    (
        "404 Not Found",
        json!({"error": {"code": "not_found", "message": "audit row not found"}}).to_string(),
    )
}

fn server_error(message: &str) -> (&'static str, String) {
    (
        "500 Internal Server Error",
        json!({"error": {"code": "internal_error", "message": message}}).to_string(),
    )
}

#[cfg(test)]
#[path = "routes_audit_detail_tests.rs"]
mod tests;
