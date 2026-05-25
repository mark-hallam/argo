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
//! - `POST /api/v1/skills/<skillId>/ir` — save an edited graph as a
//!   new immutable version and schedule a promotion A/B run when a
//!   previous version exists.
//!
//! Bearer auth runs upstream in `routes::dispatch_route`; tenant is
//! resolved per `X-Argo-Tenant-Id` (with workspace-default fallback,
//! matching `routes_audit_feed_scope`). The audit row goes out via
//! the existing [`AuditSink`] under `AuditKind::HttpRequest` with an
//! `event_type` of `skill.ir.viewed` / `skill.ir.versions.viewed`.

use super::routes_auth::{
    authenticate_request, enforce_header_match, AuthCheckOutcome, AuthenticatedPrincipal,
};
use super::routes_skill_ir_helpers::*;
use super::store_unavailable;
use argo_skill::ir::{
    canonical_hash_hex, export_skill_ir_bundle, graph_from_skill_ir_bundle, save_edited_graph,
    SkillIrStore, SqliteSkillIrStore,
};
use argo_store::SessionStore;
use argo_types::TenantId;
use serde_json::{json, Value};
use std::sync::Arc;

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
        [skill_id, "ir", "export"] => Some(export_response(req, skill_id, query, store).await),
        [skill_id, "ir", "versions"] => Some(versions_response(req, skill_id, query, store).await),
        _ => None,
    }
}

/// Dispatcher hook for `POST /api/v1/skills/<id>/ir`.
pub(super) async fn dispatch_post(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    let rest = req.strip_prefix("POST /api/v1/skills/")?;
    let (head, _) = rest.split_once(' ')?;
    let (path, query) = head.split_once('?').unwrap_or((head, ""));
    let segments: Vec<&str> = path.split('/').collect();
    match segments.as_slice() {
        [skill_id, "ir"] => Some(save_response(req, skill_id, query, store).await),
        [skill_id, "ir", "import"] => Some(import_response(req, skill_id, query, store).await),
        _ => None,
    }
}

async fn import_response(
    req: &str,
    skill_id_raw: &str,
    query: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let (auth, tenant) = match authorize(req, query, store).await {
        Ok(p) => p,
        Err(resp) => return resp,
    };
    let skill_id = match parse_skill_id(skill_id_raw) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let bundle = match parse_import_bundle(req) {
        Ok(b) => b,
        Err(resp) => return resp,
    };
    let imported_hash = bundle.ir_hash.clone();
    let graph = match graph_from_skill_ir_bundle(&bundle) {
        Ok(g) => g,
        Err(e) => return bad_request(&format!("skill_ir import: {e}")),
    };
    let outcome = match save_edited_graph(store, tenant, skill_id, &graph).await {
        Ok(out) => out,
        Err(e) => return bad_request(&format!("skill_ir import save: {e}")),
    };
    emit_audit(
        store,
        &auth.principal,
        "skill.ir.imported",
        tenant,
        skill_id,
        Some(&outcome.version),
    )
    .await;
    let graph_value = match serde_json::to_value(&graph) {
        Ok(v) => v,
        Err(e) => return server_error(&format!("graph encode: {e}")),
    };
    let body = json!({
        "skillId": skill_id.to_string(),
        "tenantId": tenant.to_string(),
        "version": outcome.version,
        "irHash": outcome.ir_hash_hex,
        "importedHash": imported_hash,
        "graph": graph_value,
        "abTestScheduled": outcome.ab_test_run_id.is_some(),
        "abTestRunId": outcome.ab_test_run_id,
    });
    ("200 OK", body.to_string())
}

async fn save_response(
    req: &str,
    skill_id_raw: &str,
    query: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let (auth, tenant) = match authorize(req, query, store).await {
        Ok(p) => p,
        Err(resp) => return resp,
    };
    let skill_id = match parse_skill_id(skill_id_raw) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let graph = match parse_save_graph(req) {
        Ok(g) => g,
        Err(resp) => return resp,
    };
    let outcome = match save_edited_graph(store, tenant, skill_id, &graph).await {
        Ok(out) => out,
        Err(e) => return bad_request(&format!("skill_ir save: {e}")),
    };
    emit_audit(
        store,
        &auth.principal,
        "skill.ir.saved",
        tenant,
        skill_id,
        Some(&outcome.version),
    )
    .await;
    let graph_value = match serde_json::to_value(&graph) {
        Ok(v) => v,
        Err(e) => return server_error(&format!("graph encode: {e}")),
    };
    let body = json!({
        "skillId": skill_id.to_string(),
        "tenantId": tenant.to_string(),
        "version": outcome.version,
        "irHash": outcome.ir_hash_hex,
        "graph": graph_value,
        "abTestScheduled": outcome.ab_test_run_id.is_some(),
        "abTestRunId": outcome.ab_test_run_id,
    });
    ("200 OK", body.to_string())
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
    let (auth, tenant) = match authorize(req, query, store).await {
        Ok(p) => p,
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
        &auth.principal,
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

async fn export_response(
    req: &str,
    skill_id_raw: &str,
    query: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let (auth, tenant) = match authorize(req, query, store).await {
        Ok(p) => p,
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
    let bundle = match export_skill_ir_bundle(skill_id, &version_str, &graph) {
        Ok(b) => b,
        Err(e) => return server_error(&format!("skill_ir export: {e}")),
    };
    emit_audit(
        store,
        &auth.principal,
        "skill.ir.exported",
        tenant,
        skill_id,
        Some(&version_str),
    )
    .await;
    ("200 OK", json!(bundle).to_string())
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
    let (auth, tenant) = match authorize(req, query, store).await {
        Ok(p) => p,
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
        &auth.principal,
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

/// Authenticate the request, enforce that any caller-supplied
/// principal/tenant headers match the authenticated credential, then
/// resolve the tenant the handler should run against. Bundles the
/// three gates so each route handler can express auth + tenant in a
/// single call.
async fn authorize(
    req: &str,
    query: &str,
    store: &Arc<SessionStore>,
) -> Result<(AuthenticatedPrincipal, TenantId), (&'static str, String)> {
    let auth = match authenticate_request(req, Some(store)).await {
        AuthCheckOutcome::Pass(a) => a,
        AuthCheckOutcome::Reject(resp) => return Err(resp),
    };
    enforce_header_match(req, &auth)?;
    let tenant = resolve_tenant(req, query, &auth)?;
    Ok((auth, tenant))
}

#[cfg(test)]
#[path = "routes_skill_ir_tests.rs"]
mod tests;
