//! `GET /api/v1/cohorts` + `GET /api/v1/cohorts/<id>` — fleet-wide
//! skill-cohort read surface (Stream `mission-skill-cohort-ui`,
//! MISSION_BACKLOG #198).
//!
//! Workspace-tenant-scoped. Bearer auth is gated upstream by
//! `routes::dispatch_route`. Cross-tenant `?tenant_id=` query honors
//! the same 403 rule as `routes_team_list`.
//!
//! ## Surface
//!
//! - `GET /api/v1/cohorts` →
//!   `{ "cohorts": [CohortSummary], "total_count": usize }`
//! - `GET /api/v1/cohorts/<cohort_id>` → flattened
//!   `CohortSummary` + `{ "members": [...], "skills": [...] }`
//!
//! Cohort identity is `"<vertical>::<trust_band>"`. See
//! `argo_store::cohort` for band definitions.
//!
//! ## Sensible deviations
//!
//! - The brief asks for `[{...}]` at the list level; we wrap in
//!   `{ "cohorts": [...], "total_count": n }` to match the existing
//!   `routes_team_list` convention (gateway response writers are
//!   fixed-shape).
//! - Detail flattens `CohortSummary` into the top-level JSON object
//!   per the brief's `{ ...summary, members, skills }` shape.

use super::routes_auth_principal::principal_id_for_audit;
use argo_audit::{AuditKind, AuditSink};
use argo_store::cohort::{cohort_detail, list_cohorts, CohortSummary};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

/// `GET /api/v1/cohorts`.
pub(super) async fn list_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let store = match store {
        Some(s) => s,
        None => return super::store_unavailable(),
    };
    let workspace_tenant = match resolve_workspace_tenant(store).await {
        Ok(t) => t,
        Err(e) => return server_error(&format!("resolve tenant: {e}")),
    };
    match parse_tenant_query(req) {
        Ok(Some(req_t)) if req_t != workspace_tenant => {
            return forbidden("cross-tenant query not permitted");
        }
        Ok(_) => {}
        Err(msg) => return bad_request(&msg),
    }

    let cohorts: Vec<CohortSummary> =
        match list_cohorts(store.as_ref(), workspace_tenant).await {
            Ok(v) => v,
            Err(e) => return server_error(&format!("list cohorts: {e}")),
        };
    let total = cohorts.len();
    emit_list_audit(store, req, total, workspace_tenant).await;
    let body = json!({ "cohorts": cohorts, "total_count": total }).to_string();
    ("200 OK", body)
}

/// `GET /api/v1/cohorts/<cohort_id>`.
pub(super) async fn detail_response(
    req: &str,
    raw_cohort_id: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let store = match store {
        Some(s) => s,
        None => return super::store_unavailable(),
    };
    let workspace_tenant = match resolve_workspace_tenant(store).await {
        Ok(t) => t,
        Err(e) => return server_error(&format!("resolve tenant: {e}")),
    };
    match parse_tenant_query(req) {
        Ok(Some(req_t)) if req_t != workspace_tenant => {
            return forbidden("cross-tenant query not permitted");
        }
        Ok(_) => {}
        Err(msg) => return bad_request(&msg),
    }
    let decoded = decode_path_segment(raw_cohort_id);
    if decoded.is_empty() {
        return bad_request("cohort_id required");
    }
    let detail = match cohort_detail(store.as_ref(), workspace_tenant, &decoded).await {
        Ok(Some(d)) => d,
        Ok(None) => return not_found(&format!("cohort '{decoded}' not found")),
        Err(e) => return server_error(&format!("cohort detail: {e}")),
    };
    emit_detail_audit(store, req, &decoded, workspace_tenant).await;
    let body = serde_json::to_string(&detail).unwrap_or_else(|_| "{}".to_string());
    ("200 OK", body)
}

async fn resolve_workspace_tenant(store: &SessionStore) -> ArgoResult<TenantId> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let name = cwd.file_name().and_then(|s| s.to_str()).unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (tenant, _ws) = store.ensure_workspace(name, &root).await?;
    Ok(tenant)
}

fn parse_tenant_query(req: &str) -> Result<Option<TenantId>, String> {
    let tail = req.strip_prefix("GET ").unwrap_or(req);
    let head = tail.split_once(' ').map_or(tail, |(p, _)| p);
    let Some(qs) = head.split_once('?').map(|(_, q)| q) else {
        return Ok(None);
    };
    let raw = qs.split('&').find_map(|kv| {
        let (k, v) = kv.split_once('=')?;
        (k == "tenant_id").then(|| v.to_string())
    });
    let Some(raw) = raw else { return Ok(None) };
    Uuid::parse_str(&raw)
        .map(|u| Some(TenantId(u)))
        .map_err(|e| format!("invalid tenant_id '{raw}': {e}"))
}

/// Strip the trailing ` HTTP/1.1` and any `?query` from the path
/// segment, then percent-decode `%3A`-style `:` separators if a client
/// has encoded them. v1 implementation is permissive — the cohort id
/// is opaque to the route layer; the store layer validates the format.
fn decode_path_segment(raw: &str) -> String {
    let trimmed = raw
        .split_whitespace()
        .next()
        .unwrap_or(raw)
        .split('?')
        .next()
        .unwrap_or(raw);
    let mut out = String::with_capacity(trimmed.len());
    let bytes = trimmed.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(s) = std::str::from_utf8(&bytes[i + 1..i + 3]) {
                if let Ok(b) = u8::from_str_radix(s, 16) {
                    out.push(b as char);
                    i += 3;
                    continue;
                }
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

async fn emit_list_audit(
    store: &Arc<SessionStore>,
    req: &str,
    count: usize,
    tenant: TenantId,
) {
    let actor = principal_id_for_audit(req).unwrap_or_else(|| "anonymous".to_string());
    let payload = json!({
        "endpoint": "cohorts.list.queried",
        "actor": actor,
        "tenant_id": tenant.to_string(),
        "count": count,
    });
    let sink = AuditSink::new(Arc::clone(store));
    if let Err(e) = sink
        .write(None, AuditKind::HttpRequest, payload, Some(actor.clone()))
        .await
    {
        tracing::warn!(
            target: "gateway::cohorts",
            error = %e,
            actor = %actor,
            "cohorts.list.queried audit write failed"
        );
    }
}

async fn emit_detail_audit(
    store: &Arc<SessionStore>,
    req: &str,
    cohort_id: &str,
    tenant: TenantId,
) {
    let actor = principal_id_for_audit(req).unwrap_or_else(|| "anonymous".to_string());
    let payload = json!({
        "endpoint": "cohorts.detail.queried",
        "actor": actor,
        "tenant_id": tenant.to_string(),
        "cohort_id": cohort_id,
    });
    let sink = AuditSink::new(Arc::clone(store));
    if let Err(e) = sink
        .write(None, AuditKind::HttpRequest, payload, Some(actor.clone()))
        .await
    {
        tracing::warn!(
            target: "gateway::cohorts",
            error = %e,
            actor = %actor,
            "cohorts.detail.queried audit write failed"
        );
    }
}

fn bad_request(message: &str) -> (&'static str, String) {
    error_envelope("400 Bad Request", "bad_request", message)
}

fn forbidden(message: &str) -> (&'static str, String) {
    error_envelope("403 Forbidden", "forbidden", message)
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
#[path = "routes_cohorts_tests.rs"]
mod tests;
