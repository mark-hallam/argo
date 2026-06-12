//! `GET /api/v1/workers/<id>/capability-requests` — the pending
//! capability requests a teammate accrued when its capability fence
//! denied a tool/connector (brief
//! `tg1-emit-capability-request-on-fenced-tool-deny`).
//!
//! ## What this surfaces
//!
//! When a worker's run hits a tool/connector its fence denies, the
//! runtime persists a pending `capability_request` row (one per
//! `(tenant, worker, tool)`). This route is what the SPA's
//! connect-on-demand grant card (`option-G2-elevated`) polls to learn
//! "this teammate needed <connector>" so the user can grant it. Reads
//! only — granting (advancing `status`) is a later slice (tg3).
//!
//! ## Tenant scoping (the security crux)
//!
//! Identical to `routes_worker_outputs`:
//!
//! 1. Resolve the JWT tenant ([`resolve_tenant_for_request`]).
//! 2. Load the worker by `(tenant, worker_id)` — a cross-tenant or
//!    unknown worker surfaces a uniform `404` (existence is not leaked).
//! 3. List the worker's pending requests, scoped to the worker's own
//!    `(tenant_id, worker_id)` — the store filter is tenant-first, so a
//!    `worker_id` collision across tenants can never surface another
//!    tenant's requests.

use super::routes_auth_principal::principal_id_for_audit;
use argo_audit::{AuditKind, AuditSink};
use argo_identity::{IdentityStore, Worker, WorkerId};
use argo_store::SessionStore;
use argo_types::TenantId;
use serde_json::{json, Value};
use std::sync::Arc;

/// `GET /api/v1/workers/<id>/capability-requests` entry point.
/// `worker_id_raw` is the already-split path segment.
pub(super) async fn list_response(
    worker_id_raw: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    let worker_id = match parse_worker_id(worker_id_raw) {
        Ok(id) => id,
        Err(r) => return r,
    };
    let (tenant, worker) = match resolve_worker(store, req, worker_id).await {
        Ok(w) => w,
        Err(r) => return r,
    };
    let rows = match argo_store::capability_request::list_pending(
        store.pool(),
        &worker.tenant_id.to_string(),
        &worker.id.to_string(),
    )
    .await
    {
        Ok(rows) => rows,
        Err(e) => return server_error(&format!("list capability requests: {e}")),
    };
    let body = build_json(&worker, &rows);
    emit_view_audit(store, req, tenant, worker_id).await;
    ("200 OK", body.to_string())
}

// ----- id parsing ----------------------------------------------------------

fn parse_worker_id(raw: &str) -> Result<WorkerId, (&'static str, String)> {
    let t = raw.split('?').next().unwrap_or(raw);
    t.parse::<WorkerId>()
        .map_err(|e| bad_request(&format!("invalid worker_id '{t}': {e}")))
}

// ----- tenant-scoped worker resolution -------------------------------------

/// Resolve the JWT tenant + load the worker. Cross-tenant / unknown /
/// archived all surface as a uniform `404` — existence is never leaked,
/// matching the worker-detail + worker-output routes.
async fn resolve_worker(
    store: &Arc<SessionStore>,
    req: &str,
    worker_id: WorkerId,
) -> Result<(TenantId, Worker), (&'static str, String)> {
    let tenant = super::resolve_tenant::resolve_tenant_for_request(req, store)
        .await
        .map_err(|e| server_error(&format!("resolve tenant: {e}")))?;
    match IdentityStore::new(store).get_worker(tenant, worker_id).await {
        Ok(Some(worker)) => Ok((tenant, worker)),
        Ok(None) => Err(not_found("worker not found")),
        Err(e) => Err(server_error(&format!("get worker: {e}"))),
    }
}

// ----- response shaping ----------------------------------------------------

fn build_json(worker: &Worker, rows: &[argo_store::capability_request::CapabilityRequest]) -> Value {
    let requests: Vec<Value> = rows
        .iter()
        .map(|r| {
            json!({
                "id": r.id,
                "tool_or_connector": r.tool_or_connector,
                "requested_access": r.requested_access,
                "run_id": r.run_id,
                "status": r.status,
                "created_utc": r.created_utc,
                "last_seen_utc": r.last_seen_utc,
            })
        })
        .collect();
    json!({
        "worker_id": worker.id.to_string(),
        "worker_name": worker.persona.name,
        "capability_requests": requests,
    })
}

// ----- audit ---------------------------------------------------------------

async fn emit_view_audit(
    store: &Arc<SessionStore>,
    req: &str,
    tenant: TenantId,
    worker_id: WorkerId,
) {
    let payload = json!({
        "endpoint": "worker.capability_requests.viewed",
        "worker_id": worker_id.to_string(),
    });
    let sink = AuditSink::new_with_tenant(Arc::clone(store), tenant.to_string());
    if let Err(e) = sink
        .write(None, AuditKind::HttpRequest, payload, principal_id_for_audit(req))
        .await
    {
        tracing::warn!(
            target: "gateway::worker_capability_requests",
            error = %e,
            "worker.capability_requests.viewed audit write failed"
        );
    }
}

// ----- error envelopes -----------------------------------------------------

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
#[path = "routes_worker_capability_requests_tests.rs"]
mod tests;
