//! SCIM 2.0 User-provisioning endpoints
//! (`mission-scim-provisioning`, MISSION_BACKLOG Section 7 #231).
//!
//! Mounts five routes under `/scim/v2/Users`:
//!
//! - `GET    /scim/v2/Users`          — list (filter, startIndex, count)
//! - `POST   /scim/v2/Users`          — provision
//! - `GET    /scim/v2/Users/<id>`     — read
//! - `PATCH  /scim/v2/Users/<id>`     — partial update (active=false → deactivate)
//! - `DELETE /scim/v2/Users/<id>`     — soft-delete (tombstone row inserted)
//!
//! Auth: `ARGO_SCIM_BEARER_TOKEN` (distinct from the operator-facing
//! `ARGO_GATEWAY_BEARER_TOKEN` so the IdP receives a scoped
//! credential). The gate is owned by [`super::scim_auth`].
//!
//! Response envelopes follow RFC 7644 §3 — every body carries the
//! `schemas` field, every error carries the
//! `urn:ietf:params:scim:api:messages:2.0:Error` schema.
//!
//! Tenant scoping: workspace-derived (single-tenant per workspace).
//! Cross-tenant lookups are blocked at the storage layer.
//!
//! Pure helpers (envelope builders, query parsing, PATCH translation)
//! live in `routes_scim_users_envelope` to keep this file under the
//! 500-LOC budget.

use super::routes_scim_users_envelope::{
    build_patch, extract_emails, list_envelope, ok_json, parse_list_query, patch_changes_anything,
    scim_error, scim_error_with_type, user_to_envelope,
};
use super::scim_auth::{check_scim_bearer, is_scim_path};
use argo_store::scim_user::{NewScimUser, ScimUserStore};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use chrono::Utc;
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;

/// Entry point for the gateway dispatchers. Returns `None` for
/// non-SCIM requests so the caller continues its normal match chain;
/// otherwise runs the bearer-token gate and dispatches by verb + path.
pub(super) async fn dispatch(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    if !is_scim_path(req) {
        return None;
    }
    if let Some(resp) = check_scim_bearer(req) {
        return Some(resp);
    }
    let (verb, path) = method_and_path(req)?;
    let store = match store {
        Some(s) => s,
        None => return Some(super::store_unavailable()),
    };
    let tenant_id = match resolve_tenant(store).await {
        Ok(t) => t,
        Err(e) => return Some(scim_error(500, &format!("resolve tenant: {e}"))),
    };
    let scim = ScimUserStore::new(store.pool());
    match (verb, route_segments(path).as_slice()) {
        ("GET", ["scim", "v2", "Users"]) => Some(list_users(&scim, tenant_id, path).await),
        ("POST", ["scim", "v2", "Users"]) => Some(create_user(&scim, tenant_id, req).await),
        ("GET", ["scim", "v2", "Users", id]) => Some(get_user(&scim, id).await),
        ("PATCH", ["scim", "v2", "Users", id]) => Some(patch_user(&scim, id, req).await),
        ("DELETE", ["scim", "v2", "Users", id]) => Some(delete_user(&scim, id).await),
        ("GET" | "POST" | "PATCH" | "DELETE", _) => {
            Some(scim_error(404, &format!("no SCIM route for {verb} {path}")))
        }
        _ => Some(scim_error(405, &format!("method {verb} not allowed"))),
    }
}

/// `GET /scim/v2/Users[?filter=userName eq "alice"&startIndex=1&count=100]`.
async fn list_users(
    scim: &ScimUserStore<'_>,
    tenant_id: TenantId,
    path: &str,
) -> (&'static str, String) {
    let query = match parse_list_query(path) {
        Ok(v) => v,
        Err(msg) => return scim_error(400, &msg),
    };
    if let Some(needle) = query.filter_user_name {
        let row = match scim.find_by_user_name(tenant_id, &needle).await {
            Ok(v) => v,
            Err(e) => return scim_error(500, &format!("filter: {e}")),
        };
        let items: Vec<Value> = row.iter().map(user_to_envelope).collect();
        let total = i64::try_from(items.len()).unwrap_or(0);
        return ok_json(list_envelope(items, total, query.start_index, total));
    }
    let (rows, total) = match scim.list(tenant_id, query.start_index, query.count).await {
        Ok(v) => v,
        Err(e) => return scim_error(500, &format!("list: {e}")),
    };
    let items: Vec<Value> = rows.iter().map(user_to_envelope).collect();
    let returned = i64::try_from(items.len()).unwrap_or(0);
    ok_json(list_envelope(items, total, query.start_index, returned))
}

