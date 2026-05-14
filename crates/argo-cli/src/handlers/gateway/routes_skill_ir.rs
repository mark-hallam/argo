//! `/api/v1/skills/<skillId>/ir` — Skill IR JSON endpoints serving
//! the L4 canvas viewer (Track L brief: `streams/L4-gw-gateway-skill-ir-endpoint.md`).
//!
//! - `GET /api/v1/skills/<skillId>/ir?version=<v>` — canonical
//!   [`SkillGraph`] JSON for `(tenant, skill_id, version)`. Omit
//!   `version` to fetch the latest revision. Returns the same
//!   camelCase shape produced by `argo-skill::ir::canonical_json`,
//!   wrapped with a `{skillId, version, irHash, graph}` envelope so
//!   the TS viewer can render history without re-hashing.
//! - `GET /api/v1/skills/<skillId>/ir/versions` — every recorded
//!   `(version, ir_hash)` pair for the skill under the caller's
//!   tenant, newest first. Used by the version-history dropdown.
//!
//! Bearer auth runs upstream in `routes::dispatch_route`; tenant is
//! resolved per `X-Argo-Tenant-Id` (with workspace-default fallback,
//! matching `routes_audit_feed_scope`). The audit row goes out via
//! the existing [`AuditSink`] under `AuditKind::HttpRequest` with an
//! `event_type` of `skill.ir.viewed` / `skill.ir.versions.viewed`.

use super::routes_auth_principal::principal_id_for_audit;
use super::store_unavailable;
use argo_audit::{AuditKind, AuditSink};
use argo_skill::ir::{SkillIrStore, SqliteSkillIrStore, canonical_hash_hex};
use argo_store::SessionStore;
use argo_types::{SkillId, TenantId};
use serde_json::{Value, json};
use std::sync::Arc;
use uuid::Uuid;

const HEADER_TENANT_ID: &str = "x-argo-tenant-id";
const HEADER_CROSS_TENANT: &str = "x-argo-audit-cross-tenant";
const DEFAULT_WORKSPACE_TENANT_FALLBACK: &str = "00000000-0000-0000-0000-000000000000";

/// Dispatcher hook for `GET /api/v1/skills/<id>/ir[...]`. Returns
/// `None` when the request shape doesn't match either of this
/// module's two routes so the caller can fall through to the next
/// arm of `dispatch_api_get`.
pub(super) async fn dispatch(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    let rest = req.strip_prefix("GET /api/v1/skills/")?;
    let (head, _) = rest.split_once(' ')?;
    let (path, query) = head.split_once('?').unwrap_or((head, ""));
    let segments: Vec<&str> = path.split('/').collect();
    match segments.as_slice() {
        [skill_id, "ir"] => Some(ir_response(req, skill_id, query, store).await),
        [skill_id, "ir", "versions"] => {
            Some(versions_response(req, skill_id, query, store).await)
        }
        _ => None,
    }
}

async fn ir_response(
    req: &str,
    skill_id_raw: &str,
    query: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let tenant = match resolve_tenant(req, query) {
        Ok(t) => t,
        Err(resp) => return resp,
    };
    let skill_id = match parse_skill_id(skill_id_raw) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let version = query_value(query, "version");
    let ir_store = SqliteSkillIrStore::new(store);
    let (version_str, graph) = match version.as_deref() {
        Some(v) => match ir_store.get(tenant, skill_id, v).await {
            Ok(Some(g)) => (v.to_string(), g),
            Ok(None) => return not_found("skill IR version not found"),
            Err(e) => return server_error(&format!("skill_ir get: {e}")),
        },
        None => match ir_store.latest(tenant, skill_id).await {
            Ok(Some((v, g))) => (v, g),
            Ok(None) => return not_found("skill has no IR"),
            Err(e) => return server_error(&format!("skill_ir latest: {e}")),
        },
    };
    let hash = match canonical_hash_hex(&graph) {
        Ok(h) => h,
        Err(e) => return server_error(&format!("canonical_hash: {e}")),
    };
    let graph_value = match serde_json::to_value(&graph) {
        Ok(v) => v,
        Err(e) => return server_error(&format!("graph encode: {e}")),
    };
    emit_audit(
        store,
        req,
        "skill.ir.viewed",
        tenant,
        skill_id,
        Some(&version_str),
    )
    .await;
    let body = json!({
        "skillId": skill_id.to_string(),
        "tenantId": tenant.to_string(),
        "version": version_str,
        "irHash": hash,
        "graph": graph_value,
    });
    ("200 OK", body.to_string())
}

