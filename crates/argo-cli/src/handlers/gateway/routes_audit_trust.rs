//! Customer-facing audit **trust surface** (`w3-audit-trust-surface`).
//!
//! The tamper-evident, hash-chained audit log is SKYCOT's compliance
//! moat. Until now it was provable only from the CLI (`argo audit
//! verify`, `argo audit export`). This module exposes three
//! authenticated, **tenant-scoped, read-only** HTTP endpoints so a
//! customer can self-serve prove their log is intact and hand a bundle
//! to their regulator — without operator involvement:
//!
//! - `GET /api/v1/audit/verify` — runs the existing chain-integrity
//!   verifier ([`argo_audit::verify_tenant`]) over the CALLING
//!   tenant's audit chain. Returns `{ intact, rows_checked,
//!   first_broken_seq?, head_hash, verified_at }`. Truth-bounded: a
//!   broken chain reports `intact: false` with the first offending
//!   `seq` — it never fakes "intact".
//! - `GET /api/v1/audit/export[?format=json|csv|tar]` — returns the
//!   existing compliance evidence bundle
//!   ([`argo_audit::export_bundle`]) for the calling tenant, base64-
//!   wrapped (the v1 listener hard-codes a JSON content-type). The
//!   self-contained artefact a customer hands their regulator;
//!   includes `verify.sh` for offline re-verification.
//! - `GET /api/v1/audit/cert/<action_id>` — a per-action "verified-
//!   delivery certificate": gathers the audit row identified by
//!   `<action_id>` (hex of the row's UUIDv7 id), plus its full
//!   owning-session chain, recomputes the hashes, and returns a
//!   self-contained cert proving the row is anchored in an intact
//!   chain. 404 when the id is not in the CALLING tenant's chain —
//!   a cross-tenant id leaks nothing.
//!
//! ## Tenant isolation (load-bearing)
//!
//! Every endpoint resolves the tenant from the AUTHENTICATED SESSION
//! via [`super::resolve_tenant::resolve_tenant_for_request`], NOT a
//! client-supplied header/query. That resolver prefers the
//! cryptographically-verified `__Host-session` / bearer session-JWT
//! `tnt` claim; an explicit `?tenant=` / `X-Tenant-Id` override is
//! honored only on the no-session path AND only when the perimeter is
//! open (dev/demo). So an authenticated caller can ONLY ever read its
//! own tenant's chain, and an unauthenticated production caller can
//! never select an arbitrary tenant. This is the strict-isolation
//! invariant the trust surface depends on.

use super::resolve_tenant::resolve_tenant_for_request;
use super::role_gate::{enforce_role, Action};
use super::store_unavailable;
use argo_audit::export_bundle::{build_bundle, fetch_rows_for_export, BundleFormat, ExportWindow};
use argo_audit::{compute_chain_hash, tenant_head_hash, verify_tenant};
use argo_store::SessionStore;
use base64::{engine::general_purpose, Engine as _};
use serde_json::json;
use std::sync::Arc;

/// `GET /api/v1/audit/verify` — prove the calling tenant's chain is
/// intact. See module docs. Read-only; never mutates the chain.
pub(super) async fn verify_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    if let Some(deny) = enforce_role(req, Action::AuditDetailRead) {
        return deny;
    }
    let Some(store) = store else {
        return store_unavailable();
    };
    let tenant_id = resolve_tenant_for_request(req, store)
        .await
        .map(|t| t.to_string())
        .unwrap_or_default();
    let report = match verify_tenant(store, &tenant_id).await {
        Ok(r) => r,
        Err(e) => return server_error(&format!("verify_tenant: {e}")),
    };
    let head_hash = match tenant_head_hash(store, &tenant_id).await {
        Ok(h) => h,
        Err(e) => return server_error(&format!("tenant_head_hash: {e}")),
    };
    // Truth-bounded: report the REAL state. The first broken row's
    // seq is surfaced so a customer can pinpoint the tamper; absent
    // when the chain is intact.
    let first_broken_seq = report.breaks.iter().map(|b| b.seq).min();
    let body = json!({
        "intact": report.ok,
        "rows_checked": report.rows,
        "breaks": report.breaks.len(),
        "first_broken_seq": first_broken_seq,
        "head_hash": head_hash,
        "verified_at": chrono::Utc::now().to_rfc3339(),
        "tenant_id": tenant_id,
    });
    ("200 OK", body.to_string())
}

