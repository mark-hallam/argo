//! `POST /api/v1/workers` — hire-commit handler (Stream J5).
//!
//! HTTP shim over [`argo_identity::IdentityStore::create_worker_from_draft`].
//! The frontend E2-e hire flow POSTs the J4 worker-draft + the
//! preceding classification context (`problem` + `answers`) and gets
//! back the new `worker_id`.
//!
//! Bearer-auth is gated upstream by `routes_dispatch_post::check_bearer_auth`.
//!
//! ## v1 posture (mirrors the brief)
//!
//! - Tenant binding: cwd-derived via `SessionStore::ensure_workspace`,
//!   matching the existing quota / worker_actions handlers. Per-tenant
//!   gateway routing is a `mission-enterprise-*` follow-up.
//! - Idempotency: process-local `(tenant, Idempotency-Key) → worker_id`
//!   map; 24h TTL via lazy sweep. Multi-instance gateway gets Redis in
//!   Track-H (`mission-enterprise-hire-idempotency-redis`).
//! - Audit row: full hire context with email/phone redacted via a
//!   narrow inline regex pass — `argo-redact::redact_text` (J2) is the
//!   real fence once it lands; until then, basic regex covers the
//!   common case without taking a new workspace dep.
//! - Vertical id: validated against the on-disk vertical catalog
//!   (same loader the `GET /verticals` route uses).
//! - Quota: no per-tenant worker-count cap — see
//!   `mission-enterprise-tenant-worker-quota` for the real limit.

use super::body_from_request;
use super::routes_auth_principal::principal_id_for_audit;
use argo_audit::{AuditKind, AuditSink};
use argo_identity::{HireDraft, IdentityStore, WorkerId};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

const IDEMPOTENCY_HEADER: &str = "Idempotency-Key";
const IDEMPOTENCY_TTL_SECS: i64 = 24 * 60 * 60;

/// `POST /api/v1/workers` — commit a hire from the E2-e draft.
///
/// On success returns `201 {worker_id, success: true}`. Bearer auth +
/// principal-header parsing are upstream of this handler; this entry
/// point trusts both and binds the new worker to the workspace's
/// cwd-derived tenant.
pub(super) async fn hire_response(
    req: &str,
    store: Option<&std::sync::Arc<SessionStore>>,
) -> (&'static str, String) {
    let body = body_from_request(req);
    let parsed: Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => return bad_request(&format!("parse body: {e}")),
    };
    let problem = match parsed.get("problem").and_then(Value::as_str) {
        Some(s) if !s.trim().is_empty() => s.trim().to_string(),
        _ => return bad_request("`problem` (non-empty string) is required"),
    };
    let answers = match parsed.get("answers") {
        Some(v) if v.is_object() => v.clone(),
        _ => return bad_request("`answers` (object) is required"),
    };
    let draft: HireDraft = match parsed
        .get("worker_draft")
        .ok_or("`worker_draft` (object) is required")
        .and_then(|v| serde_json::from_value(v.clone()).map_err(|_| "worker_draft: invalid shape"))
    {
        Ok(d) => d,
        Err(msg) => return bad_request(msg),
    };
    let store = match store {
        Some(s) => s,
        None => return super::store_unavailable(),
    };

    let (tenant_id, principal_id) = match resolve_tenant_principal(store, req).await {
        Ok(pair) => pair,
        Err(e) => return server_error(&format!("resolve tenant: {e}")),
    };

    if !vertical_exists(&draft.suggested_vertical) {
        return bad_request(&format!(
            "unknown vertical '{}'",
            draft.suggested_vertical
        ));
    }

    let idem_key = idempotency_key(req);
    let body_digest = digest_body(body);
    if let Some(key) = idem_key.as_deref() {
        match check_idempotency(tenant_id, key, &body_digest) {
            IdempotencyHit::Match(existing) => {
                return ok_idempotent(existing, draft.suggested_vertical.as_str());
            }
            IdempotencyHit::Conflict => {
                return conflict_response(key);
            }
            IdempotencyHit::None => {}
        }
    }

    let identities = IdentityStore::new(store);
    let new_id = match identities
        .create_worker_from_draft(&draft, tenant_id, &principal_id)
        .await
    {
        Ok(id) => id,
        Err(ArgoError::Schema(msg)) => return bad_request(&msg),
        Err(e) => return server_error(&format!("create_worker: {e}")),
    };

    if let Some(key) = idem_key.as_deref() {
        remember_idempotency(tenant_id, key, &body_digest, new_id);
    }

    emit_hire_audit(store, &principal_id, new_id, &problem, &answers, &draft).await;
    (
        "201 Created",
        json!({
            "worker_id": new_id.to_string(),
            "success": true,
        })
        .to_string(),
    )
}

async fn resolve_tenant_principal(
    store: &SessionStore,
    req: &str,
) -> ArgoResult<(TenantId, String)> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (tenant, _ws) = store.ensure_workspace(name, &root).await?;
    let principal_id =
        principal_id_for_audit(req).unwrap_or_else(|| "system:hire-endpoint".to_string());
    Ok((tenant, principal_id))
}

fn vertical_exists(id: &str) -> bool {
    let dir = crate::handlers::vertical::resolve_verticals_dir();
    let Ok((registry, _errors)) = argo_vertical::load_dir(&dir) else {
        return false;
    };
    registry.get(id).is_some()
}

