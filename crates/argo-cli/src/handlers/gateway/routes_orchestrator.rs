//! Tenant-scoped orchestrator session + handoff gateway routes.
//!
//! Handler implementations live in sibling modules so this file stays under
//! the 500-LOC budget. The dispatcher in [`dispatch_route`] still owns auth,
//! schema-ensure, and method/path matching.
//!
//! - Session lifecycle handlers: [`super::routes_orchestrator_sessions`].
//! - Handoff sub-resource handlers: [`super::routes_orchestrator_handoffs`].
//! - SLA sub-resource handlers: [`super::routes_orchestrator_slas`].

use argo_orchestrator::OrchestratorStore;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use chrono::{TimeZone, Utc};
use serde_json::{json, Value};
use std::sync::Arc;

pub(super) use super::routes_orchestrator_handoffs::{
    handoff_json, list_handoffs, patch_handoff, show_handoff,
};
pub(super) use super::routes_orchestrator_sessions::{
    create_session, list_sessions, patch_session, show_session,
};

pub(super) async fn dispatch_route(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    let (method, target) = request_line(req)?;
    let path = target.split('?').next().unwrap_or(target);
    if !is_orchestrator_path(method, path, req) {
        return None;
    }
    let Some(store) = store else {
        return Some(super::store_unavailable());
    };
    Some(match route(method, target, req, store).await {
        Ok(resp) => resp,
        Err(resp) => resp,
    })
}

async fn route(
    method: &str,
    target: &str,
    req: &str,
    store: &Arc<SessionStore>,
) -> Result<(&'static str, String), (&'static str, String)> {
    let body = super::body_from_request(req);
    let body_json = parse_body(body)?;
    let tenant = tenant_from_req(target, body_json.as_ref())?;
    auth(req, tenant)?;
    let repo = OrchestratorStore::new(Arc::clone(store));
    repo.ensure_schema().await.map_err(server)?;
    let path = target.split('?').next().unwrap_or(target);
    match (method, path) {
        ("GET", "/sessions") => list_sessions(store, target, tenant).await,
        ("POST", "/sessions") => create_session(&repo, tenant, body_json.as_ref()).await,
        ("GET", p) if p.starts_with("/sessions/") => show_session(&repo, tenant, p).await,
        ("PATCH", p) if p.starts_with("/sessions/") && p.ends_with("/status") => {
            patch_session(&repo, tenant, p, body_json.as_ref()).await
        }
        _ if super::routes_orchestrator_slas::is_sla_path(method, path) => {
            super::routes_orchestrator_slas::dispatch(
                method,
                path,
                store,
                tenant,
                body_json.as_ref(),
            )
            .await
        }
        ("GET", "/handoffs") => list_handoffs(store, target, tenant).await,
        ("GET", p) if p.starts_with("/handoffs/") => show_handoff(&repo, tenant, p).await,
        ("PATCH", p) if p.starts_with("/handoffs/") && p.ends_with("/status") => {
            patch_handoff(&repo, tenant, p, body_json.as_ref()).await
        }
        _ => Err(not_found()),
    }
}

fn is_orchestrator_path(method: &str, path: &str, req: &str) -> bool {
    path.starts_with("/handoffs/")
        || path == "/handoffs"
        || super::routes_orchestrator_slas::is_sla_path(method, path)
        || method == "PATCH" && path.starts_with("/sessions/")
        || path.starts_with("/sessions/") && (target_has_tenant(req) || default_tenant().is_some())
        || path == "/sessions"
            && (target_has_tenant(req) || body_has_tenant(req) || default_tenant().is_some())
}

fn auth(req: &str, tenant: TenantId) -> Result<(), (&'static str, String)> {
    let token = header(req, "Authorization")
        .and_then(|h| h.strip_prefix("Bearer ").map(str::to_string))
        .or_else(|| cookie(req, "argo_operator_token"))
        .ok_or_else(|| unauthorized("missing bearer token"))?;
    // tenant-unscoped: gateway process-level secret (no per-request tenant context for this slot)
    if let Ok(expected) = argo_config::cred::get_unscoped("ARGO_WEBINBOX_BEARER_TOKEN") {
        if !expected.is_empty() && token != expected {
            return Err(unauthorized("invalid bearer token"));
        }
    }
    if let Some(expected) = default_tenant() {
        if tenant.to_string() != expected {
            return Err(forbidden("tenant mismatch"));
        }
    }
    Ok(())
}

