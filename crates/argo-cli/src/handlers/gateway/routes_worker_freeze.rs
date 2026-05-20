//! `POST /api/v1/workers/<id>/freeze` and `/unfreeze` — gateway HTTP
//! surface for the PM kill switch (mission-pm-kill-switch-4-gateway).
//!
//! Both routes:
//! 1. Parse the path-segment worker id (UUID-validated).
//! 2. Resolve the workspace tenant + load the worker (404 / 403 if
//!    unknown or cross-tenant).
//! 3. Check `Capability::WorkerFreeze` against the requesting
//!    principal's role (`X-Argo-Role` header, default `user`). On
//!    deny: emit an [`AuditKind::RbacDeny`] row and return 403.
//! 4. `freeze` validates the JSON body's `reason` field (non-empty,
//!    required); `unfreeze` reads no body.
//! 5. Call the [`argo_store::worker_freeze`] primitive.
//! 6. Emit an [`AuditKind::HttpRequest`] success row and return JSON.
//!
//! Success response (freeze): `{ worker_id, frozen_at, frozen_by,
//! reason }` — mirrors [`argo_store::worker_freeze::FreezeRecord`].
//!
//! Success response (unfreeze): `{ worker_id, unfrozen, unfrozen_by }`.
//!
//! ## Sensible deviations from the brief
//!
//! - The brief notes mention "the existing route pattern already emits
//!   RbacDeny / RbacAllow audit rows on the capability check" — in
//!   practice no gateway route did capability gating before this brief.
//!   Only [`AuditKind::RbacDeny`] exists in `argo-audit`, not an Allow
//!   counterpart. This handler emits `RbacDeny` on failure and the
//!   ordinary `HttpRequest` audit kind on success.
//! - The brief named `routes_dispatch_post.rs` as the mount point, but
//!   the worker-state-toggle dispatch lives in
//!   `routes_dispatch_post_paths.rs::dispatch_worker_lifecycle` (the
//!   pause/resume/archive arms). The two new arms go alongside those
//!   for pattern consistency.

use super::role_gate::{enforce_role, resolve_role, Action};
use super::routes_auth_principal::principal_id_for_audit;
use super::{body_from_request, store_unavailable};
use argo_audit::{AuditKind, AuditSink};
use argo_identity::user_tenant::UserTenantRole;
use argo_identity::{IdentityStore, Worker, WorkerId};
use argo_store::worker_freeze::{self as freeze_store, FreezeRecord};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use serde_json::{json, Value};
use std::sync::Arc;

/// `POST /api/v1/workers/<id>/freeze`. Body shape: `{ "reason": <str> }`.
pub(super) async fn post_freeze(
    worker_id_raw: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let (tenant, worker) = match preflight(worker_id_raw, store).await {
        Ok(p) => p,
        Err(resp) => return resp,
    };
    if let Err(resp) = enforce_worker_freeze(store, &worker, req).await {
        return resp;
    }
    let reason = match parse_reason(req) {
        Ok(r) => r,
        Err(resp) => return resp,
    };
    let actor = actor_label(req);
    let record = match freeze_store::freeze(
        store.pool(),
        &tenant.to_string(),
        &worker.id.to_string(),
        &actor,
        &reason,
    )
    .await
    {
        Ok(r) => r,
        Err(e) => return server_error(&format!("freeze: {e}")),
    };
    emit_success(store, req, &worker, "worker.frozen", &record).await;
    ("200 OK", freeze_response(&record))
}

/// `POST /api/v1/workers/<id>/unfreeze`. No body expected.
pub(super) async fn post_unfreeze(
    worker_id_raw: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let (tenant, worker) = match preflight(worker_id_raw, store).await {
        Ok(p) => p,
        Err(resp) => return resp,
    };
    if let Err(resp) = enforce_worker_freeze(store, &worker, req).await {
        return resp;
    }
    let actor = actor_label(req);
    let changed = match freeze_store::unfreeze(
        store.pool(),
        &tenant.to_string(),
        &worker.id.to_string(),
        &actor,
    )
    .await
    {
        Ok(c) => c,
        Err(e) => return server_error(&format!("unfreeze: {e}")),
    };
    emit_unfreeze(store, req, &worker, changed).await;
    let body = json!({
        "worker_id": worker.id.to_string(),
        "unfrozen": changed,
        "unfrozen_by": actor,
    });
    ("200 OK", body.to_string())
}

