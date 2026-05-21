//! `POST /api/v1/audit` — record an out-of-band ("audit-the-audit")
//! entry (`web-gateway-endpoints-audit-mutations-batch`).
//!
//! Records operator inspection actions that happen outside the
//! normal agent loop — e.g. "operator manually verified row #1234
//! against the export bundle on 2026-05-21". Body:
//! `{ kind, body, related_action_id? }`.
//!
//! The row lands in the hash chain via [`AuditSink`] with
//! [`AuditKind::HttpRequest`]; the operator-supplied `kind` field
//! is preserved in `payload.annotation_kind` so the chain
//! verifier's `AuditKind::from_label` invariant stays intact (an
//! arbitrary string would otherwise trip `verify_session`).

use super::body_from_request;
use super::routes_auth_principal::principal_id_for_audit;
use super::store_unavailable;
use argo_audit::{AuditKind, AuditSink, DEFAULT_AUDIT_TENANT_ID};
use argo_store::SessionStore;
use serde_json::{json, Value};
use std::sync::Arc;

/// Annotation body length cap. 8 KiB is enough for an inspection
/// note + reference links without inviting payload-bloat abuse.
const BODY_MAX_LEN: usize = 8 * 1024;

/// Annotation kind length cap. The kind is a free-text
/// sub-classification, not a chain label — short labels keep the
/// audit feed scannable.
const KIND_MAX_LEN: usize = 64;

/// `POST /api/v1/audit` — record an out-of-band annotation.
pub(super) async fn record_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let parsed: Value = match serde_json::from_str(body_from_request(req)) {
        Ok(v) => v,
        Err(e) => return bad_request(&format!("parse body: {e}")),
    };
    let annotation_kind = match parsed.get("kind").and_then(Value::as_str) {
        Some(s) if !s.trim().is_empty() && s.len() <= KIND_MAX_LEN => s.trim().to_string(),
        Some(_) => {
            return bad_request(&format!(
                "`kind` must be a non-empty string ≤ {KIND_MAX_LEN} chars"
            ));
        }
        None => return bad_request("`kind` (string) is required"),
    };
    let annotation_body = match parsed.get("body").and_then(Value::as_str) {
        Some(s) if !s.is_empty() && s.len() <= BODY_MAX_LEN => s.to_string(),
        Some(_) => {
            return bad_request(&format!(
                "`body` must be a non-empty string ≤ {BODY_MAX_LEN} chars"
            ));
        }
        None => return bad_request("`body` (string) is required"),
    };
    let related_action_id = match parsed.get("related_action_id") {
        Some(Value::Null) | None => None,
        Some(Value::String(s)) if s.is_empty() => None,
        Some(Value::String(s)) => match validate_action_id(s) {
            Ok(()) => Some(s.clone()),
            Err(msg) => return bad_request(&msg),
        },
        _ => return bad_request("`related_action_id` must be a hex string or null"),
    };
    let principal = principal_id_for_audit(req);
    let tenant = resolve_tenant(store)
        .await
        .unwrap_or_else(|| DEFAULT_AUDIT_TENANT_ID.to_string());
    let sink = AuditSink::new_with_tenant(Arc::clone(store), tenant);
    let payload = json!({
        "endpoint": "audit.annotation",
        "annotation_kind": annotation_kind,
        "body": annotation_body,
        "related_action_id": related_action_id,
        "recorded_by": principal,
    });
    if let Err(e) = sink
        .write(None, AuditKind::HttpRequest, payload, principal.clone())
        .await
    {
        return server_error(&format!("write annotation: {e}"));
    }
    // Pull the seq + id we just wrote so the response carries the
    // chain coordinates the operator can quote in a follow-up.
    let row: (i64, Vec<u8>) = match sqlx::query_as::<_, (i64, Vec<u8>)>(
        "SELECT seq, id FROM audit_record WHERE kind = ? \
         ORDER BY seq DESC LIMIT 1",
    )
    .bind(AuditKind::HttpRequest.label())
    .fetch_one(store.pool())
    .await
    {
        Ok(r) => r,
        Err(e) => return server_error(&format!("read inserted row: {e}")),
    };
    let body = json!({
        "seq": row.0,
        "audit_id": hex::encode(&row.1),
        "kind": AuditKind::HttpRequest.label(),
        "annotation_kind": annotation_kind,
        "related_action_id": related_action_id,
        "recorded_at": chrono::Utc::now().to_rfc3339(),
    });
    ("201 Created", body.to_string())
}

fn validate_action_id(s: &str) -> Result<(), String> {
    if hex::decode(s).map(|b| b.len() == 16).unwrap_or(false) {
        Ok(())
    } else {
        Err("`related_action_id` must be a 32-char hex audit id".to_string())
    }
}

async fn resolve_tenant(store: &Arc<SessionStore>) -> Option<String> {
    let cwd = std::env::current_dir().ok()?;
    let name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (tenant, _ws) = store.ensure_workspace(name, &root).await.ok()?;
    Some(tenant.0.to_string())
}

fn bad_request(message: &str) -> (&'static str, String) {
    error_envelope("400 Bad Request", "bad_request", message)
}

fn server_error(message: &str) -> (&'static str, String) {
    error_envelope("500 Internal Server Error", "internal_error", message)
}

fn error_envelope(status: &'static str, code: &str, message: &str) -> (&'static str, String) {
    (
        status,
        json!({
            "error": { "code": code, "message": message }
        })
        .to_string(),
    )
}

#[cfg(test)]
#[path = "routes_audit_record_tests.rs"]
mod tests;