fn idempotency_key(req: &str) -> Option<String> {
    let headers = req
        .split_once("\r\n\r\n")
        .map_or(req, |(headers, _body)| headers);
    headers.lines().skip(1).find_map(|line| {
        let (name, value) = line.split_once(':')?;
        name.eq_ignore_ascii_case(IDEMPOTENCY_HEADER)
            .then(|| value.trim().to_string())
            .filter(|s| !s.is_empty())
    })
}

fn digest_body(body: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(body.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[derive(Debug)]
enum IdempotencyHit {
    Match(WorkerId),
    Conflict,
    None,
}

struct IdemEntry {
    body_digest: String,
    worker_id: WorkerId,
    at: DateTime<Utc>,
}

fn idem_cache() -> &'static Mutex<HashMap<(TenantId, String), IdemEntry>> {
    static CACHE: OnceLock<Mutex<HashMap<(TenantId, String), IdemEntry>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn check_idempotency(tenant: TenantId, key: &str, body_digest: &str) -> IdempotencyHit {
    let Ok(mut guard) = idem_cache().lock() else {
        return IdempotencyHit::None;
    };
    sweep_expired(&mut guard);
    match guard.get(&(tenant, key.to_string())) {
        Some(entry) if entry.body_digest == body_digest => IdempotencyHit::Match(entry.worker_id),
        Some(_) => IdempotencyHit::Conflict,
        None => IdempotencyHit::None,
    }
}

fn remember_idempotency(tenant: TenantId, key: &str, body_digest: &str, worker_id: WorkerId) {
    let Ok(mut guard) = idem_cache().lock() else {
        return;
    };
    sweep_expired(&mut guard);
    guard.insert(
        (tenant, key.to_string()),
        IdemEntry {
            body_digest: body_digest.to_string(),
            worker_id,
            at: Utc::now(),
        },
    );
}

fn sweep_expired(map: &mut HashMap<(TenantId, String), IdemEntry>) {
    let now = Utc::now();
    map.retain(|_, e| (now - e.at).num_seconds() < IDEMPOTENCY_TTL_SECS);
}

#[cfg(test)]
pub(super) fn reset_idem_cache_for_tests() {
    if let Ok(mut guard) = idem_cache().lock() {
        guard.clear();
    }
}

async fn emit_hire_audit(
    store: &std::sync::Arc<SessionStore>,
    principal_id: &str,
    worker_id: WorkerId,
    problem: &str,
    answers: &Value,
    draft: &HireDraft,
) {
    let payload = json!({
        "endpoint": "workers.hire",
        "worker_id": worker_id.to_string(),
        "vertical_id": draft.suggested_vertical,
        "persona_name": draft.suggested_name,
        "approval_policy_level": draft.approval_policy.level,
        "duty_count": draft.duties.len(),
        "problem": redact_text(problem),
        "answers": redact_value(answers),
        "draft": json!({
            "suggested_name": draft.suggested_name,
            "suggested_vertical": draft.suggested_vertical,
            "charter_len": draft.charter.len(),
            "duty_entry_actions": draft
                .duties
                .iter()
                .map(|d| d.entry_action.clone())
                .collect::<Vec<_>>(),
        }),
    });
    let sink = AuditSink::new(std::sync::Arc::clone(store));
    if let Err(e) = sink
        .write(None, AuditKind::HttpRequest, payload, Some(principal_id.to_string()))
        .await
    {
        tracing::warn!(
            target: "gateway::workers_hire",
            error = %e,
            worker_id = %worker_id,
            "hire audit write failed"
        );
    }
}

/// Lightweight email/phone redactor for the audit-log copy of the
/// hire context. Frontend payloads typically only carry emails;
/// `argo-redact::redact_text` (Stream J2) becomes the authoritative
/// fence once it lands.
fn redact_text(input: &str) -> String {
    let email = redact_pattern(input, r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,24}", "[EMAIL]");
    redact_pattern(&email, r"\b\d{3}[-.\s]\d{3}[-.\s]\d{4}\b", "[PHONE]")
}

fn redact_pattern(input: &str, pattern: &str, sentinel: &str) -> String {
    match regex::Regex::new(pattern) {
        Ok(re) => re.replace_all(input, sentinel).into_owned(),
        Err(_) => input.to_string(),
    }
}

fn redact_value(value: &Value) -> Value {
    match value {
        Value::String(s) => Value::String(redact_text(s)),
        Value::Array(items) => Value::Array(items.iter().map(redact_value).collect()),
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(k, v)| (k.clone(), redact_value(v)))
                .collect(),
        ),
        other => other.clone(),
    }
}

fn ok_idempotent(worker_id: WorkerId, _vertical: &str) -> (&'static str, String) {
    (
        "200 OK",
        json!({
            "worker_id": worker_id.to_string(),
            "success": true,
            "idempotent": true,
        })
        .to_string(),
    )
}

fn bad_request(message: &str) -> (&'static str, String) {
    error_envelope("400 Bad Request", "bad_request", message)
}

fn conflict_response(key: &str) -> (&'static str, String) {
    error_envelope(
        "409 Conflict",
        "idempotency_conflict",
        &format!("Idempotency-Key '{key}' was previously used with a different body"),
    )
}

fn server_error(message: &str) -> (&'static str, String) {
    error_envelope("500 Internal Server Error", "internal_error", message)
}

fn error_envelope(status: &'static str, code: &str, message: &str) -> (&'static str, String) {
    (
        status,
        json!({
            "error": {
                "code": code,
                "message": message,
            }
        })
        .to_string(),
    )
}

#[cfg(test)]
#[path = "routes_workers_hire_tests.rs"]
mod tests;