/// `POST /scim/v2/Users` — provision.
async fn create_user(
    scim: &ScimUserStore<'_>,
    tenant_id: TenantId,
    req: &str,
) -> (&'static str, String) {
    let body = super::body_from_request(req);
    let parsed: Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => return scim_error(400, &format!("parse body: {e}")),
    };
    let user_name = match parsed.get("userName").and_then(Value::as_str) {
        Some(s) if !s.trim().is_empty() => s.trim().to_string(),
        _ => return scim_error(400, "`userName` is required"),
    };
    match scim.find_by_user_name(tenant_id, &user_name).await {
        Ok(Some(_)) => {
            return scim_error_with_type(409, "uniqueness", "userName already in use");
        }
        Ok(None) => {}
        Err(e) => return scim_error(500, &format!("uniqueness check: {e}")),
    }
    let new = NewScimUser {
        tenant_id,
        external_id: parsed
            .get("externalId")
            .and_then(Value::as_str)
            .map(str::to_string),
        user_name: user_name.clone(),
        display_name: parsed
            .get("displayName")
            .and_then(Value::as_str)
            .map(str::to_string),
        emails: extract_emails(parsed.get("emails")),
        active: parsed.get("active").and_then(Value::as_bool).unwrap_or(true),
        created_at: Utc::now(),
    };
    let id = match scim.insert(new).await {
        Ok(id) => id,
        Err(e) => return scim_error(500, &format!("insert: {e}")),
    };
    match scim.get(id).await {
        Ok(Some(row)) => ("201 Created", user_to_envelope(&row).to_string()),
        Ok(None) => scim_error(500, "user vanished after insert"),
        Err(e) => scim_error(500, &format!("re-read: {e}")),
    }
}

/// `GET /scim/v2/Users/<id>`.
async fn get_user(scim: &ScimUserStore<'_>, id: &str) -> (&'static str, String) {
    let parsed = match Uuid::parse_str(id) {
        Ok(u) => u,
        Err(_) => return scim_error(400, "invalid user id (expected UUID)"),
    };
    match scim.get(parsed).await {
        Ok(Some(row)) => ok_json(user_to_envelope(&row)),
        Ok(None) => scim_error(404, &format!("no user with id '{id}'")),
        Err(e) => scim_error(500, &format!("get: {e}")),
    }
}

/// `PATCH /scim/v2/Users/<id>` — partial update.
///
/// Accepts both the RFC 7644 PatchOp envelope (`schemas: [PatchOp],
/// Operations: [...]`) and a plain-attribute body (`{ active: false,
/// displayName: "..." }`). The plain form is what Okta sends in
/// some integration profiles; supporting both keeps interop wide.
async fn patch_user(
    scim: &ScimUserStore<'_>,
    id: &str,
    req: &str,
) -> (&'static str, String) {
    let parsed = match Uuid::parse_str(id) {
        Ok(u) => u,
        Err(_) => return scim_error(400, "invalid user id (expected UUID)"),
    };
    let body = super::body_from_request(req);
    let value: Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => return scim_error(400, &format!("parse body: {e}")),
    };
    let patch = match build_patch(&value) {
        Ok(p) => p,
        Err(msg) => return scim_error(400, &msg),
    };
    if !patch_changes_anything(&patch) {
        return match scim.get(parsed).await {
            Ok(Some(row)) => ok_json(user_to_envelope(&row)),
            Ok(None) => scim_error(404, &format!("no user with id '{id}'")),
            Err(e) => scim_error(500, &format!("patch read: {e}")),
        };
    }
    match scim.update(parsed, patch).await {
        Ok(true) => {}
        Ok(false) => return scim_error(404, &format!("no user with id '{id}'")),
        Err(e) => return scim_error(500, &format!("patch: {e}")),
    }
    match scim.get(parsed).await {
        Ok(Some(row)) => ok_json(user_to_envelope(&row)),
        Ok(None) => scim_error(404, &format!("no user with id '{id}'")),
        Err(e) => scim_error(500, &format!("patch re-read: {e}")),
    }
}

/// `DELETE /scim/v2/Users/<id>` — soft-delete + tombstone insert.
async fn delete_user(scim: &ScimUserStore<'_>, id: &str) -> (&'static str, String) {
    let parsed = match Uuid::parse_str(id) {
        Ok(u) => u,
        Err(_) => return scim_error(400, "invalid user id (expected UUID)"),
    };
    match scim.soft_delete(parsed, Utc::now()).await {
        Ok(true) => ("204 No Content", String::new()),
        Ok(false) => scim_error(404, &format!("no live user with id '{id}'")),
        Err(e) => scim_error(500, &format!("delete: {e}")),
    }
}

async fn resolve_tenant(store: &SessionStore) -> ArgoResult<TenantId> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (tenant, _ws) = store.ensure_workspace(name, &root).await?;
    Ok(tenant)
}

fn method_and_path(req: &str) -> Option<(&str, &str)> {
    let line = req.lines().next()?;
    let mut tokens = line.split_whitespace();
    let verb = tokens.next()?;
    let path = tokens.next()?;
    Some((verb, path))
}

fn route_segments(path: &str) -> Vec<&str> {
    let head = path.split('?').next().unwrap_or(path);
    head.split('/').filter(|s| !s.is_empty()).collect()
}

#[cfg(test)]
#[path = "routes_scim_users_tests.rs"]
mod tests;