/// `GET /api/v1/audit/export[?format=json|csv|tar]` — compliance
/// bundle for the calling tenant. Owner-only (a full-tenant data
/// export, gated like the other GDPR-style exports). Read-only.
pub(super) async fn export_response(
    req: &str,
    query: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    if let Some(deny) = enforce_role(req, Action::DataExport) {
        return deny;
    }
    let Some(store) = store else {
        return store_unavailable();
    };
    let format = match query_kv(query, "format") {
        Some(raw) => match BundleFormat::parse(&raw) {
            Some(f) => f,
            None => return bad_request(&format!("unknown format `{raw}` (allowed: json|csv|tar)")),
        },
        None => BundleFormat::Json,
    };
    let tenant_id = resolve_tenant_for_request(req, store)
        .await
        .map(|t| t.to_string())
        .unwrap_or_default();
    // Whole-history window — the regulator bundle is the full log.
    let rows = match fetch_rows_for_export(store, &tenant_id, &ExportWindow::default()).await {
        Ok(r) => r,
        Err(e) => return server_error(&format!("fetch rows: {e}")),
    };
    let bytes = match build_bundle(&rows, format) {
        Ok(b) => b,
        Err(e) => return server_error(&format!("build bundle: {e}")),
    };
    let body = json!({
        "format": format,
        "content_type": format.content_type(),
        "filename": format!("audit-bundle.{}", format.extension()),
        "row_count": rows.len(),
        "size_bytes": bytes.len(),
        "sha256": sha256_hex(&bytes),
        "tenant_id": tenant_id,
        "b64": general_purpose::STANDARD.encode(&bytes),
    });
    ("200 OK", body.to_string())
}

/// `GET /api/v1/audit/cert/<action_id>` — per-action verified-delivery
/// certificate. See module docs. 404 (existence-hide) when the id is
/// not in the calling tenant's chain.
pub(super) async fn cert_response(
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
    let tenant_id = resolve_tenant_for_request(req, store)
        .await
        .map(|t| t.to_string())
        .unwrap_or_default();
    let target = match fetch_cert_row(store, &id_bytes, &tenant_id).await {
        Ok(Some(r)) => r,
        Ok(None) => return not_found(),
        Err(e) => return server_error(&format!("cert row fetch: {e}")),
    };
    // Gather the owning-session chain (or the row itself when it has
    // no session) so the cert carries the linking proof, then verify
    // those rows recompute cleanly.
    let chain = match fetch_session_chain(store, &target, &tenant_id).await {
        Ok(rows) => rows,
        Err(e) => return server_error(&format!("cert chain fetch: {e}")),
    };
    let (verified, proof) = verify_and_build_proof(&chain, &id_bytes);
    let body = json!({
        "certificate": {
            "action_id": action_id,
            "session_id": target.session_id.as_ref().map(hex::encode),
            "kind": target.kind,
            "self_hash": hex::encode(&target.self_hash),
            "verified": verified,
            "chain_rows": chain.len(),
            "proof": proof,
            "tenant_id": tenant_id,
            "issued_at": chrono::Utc::now().to_rfc3339(),
        }
    });
    ("200 OK", body.to_string())
}

#[derive(sqlx::FromRow)]
struct CertRow {
    id: Vec<u8>,
    session_id: Option<Vec<u8>>,
    kind: String,
    payload_json: String,
    parent_hash: Vec<u8>,
    self_hash: Vec<u8>,
    seq: i64,
}

