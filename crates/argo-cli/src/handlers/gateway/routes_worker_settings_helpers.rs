//! Shared helpers for the J11 worker-settings handlers.
//!
//! Routing, lookup, archive-gate, role-parsing, audit emission, and the
//! error-envelope formatters live here so the read-path and danger-zone
//! handler modules stay under the 500-LOC ceiling individually.

use super::super::routes_auth_principal::principal_id_for_audit;
use super::super::routes_worker_detail::PrincipalRole;
use super::super::{body_from_request, store_unavailable};
use argo_audit::{AuditKind, AuditSink};
use argo_identity::{IdentityStore, Worker, WorkerId};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use chrono::{DateTime, Duration, Utc};
use serde_json::{json, Value};
use std::sync::Arc;

pub(super) const RECOVERABLE_DAYS: i64 = 30;
pub(super) const ROLE_HEADER: &str = "X-Argo-Role";
pub(super) const CONFIRM_HEADER: &str = "X-Confirm-Worker-Name";

pub(super) fn store_unavail() -> (&'static str, String) {
    store_unavailable()
}

pub(super) async fn preflight(
    worker_id_raw: &str,
    store: &Arc<SessionStore>,
    check_archived: bool,
) -> Result<(TenantId, Worker), (&'static str, String)> {
    let worker_id = parse_worker_id(worker_id_raw)?;
    let tenant = resolve_tenant(store)
        .await
        .map_err(|e| server_error(&format!("resolve tenant: {e}")))?;
    let worker = load_worker_by_id(store, tenant, worker_id).await?;
    if check_archived {
        if let Some(resp) = enforce_archive_410(store, tenant, worker_id).await {
            return Err(resp);
        }
    }
    Ok((tenant, worker))
}

pub(super) fn parse_worker_id(raw: &str) -> Result<WorkerId, (&'static str, String)> {
    let cleaned = raw.split('?').next().unwrap_or(raw);
    cleaned
        .parse::<WorkerId>()
        .map_err(|e| bad_request(&format!("invalid worker_id '{cleaned}': {e}")))
}

pub(super) async fn load_worker_by_id(
    store: &Arc<SessionStore>,
    tenant: TenantId,
    worker_id: WorkerId,
) -> Result<Worker, (&'static str, String)> {
    let identities = IdentityStore::new(store);
    match identities.get_worker(tenant, worker_id).await {
        Ok(Some(w)) => Ok(w),
        Ok(None) => match exists_in_other_tenant(store, tenant, worker_id).await {
            Ok(true) => Err(forbidden(&format!("worker {worker_id} not in this tenant"))),
            Ok(false) => Err(not_found(&format!("worker {worker_id} not found"))),
            Err(e) => Err(server_error(&format!("cross-tenant probe: {e}"))),
        },
        Err(e) => Err(server_error(&format!("get worker: {e}"))),
    }
}

pub(super) async fn enforce_archive_410(
    store: &Arc<SessionStore>,
    tenant: TenantId,
    worker_id: WorkerId,
) -> Option<(&'static str, String)> {
    match fetch_archived_at(store, tenant, worker_id).await {
        Ok(Some(ts)) => Some(gone_response(worker_id, &ts)),
        Ok(None) => None,
        Err(e) => Some(server_error(&format!("archived lookup: {e}"))),
    }
}

pub(super) fn parse_body(req: &str) -> Result<Value, (&'static str, String)> {
    let body = body_from_request(req);
    if body.trim().is_empty() {
        return Ok(json!({}));
    }
    serde_json::from_str::<Value>(body).map_err(|e| bad_request(&format!("invalid JSON body: {e}")))
}

pub(super) fn require_string<'a>(body: &'a Value, key: &str) -> Option<&'a str> {
    body.get(key).and_then(Value::as_str).filter(|s| !s.is_empty())
}

pub(super) fn parse_role_header(req: &str) -> Option<PrincipalRole> {
    let raw = header_value(req, ROLE_HEADER)?;
    match raw.trim().to_ascii_lowercase().as_str() {
        "delegator" => Some(PrincipalRole::Delegator),
        "operator" => Some(PrincipalRole::Operator),
        "auditor" => Some(PrincipalRole::Auditor),
        "compliance-officer" | "compliance_officer" => Some(PrincipalRole::ComplianceOfficer),
        _ => None,
    }
}