async fn versions_response(
    req: &str,
    skill_id_raw: &str,
    query: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let tenant = match resolve_tenant(req, query) {
        Ok(t) => t,
        Err(resp) => return resp,
    };
    let skill_id = match parse_skill_id(skill_id_raw) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let ir_store = SqliteSkillIrStore::new(store);
    let versions = match ir_store.list_versions(tenant, skill_id).await {
        Ok(v) => v,
        Err(e) => return server_error(&format!("skill_ir list_versions: {e}")),
    };
    if versions.is_empty() {
        return not_found("skill has no IR");
    }
    let mut items: Vec<Value> = Vec::with_capacity(versions.len());
    for version in &versions {
        let graph = match ir_store.get(tenant, skill_id, version).await {
            Ok(Some(g)) => g,
            Ok(None) => {
                return server_error(&format!(
                    "skill_ir versions: listed version '{version}' vanished"
                ));
            }
            Err(e) => return server_error(&format!("skill_ir get: {e}")),
        };
        let hash = match canonical_hash_hex(&graph) {
            Ok(h) => h,
            Err(e) => return server_error(&format!("canonical_hash: {e}")),
        };
        items.push(json!({ "version": version, "irHash": hash }));
    }
    emit_audit(
        store,
        req,
        "skill.ir.versions.viewed",
        tenant,
        skill_id,
        None,
    )
    .await;
    let body = json!({
        "skillId": skill_id.to_string(),
        "tenantId": tenant.to_string(),
        "versions": items,
    });
    ("200 OK", body.to_string())
}

fn resolve_tenant(req: &str, query: &str) -> Result<TenantId, (&'static str, String)> {
    let header_tenant = header_value(req, HEADER_TENANT_ID)
        .unwrap_or_else(|| DEFAULT_WORKSPACE_TENANT_FALLBACK.to_string());
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

fn parse_tenant_id(raw: &str) -> Result<TenantId, (&'static str, String)> {
    Uuid::parse_str(raw)
        .map(TenantId)
        .map_err(|e| bad_request(&format!("invalid tenant id '{raw}': {e}")))
}

fn parse_skill_id(raw: &str) -> Result<SkillId, (&'static str, String)> {
    Uuid::parse_str(raw)
        .map(SkillId)
        .map_err(|e| bad_request(&format!("invalid skill id '{raw}': {e}")))
}

fn query_value(query: &str, key: &str) -> Option<String> {
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

fn header_value(req: &str, name_lower: &str) -> Option<String> {
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

async fn emit_audit(
    store: &Arc<SessionStore>,
    req: &str,
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
            principal_id_for_audit(req),
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

fn bad_request(message: &str) -> (&'static str, String) {
    error_envelope("400 Bad Request", "bad_request", message)
}

fn not_found(message: &str) -> (&'static str, String) {
    error_envelope("404 Not Found", "not_found", message)
}

fn forbidden(message: &str) -> (&'static str, String) {
    error_envelope("403 Forbidden", "forbidden", message)
}

fn server_error(message: &str) -> (&'static str, String) {
    error_envelope("500 Internal Server Error", "internal_error", message)
}

fn error_envelope(
    status: &'static str,
    code: &str,
    message: &str,
) -> (&'static str, String) {
    (
        status,
        json!({"error": {"code": code, "message": message}}).to_string(),
    )
}

#[cfg(test)]
#[path = "routes_skill_ir_tests.rs"]
mod tests;
