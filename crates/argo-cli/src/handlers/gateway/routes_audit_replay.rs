//! `POST /api/v1/audit/<id>/replay` — sandboxed replay of one audit
//! row (`web-gateway-endpoints-audit-mutations-batch`).
//!
//! Forensic operation: re-canonicalize the stored payload, recompute
//! the chain hash from `(parent_hash, kind, payload_json)`, and
//! compare against the persisted `self_hash`. Returns a verdict
//! object so an auditor can decide whether the row is intact or has
//! been tampered with after-the-fact.
//!
//! Determinism: the chain hash is SHA-256 of fixed inputs read from
//! the row itself, so two replays of the same id always produce the
//! same verdict. No external state is touched.
//!
//! Distinct from [`argo_replay`], which replays whole sessions. The
//! audit-row replay is row-scoped chain integrity, not turn-scoped
//! model re-execution.

use super::routes_auth_principal::principal_id_for_audit;
use super::store_unavailable;
use argo_audit::{compute_chain_hash, AuditKind, AuditSink, DEFAULT_AUDIT_TENANT_ID};
use argo_store::SessionStore;
use serde_json::{json, Value};
use std::sync::Arc;

#[derive(sqlx::FromRow)]
struct ReplayRow {
    seq: i64,
    session_id: Option<Vec<u8>>,
    kind: String,
    payload_json: String,
    parent_hash: Vec<u8>,
    self_hash: Vec<u8>,
    created_at: i64,
}

/// Build the verdict body for a `POST /api/v1/audit/<id>/replay`
/// call. Pure read over `audit_record`; the side-effect is one
/// `http.request` audit row that records the replay action.
pub(super) async fn replay_response(
    audit_id: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let id_bytes = match hex::decode(audit_id) {
        Ok(b) if b.len() == 16 => b,
        _ => return bad_request("audit id must be 32-char hex (16 bytes)"),
    };
    let tenant_id = request_tenant_id(req);
    let row = match sqlx::query_as::<_, ReplayRow>(
        "SELECT seq, session_id, kind, payload_json, parent_hash, self_hash, created_at \
         FROM audit_record WHERE id = ? AND tenant_id = ? LIMIT 1",
    )
    .bind(&id_bytes)
    .bind(&tenant_id)
    .fetch_optional(store.pool())
    .await
    {
        Ok(Some(r)) => r,
        Ok(None) => return not_found(audit_id),
        Err(e) => return server_error(&format!("audit replay fetch: {e}")),
    };
    let parent = match to_32(&row.parent_hash) {
        Some(p) => p,
        None => return server_error("stored parent_hash is not 32 bytes — corrupted row"),
    };
    let expected_hex = hex::encode(&row.self_hash);
    let recomputed = compute_chain_hash(&parent, &row.kind, &row.payload_json);
    let actual_hex = hex::encode(recomputed);
    let kind_known = AuditKind::from_label(&row.kind).is_some();
    let hashes_match = actual_hex == expected_hex;
    let verdict = match (kind_known, hashes_match) {
        (true, true) => "same",
        (true, false) => "differ",
        (false, _) => "error",
    };
    let diff = build_diff(verdict, &row.kind, hashes_match, kind_known);
    emit_replay_audit(store, req, &tenant_id, audit_id, verdict).await;
    let body = json!({
        "audit_id": audit_id,
        "seq": row.seq,
        "kind": row.kind,
        "session_id": row.session_id.as_deref().map(hex::encode),
        "created_at_micros": row.created_at,
        "verdict": verdict,
        "expected_self_hash": expected_hex,
        "actual_self_hash": actual_hex,
        "kind_label_known": kind_known,
        "diff": diff,
        "replayed_at": chrono::Utc::now().to_rfc3339(),
    });
    ("200 OK", body.to_string())
}

fn build_diff(verdict: &str, kind: &str, hashes_match: bool, kind_known: bool) -> Value {
    let mut obj = serde_json::Map::new();
    if verdict == "same" {
        return Value::Object(obj);
    }
    if !kind_known {
        obj.insert(
            "kind".to_string(),
            json!(format!("unknown audit kind label `{kind}`")),
        );
    }
    if !hashes_match {
        obj.insert(
            "self_hash".to_string(),
            json!("recomputed self_hash does not match stored self_hash"),
        );
    }
    Value::Object(obj)
}

fn to_32(b: &[u8]) -> Option<[u8; 32]> {
    if b.len() != 32 {
        return None;
    }
    let mut buf = [0u8; 32];
    buf.copy_from_slice(b);
    Some(buf)
}

async fn emit_replay_audit(
    store: &Arc<SessionStore>,
    req: &str,
    tenant_id: &str,
    audit_id: &str,
    verdict: &str,
) {
    let payload = json!({
        "endpoint": "audit.replay",
        "replayed_audit_id": audit_id,
        "verdict": verdict,
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
            target: "gateway::audit_replay",
            error = %e,
            "audit.replay write failed",
        );
    }
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

fn bad_request(message: &str) -> (&'static str, String) {
    error_envelope("400 Bad Request", "bad_request", message)
}

fn not_found(id: &str) -> (&'static str, String) {
    error_envelope(
        "404 Not Found",
        "not_found",
        &format!("audit row `{id}` not found"),
    )
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
#[path = "routes_audit_replay_tests.rs"]
mod tests;