fn tenant_from_req(target: &str, body: Option<&Value>) -> Result<TenantId, (&'static str, String)> {
    let raw = query(target, &["tenant", "status", "limit", "session", "worker"])?
        .get("tenant")
        .cloned()
        .or_else(|| {
            body.and_then(|v| v.get("tenant_id"))
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .or_else(default_tenant)
        .ok_or_else(|| bad("missing tenant"))?;
    uuid::Uuid::parse_str(&raw)
        .map(TenantId)
        .map_err(|e| bad_owned(format!("invalid tenant '{raw}': {e}")))
}

fn request_line(req: &str) -> Option<(&str, &str)> {
    let mut parts = req.lines().next()?.split_whitespace();
    Some((parts.next()?, parts.next()?))
}

pub(super) fn query(
    target: &str,
    allowed: &[&str],
) -> Result<std::collections::HashMap<String, String>, (&'static str, String)> {
    let Some((_, raw)) = target.split_once('?') else {
        return Ok(Default::default());
    };
    let mut out = std::collections::HashMap::new();
    for kv in raw.split('&').filter(|s| !s.is_empty()) {
        let (k, v) = kv.split_once('=').unwrap_or((kv, ""));
        if !allowed.contains(&k) {
            return Err(bad_owned(format!("unknown query parameter '{k}'")));
        }
        out.insert(k.to_string(), v.to_string());
    }
    Ok(out)
}

fn parse_body(raw: &str) -> Result<Option<Value>, (&'static str, String)> {
    if raw.trim().is_empty() {
        return Ok(None);
    }
    serde_json::from_str(raw)
        .map(Some)
        .map_err(|e| bad_owned(format!("parse JSON body: {e}")))
}

pub(super) fn path_id<'a>(path: &'a str, prefix: &str) -> Result<&'a str, (&'static str, String)> {
    path.strip_prefix(prefix)
        .filter(|id| !id.is_empty() && !id.contains('/'))
        .ok_or_else(not_found)
}

pub(super) fn uuid_blob(bytes: &[u8]) -> ArgoResult<String> {
    uuid::Uuid::from_slice(bytes)
        .map(|u| u.to_string())
        .map_err(|e| ArgoError::Invariant(format!("uuid blob: {e}")))
}

pub(super) fn micros(v: i64) -> ArgoResult<String> {
    Utc.timestamp_micros(v)
        .single()
        .map(|d| d.to_rfc3339())
        .ok_or_else(|| ArgoError::Provider("timestamp out of range".into()))
}

pub(super) fn opt_micros(v: Option<i64>) -> ArgoResult<Option<String>> {
    v.map(micros).transpose()
}

fn header<'a>(req: &'a str, key: &str) -> Option<&'a str> {
    req.lines().find_map(|l| {
        l.split_once(':')
            .and_then(|(k, v)| k.eq_ignore_ascii_case(key).then(|| v.trim()))
    })
}

fn cookie(req: &str, key: &str) -> Option<String> {
    header(req, "Cookie")?.split(';').find_map(|part| {
        let (k, v) = part.trim().split_once('=')?;
        (k == key).then(|| v.to_string())
    })
}

fn default_tenant() -> Option<String> {
    let cfg = argo_config::load().unwrap_or_default();
    cfg.channels
        .webinbox
        .tenant_id
        .filter(|s| !s.is_empty())
        .or_else(|| {
            (!cfg.tenant.default_id.is_empty()
                && cfg.tenant.default_id != argo_config::defaults::DEFAULT_TENANT_ID)
                .then_some(cfg.tenant.default_id)
        })
}

fn target_has_tenant(req: &str) -> bool {
    request_line(req)
        .and_then(|(_, target)| target.split_once('?').map(|(_, raw)| raw))
        .is_some_and(|raw| {
            raw.split('&')
                .any(|kv| kv.split_once('=').unwrap_or((kv, "")).0 == "tenant")
        })
}

fn body_has_tenant(req: &str) -> bool {
    serde_json::from_str::<Value>(super::body_from_request(req))
        .ok()
        .and_then(|v| v.get("tenant_id").cloned())
        .is_some()
}

pub(super) fn to_body(v: &impl serde::Serialize) -> String {
    serde_json::to_string(v).unwrap_or_else(|_| "{}".to_string())
}

pub(super) fn bad(msg: &str) -> (&'static str, String) {
    ("400 Bad Request", json!({"error":msg}).to_string())
}
pub(super) fn bad_owned(msg: String) -> (&'static str, String) {
    ("400 Bad Request", json!({"error":msg}).to_string())
}
fn unauthorized(msg: &str) -> (&'static str, String) {
    ("401 Unauthorized", json!({"error":msg}).to_string())
}
fn forbidden(msg: &str) -> (&'static str, String) {
    ("403 Forbidden", json!({"error":msg}).to_string())
}
pub(super) fn not_found() -> (&'static str, String) {
    ("404 Not Found", json!({"error":"not found"}).to_string())
}
pub(super) fn server(e: impl std::fmt::Display) -> (&'static str, String) {
    (
        "500 Internal Server Error",
        json!({"error":e.to_string()}).to_string(),
    )
}
pub(super) fn db(e: impl std::fmt::Display) -> ArgoError {
    ArgoError::Provider(e.to_string())
}
pub(super) fn conflict_or_server(e: ArgoError) -> (&'static str, String) {
    if matches!(e, ArgoError::Invariant(_)) {
        ("409 Conflict", json!({"error":e.to_string()}).to_string())
    } else {
        server(e)
    }
}

#[cfg(test)]
#[path = "routes_orchestrator_tests.rs"]
mod tests;