/// Fetch the single target row scoped to the calling tenant. Returns
/// `None` (→ 404) for an id absent from this tenant's chain.
async fn fetch_cert_row(
    store: &Arc<SessionStore>,
    id_bytes: &[u8],
    tenant_id: &str,
) -> Result<Option<CertRow>, sqlx::Error> {
    sqlx::query_as::<_, CertRow>(
        "SELECT id, session_id, kind, payload_json, parent_hash, self_hash, seq \
         FROM audit_record WHERE id = ? AND tenant_id = ? LIMIT 1",
    )
    .bind(id_bytes)
    .bind(tenant_id)
    .fetch_all(store.pool())
    .await
    .map(|mut v| v.pop())
}

/// Fetch the full chain the target row belongs to, scoped to the
/// tenant. Session-bound rows return the whole session chain in `seq`
/// order; a session-less row returns just itself.
async fn fetch_session_chain(
    store: &Arc<SessionStore>,
    target: &CertRow,
    tenant_id: &str,
) -> Result<Vec<CertRow>, sqlx::Error> {
    let Some(session_id) = target.session_id.as_ref() else {
        // Session-less row — the cert covers the single row.
        return Ok(vec![CertRow {
            id: target.id.clone(),
            session_id: None,
            kind: target.kind.clone(),
            payload_json: target.payload_json.clone(),
            parent_hash: target.parent_hash.clone(),
            self_hash: target.self_hash.clone(),
            seq: target.seq,
        }]);
    };
    sqlx::query_as::<_, CertRow>(
        "SELECT id, session_id, kind, payload_json, parent_hash, self_hash, seq \
         FROM audit_record WHERE session_id = ? AND tenant_id = ? ORDER BY seq ASC",
    )
    .bind(session_id.as_slice())
    .bind(tenant_id)
    .fetch_all(store.pool())
    .await
}

/// Recompute the chain hashes for `chain` (already in `seq` order) and
/// build the per-row linking proof. `verified` is true only when every
/// row's `self_hash` recomputes and links to the prior row's
/// `self_hash`. The proof lists each row's seq + truncated hashes so a
/// customer can eyeball the linkage; the `target` row is flagged.
fn verify_and_build_proof(chain: &[CertRow], target_id: &[u8]) -> (bool, serde_json::Value) {
    let mut verified = true;
    let mut prev: Option<[u8; 32]> = None;
    let mut steps = Vec::with_capacity(chain.len());
    for row in chain {
        let parent: [u8; 32] = row.parent_hash.as_slice().try_into().unwrap_or([0u8; 32]);
        let self_h: [u8; 32] = row.self_hash.as_slice().try_into().unwrap_or([0u8; 32]);
        let links = prev.is_none_or(|p| p == parent);
        let recomputed = compute_chain_hash(&parent, &row.kind, &row.payload_json);
        let hash_ok = recomputed == self_h;
        if !links || !hash_ok {
            verified = false;
        }
        steps.push(json!({
            "seq": row.seq,
            "kind": row.kind,
            "self_hash": hex::encode(&row.self_hash),
            "parent_hash": hex::encode(&row.parent_hash),
            "links_to_parent": links,
            "hash_recomputes": hash_ok,
            "is_target": row.id.as_slice() == target_id,
        }));
        prev = Some(self_h);
    }
    (verified, serde_json::Value::Array(steps))
}

fn query_kv(query: &str, key: &str) -> Option<String> {
    query.split('&').find_map(|kv| {
        let (k, v) = kv.split_once('=')?;
        (k == key && !v.is_empty()).then(|| v.to_string())
    })
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(bytes);
    hex::encode(h.finalize())
}

fn not_found() -> (&'static str, String) {
    (
        "404 Not Found",
        json!({"error": {"code": "not_found", "message": "audit row not found"}}).to_string(),
    )
}

fn bad_request(message: &str) -> (&'static str, String) {
    (
        "400 Bad Request",
        json!({"error": {"code": "bad_request", "message": message}}).to_string(),
    )
}

fn server_error(message: &str) -> (&'static str, String) {
    (
        "500 Internal Server Error",
        json!({"error": {"code": "internal_error", "message": message}}).to_string(),
    )
}

#[cfg(test)]
#[path = "routes_audit_trust_tests.rs"]
mod tests;
