use super::body_from_request;
use super::routes_auth::AuthenticatedPrincipal;
use argo_audit::{AuditKind, AuditSink};
use argo_skill::ir::{SkillGraph, SkillIrBundle};
use argo_store::SessionStore;
use argo_types::{SkillId, TenantId};
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;

const HEADER_TENANT_ID: &str = "x-argo-tenant-id";
const HEADER_CROSS_TENANT: &str = "x-argo-audit-cross-tenant";

pub(super) fn resolve_tenant(
    req: &str,
    query: &str,
    auth: &AuthenticatedPrincipal,
) -> Result<TenantId, (&'static str, String)> {
    let header_tenant =
        header_value(req, HEADER_TENANT_ID).unwrap_or_else(|| auth.tenant_id.to_string());
    let cross_cap = header_value(req, HEADER_CROSS_TENANT)
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    if let Some(override_tenant) = query_value(query, "as_tenant") {
        if override_tenant != header_tenant && !cross_cap {
            return Err(forbidden("cross-tenant access denied"));
        }
        return parse_tenant_id(&override_tenant);
    }
    parse_tenant_id(&header_tenant)
}

pub(super) fn parse_tenant_id(raw: &str) -> Result<TenantId, (&'static str, String)> {
    Uuid::parse_str(raw)
        .map(TenantId)
        .map_err(|e| bad_request(&format!("invalid tenant id '{raw}': {e}")))
}

pub(super) fn parse_skill_id(raw: &str) -> Result<SkillId, (&'static str, String)> {
    Uuid::parse_str(raw)
        .map(SkillId)
        .map_err(|e| bad_request(&format!("invalid skill id '{raw}': {e}")))
}

pub(super) fn query_value(query: &str, key: &str) -> Option<String> {
    if query.is_empty() {
        return None;
    }
    query.split('&').find_map(|kv| {
        let (k, v) = kv.split_once('=')?;
        if k == key && !v.is_empty() {
            Some(v.to_string())
        } else {
            None
        }
    })
}

pub(super) fn header_value(req: &str, name_lower: &str) -> Option<String> {
    let headers = req
        .split_once("\r\n\r\n")
        .map_or(req, |(headers, _body)| headers);
    headers.lines().skip(1).find_map(|line| {
        let (name, value) = line.split_once(':')?;
        name.trim()
            .eq_ignore_ascii_case(name_lower)
            .then(|| value.trim().to_string())
    })
}

pub(super) fn parse_save_graph(req: &str) -> Result<SkillGraph, (&'static str, String)> {
    let body = body_from_request(req);
    let parsed: Value =
        serde_json::from_str(body).map_err(|e| bad_request(&format!("parse body: {e}")))?;
    let graph_value = parsed
        .get("graph")
        .cloned()
        .ok_or_else(|| bad_request("`graph` is required"))?;
    serde_json::from_value(graph_value).map_err(|e| bad_request(&format!("decode graph: {e}")))
}

pub(super) fn parse_import_bundle(req: &str) -> Result<SkillIrBundle, (&'static str, String)> {
    let body = body_from_request(req);
    let parsed: Value =
        serde_json::from_str(body).map_err(|e| bad_request(&format!("parse body: {e}")))?;
    let bundle_value = parsed.get("bundle").cloned().unwrap_or(parsed);
    serde_json::from_value(bundle_value).map_err(|e| bad_request(&format!("decode bundle: {e}")))
}

pub(super) async fn emit_audit(
    store: &Arc<SessionStore>,
    principal_id: &str,
    event_type: &'static str,
    tenant: TenantId,
    skill_id: SkillId,
    version: Option<&str>,
) {
    let payload = json!({
        "endpoint": event_type,
        "tenant_id": tenant.to_string(),
        "skill_id": skill_id.to_string(),
        "version": version,
    });
    let sink = AuditSink::new_with_tenant(Arc::clone(store), tenant.to_string());
    if let Err(e) = sink
        .write(
            None,
            AuditKind::HttpRequest,
            payload,
            Some(principal_id.to_string()),
        )
        .await
    {
        tracing::warn!(
            target: "gateway::skill_ir",
            event = %event_type,
            error = %e,
            "skill IR audit write failed",
        );
    }
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
        json!({"error": {"code": code, "message": message}}).to_string(),
    )
}