pub(super) fn header_value(req: &str, name: &str) -> Option<String> {
    let headers = req
        .split_once("\r\n\r\n")
        .map_or(req, |(headers, _body)| headers);
    headers.lines().skip(1).find_map(|line| {
        let (k, v) = line.split_once(':')?;
        k.eq_ignore_ascii_case(name).then(|| v.trim().to_string())
    })
}

pub(super) async fn emit_audit(
    store: &Arc<SessionStore>,
    req: &str,
    worker: &Worker,
    endpoint: &str,
    extras: Value,
) {
    let mut payload = json!({
        "endpoint": endpoint,
        "worker_id": worker.id.to_string(),
        "worker_name": worker.persona.name,
    });
    if let Some(obj) = extras.as_object() {
        if let Some(into) = payload.as_object_mut() {
            for (k, v) in obj {
                into.insert(k.clone(), v.clone());
            }
        }
    }
    let sink = AuditSink::new(Arc::clone(store));
    if let Err(e) = sink
        .write(None, AuditKind::HttpRequest, payload, principal_id_for_audit(req))
        .await
    {
        tracing::warn!(
            target: "gateway::worker_settings",
            error = %e,
            endpoint = %endpoint,
            "audit write failed"
        );
    }
}

pub(super) async fn resolve_tenant(store: &SessionStore) -> ArgoResult<TenantId> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let name = cwd.file_name().and_then(|s| s.to_str()).unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (tenant, _ws) = store.ensure_workspace(name, &root).await?;
    Ok(tenant)
}

async fn exists_in_other_tenant(
    store: &SessionStore,
    tenant: TenantId,
    worker_id: WorkerId,
) -> ArgoResult<bool> {
    let row: Option<String> =
        sqlx::query_scalar("SELECT id FROM worker WHERE id = ? AND tenant_id != ? LIMIT 1")
            .bind(worker_id.to_string())
            .bind(tenant.to_string())
            .fetch_optional(store.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("cross-tenant probe: {e}")))?;
    Ok(row.is_some())
}

pub(super) async fn fetch_archived_at(
    store: &SessionStore,
    tenant: TenantId,
    worker_id: WorkerId,
) -> ArgoResult<Option<DateTime<Utc>>> {
    let raw: Option<String> = sqlx::query_scalar(
        "SELECT archived_at FROM worker \
         WHERE id = ? AND tenant_id = ? AND archived_at IS NOT NULL",
    )
    .bind(worker_id.to_string())
    .bind(tenant.to_string())
    .fetch_optional(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("archived_at lookup: {e}")))?;
    Ok(raw.and_then(|s| {
        DateTime::parse_from_rfc3339(&s)
            .ok()
            .map(|dt| dt.with_timezone(&Utc))
    }))
}

pub(super) fn gone_response(
    worker_id: WorkerId,
    archived_at: &DateTime<Utc>,
) -> (&'static str, String) {
    let recoverable = *archived_at + Duration::days(RECOVERABLE_DAYS);
    let body = json!({
        "error": { "code": "gone", "message": format!("worker {worker_id} is archived") },
        "archived_at": archived_at.to_rfc3339(),
        "recoverable_until_at": recoverable.to_rfc3339(),
        "retry_after_seconds": (recoverable - Utc::now()).num_seconds().max(0),
    });
    ("410 Gone", body.to_string())
}

pub(super) fn bad_request(message: &str) -> (&'static str, String) {
    error_envelope("400 Bad Request", "bad_request", message)
}

pub(super) fn not_found(message: &str) -> (&'static str, String) {
    error_envelope("404 Not Found", "not_found", message)
}

pub(super) fn forbidden(message: &str) -> (&'static str, String) {
    error_envelope("403 Forbidden", "forbidden", message)
}

pub(super) fn server_error(message: &str) -> (&'static str, String) {
    error_envelope("500 Internal Server Error", "internal_error", message)
}

pub(super) fn error_envelope(
    status: &'static str,
    code: &str,
    message: &str,
) -> (&'static str, String) {
    (
        status,
        json!({ "error": { "code": code, "message": message } }).to_string(),
    )
}