async fn preflight(
    worker_id_raw: &str,
    store: &Arc<SessionStore>,
) -> Result<(TenantId, Worker), (&'static str, String)> {
    let worker_id = parse_worker_id(worker_id_raw)?;
    let tenant = resolve_tenant(store)
        .await
        .map_err(|e| server_error(&format!("resolve tenant: {e}")))?;
    let worker = load_worker(store, tenant, worker_id).await?;
    Ok((tenant, worker))
}

fn parse_worker_id(raw: &str) -> Result<WorkerId, (&'static str, String)> {
    let cleaned = raw.split('?').next().unwrap_or(raw);
    cleaned
        .parse::<WorkerId>()
        .map_err(|e| bad_request(&format!("invalid worker_id '{cleaned}': {e}")))
}

async fn resolve_tenant(store: &SessionStore) -> ArgoResult<TenantId> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (tenant, _ws) = store.ensure_workspace(name, &root).await?;
    Ok(tenant)
}

async fn load_worker(
    store: &Arc<SessionStore>,
    tenant: TenantId,
    worker_id: WorkerId,
) -> Result<Worker, (&'static str, String)> {
    match IdentityStore::new(store).get_worker(tenant, worker_id).await {
        Ok(Some(w)) => Ok(w),
        Ok(None) => Err(not_found(&format!("worker {worker_id} not found"))),
        Err(e) => Err(server_error(&format!("get worker: {e}"))),
    }
}

async fn enforce_worker_freeze(
    store: &Arc<SessionStore>,
    worker: &Worker,
    req: &str,
) -> Result<(), (&'static str, String)> {
    if let Some(resp) = enforce_role(req, Action::WorkerFreeze) {
        let role_label = resolve_role(req)
            .unwrap_or(UserTenantRole::Guest)
            .as_str();
        emit_rbac_deny(store, req, worker, role_label).await;
        return Err(resp);
    }
    Ok(())
}

fn parse_reason(req: &str) -> Result<String, (&'static str, String)> {
    let body = body_from_request(req);
    let body_trim = body.trim();
    if body_trim.is_empty() {
        return Err(bad_request("missing required field 'reason'"));
    }
    let parsed: Value = serde_json::from_str(body_trim)
        .map_err(|e| bad_request(&format!("invalid JSON body: {e}")))?;
    let raw = parsed
        .get("reason")
        .and_then(Value::as_str)
        .ok_or_else(|| bad_request("missing required field 'reason'"))?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(bad_request("'reason' must not be empty"));
    }
    Ok(trimmed.to_string())
}

fn actor_label(req: &str) -> String {
    principal_id_for_audit(req).unwrap_or_else(|| "anonymous".to_string())
}

async fn emit_success(
    store: &Arc<SessionStore>,
    req: &str,
    worker: &Worker,
    endpoint: &str,
    record: &FreezeRecord,
) {
    let payload = json!({
        "endpoint": endpoint,
        "worker_id": worker.id.to_string(),
        "frozen_at": record.frozen_at,
        "frozen_by": record.frozen_by,
        "reason": record.reason,
    });
    write_audit(store, req, worker, AuditKind::HttpRequest, payload).await;
}

async fn emit_unfreeze(store: &Arc<SessionStore>, req: &str, worker: &Worker, changed: bool) {
    let payload = json!({
        "endpoint": "worker.unfrozen",
        "worker_id": worker.id.to_string(),
        "unfrozen": changed,
    });
    write_audit(store, req, worker, AuditKind::HttpRequest, payload).await;
}

async fn emit_rbac_deny(store: &Arc<SessionStore>, req: &str, worker: &Worker, role: &str) {
    let payload = json!({
        "endpoint": "worker.freeze",
        "worker_id": worker.id.to_string(),
        "role": role,
        "capability": "worker.freeze",
    });
    write_audit(store, req, worker, AuditKind::RbacDeny, payload).await;
}

async fn write_audit(
    store: &Arc<SessionStore>,
    req: &str,
    worker: &Worker,
    kind: AuditKind,
    payload: Value,
) {
    let sink = AuditSink::new_with_tenant(Arc::clone(store), worker.tenant_id.to_string());
    if let Err(e) = sink
        .write(None, kind, payload, principal_id_for_audit(req))
        .await
    {
        tracing::warn!(
            target: "gateway::worker_freeze",
            error = %e,
            "worker.freeze audit write failed"
        );
    }
}

fn freeze_response(record: &FreezeRecord) -> String {
    json!({
        "worker_id": record.worker_id,
        "frozen_at": record.frozen_at,
        "frozen_by": record.frozen_by,
        "reason": record.reason,
    })
    .to_string()
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
#[path = "routes_worker_freeze_tests.rs"]
mod tests;
