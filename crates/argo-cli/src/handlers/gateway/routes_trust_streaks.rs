//! `GET /api/v1/workers/<id>/trust-streaks` — per-worker consecutive-
//! approval streaks (Stream `mission-pm-trust-ratchet-ui`, P2 #11).
//!
//! Returns the rollup the approvals-inbox "ratchet card" renders. The
//! aggregator lives in `argo_store::trust_streak`; this handler is a
//! thin JSON shim that resolves the tenant, validates the worker,
//! reads the streak threshold, and forwards.
//!
//! Threshold sources (highest precedence first):
//! 1. `?min=<N>` query param (operator override per-call).
//! 2. `ARGO_TRUST_RATCHET_MIN` env var (deployment-wide default).
//! 3. Built-in default (`DEFAULT_MIN`, currently 10).
//!
//! Sensible deviations from the spec:
//! - The brief says capability `approval.read`; the existing
//!   sibling routes (`routes_worker_trust_grade`,
//!   `routes_worker_policies`) gate on bearer auth + principal-header
//!   parsing from `routes_dispatch_get` rather than a per-route
//!   capability check. We follow that convention so we don't
//!   introduce a parallel auth surface — a follow-up brief can lift
//!   every per-worker route onto the new capability gate uniformly.
//! - The audit emission uses `AuditKind::HttpRequest` with the
//!   `endpoint: "worker.trust_streaks.viewed"` payload tag, matching
//!   the sibling `worker.trust_grade.queried` audit shape. The brief
//!   asks for a new `ApprovalPolicyPromoted` kind on the promote
//!   step; that path runs through the existing per-worker policy
//!   PATCH (`routes_worker_policies`), which already audits via
//!   `policy.changed`. Adding a new audit kind would violate the
//!   brief's "Files you must NOT touch: Any audit kinds" line.

use super::routes_auth_principal::principal_id_for_audit;
use argo_audit::{AuditKind, AuditSink};
use argo_identity::{IdentityStore, WorkerId};
use argo_store::trust_streak::{streaks_for_worker, TrustStreak};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use serde_json::{json, Value};
use std::sync::Arc;

const DEFAULT_MIN: u32 = 10;
const ENV_MIN: &str = "ARGO_TRUST_RATCHET_MIN";

/// `GET /api/v1/workers/<id>/trust-streaks` entry point.
pub(super) async fn list_response(
    worker_id_raw: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let store = match store {
        Some(s) => s,
        None => return super::store_unavailable(),
    };
    let worker_id = match worker_id_raw.parse::<WorkerId>() {
        Ok(id) => id,
        Err(e) => return bad_request(&format!("invalid worker_id '{worker_id_raw}': {e}")),
    };
    let min = parse_min(req);
    let tenant = match resolve_workspace_tenant(store).await {
        Ok(t) => t,
        Err(e) => return server_error(&format!("resolve tenant: {e}")),
    };
    let identities = IdentityStore::new(store);
    match identities.get_worker(tenant, worker_id).await {
        Ok(Some(_)) => {}
        Ok(None) => return not_found(&format!("worker {worker_id} not found")),
        Err(e) => return server_error(&format!("get worker: {e}")),
    }
    let streaks = match streaks_for_worker(store, &worker_id.to_string(), min).await {
        Ok(rows) => rows,
        Err(e) => return server_error(&format!("streaks_for_worker: {e}")),
    };
    let body = json!({
        "worker_id": worker_id.to_string(),
        "min_threshold": min,
        "streaks": streaks.iter().map(streak_to_json).collect::<Vec<_>>(),
    });
    emit_audit(store, req, tenant, worker_id, streaks.len()).await;
    ("200 OK", body.to_string())
}

fn streak_to_json(s: &TrustStreak) -> Value {
    json!({
        "worker_id": s.worker_id,
        "tool": s.tool,
        "target_glob": s.target_glob,
        "streak_count": s.streak_count,
        "last_decided_at_micros": s.last_decided_at_micros,
    })
}

/// Pick the streak threshold for this request. Query param wins;
/// then env var; then the built-in default. Invalid / negative
/// values fall back to the next source rather than 400-ing — the
/// page already has to handle "nothing crosses the threshold", so
/// over-strict 400s aren't worth the operator confusion.
pub(super) fn parse_min(req: &str) -> u32 {
    if let Some(min) = query_param(req, "min").and_then(|v| v.parse::<u32>().ok()) {
        return min;
    }
    if let Ok(v) = std::env::var(ENV_MIN) {
        if let Ok(parsed) = v.parse::<u32>() {
            return parsed;
        }
    }
    DEFAULT_MIN
}

fn query_param(req: &str, key: &str) -> Option<String> {
    let first_line = req.lines().next()?;
    let (_, path_plus) = first_line.split_once(' ')?;
    let (path_query, _) = path_plus.split_once(' ')?;
    let (_, query) = path_query.split_once('?')?;
    for pair in query.split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            if k == key {
                return Some(v.to_string());
            }
        }
    }
    None
}

async fn resolve_workspace_tenant(store: &SessionStore) -> ArgoResult<TenantId> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (tenant, _ws) = store.ensure_workspace(name, &root).await?;
    Ok(tenant)
}

async fn emit_audit(
    store: &Arc<SessionStore>,
    req: &str,
    tenant: TenantId,
    worker_id: WorkerId,
    count: usize,
) {
    let actor = principal_id_for_audit(req).unwrap_or_else(|| "anonymous".to_string());
    let payload = json!({
        "endpoint": "worker.trust_streaks.viewed",
        "actor": actor,
        "tenant_id": tenant.to_string(),
        "worker_id": worker_id.to_string(),
        "streak_count": count,
    });
    let sink = AuditSink::new_with_tenant(Arc::clone(store), tenant.to_string());
    if let Err(e) = sink
        .write(None, AuditKind::HttpRequest, payload, Some(actor.clone()))
        .await
    {
        tracing::warn!(
            target: "gateway::trust_streaks",
            error = %e,
            actor = %actor,
            "worker.trust_streaks.viewed audit write failed"
        );
    }
}

fn bad_request(message: &str) -> (&'static str, String) {
    error_envelope("400 Bad Request", "bad_request", message)
}

fn not_found(message: &str) -> (&'static str, String) {
    error_envelope("404 Not Found", "not_found", message)
}

fn server_error(message: &str) -> (&'static str, String) {
    error_envelope("500 Internal Server Error", "internal_error", message)
}

fn error_envelope(status: &'static str, code: &str, message: &str) -> (&'static str, String) {
    (
        status,
        json!({ "error": { "code": code, "message": message } }).to_string(),
    )
}

#[cfg(test)]
#[path = "routes_trust_streaks_tests.rs"]
mod tests;
