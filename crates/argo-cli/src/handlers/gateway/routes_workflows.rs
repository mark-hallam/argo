//! Workflow-definition gateway routes (Stream `mission-pm-workflow-edit-api`).
//!
//! Exposes 4 GET + 1 POST + 1 PUT + 1 DELETE handlers backed by the
//! `argo_store::workflow_definition` primitive shipped in #1846. The
//! handlers are mounted by sibling dispatchers
//! (`routes_dispatch_get`, `routes_dispatch_post`) — this module owns
//! routing within the `/api/v1/verticals/{vertical_id}/workflows`
//! subtree but never the cross-cutting wiring.
//!
//! ## Authorization model
//!
//! Each handler builds a workspace-scoped [`CapabilityEngine`] from
//! the operator role presets and resolves the caller's
//! [`Principal`] from the request's `X-Argo-Principal` header. The
//! `X-Argo-Role` header (`owner` / `admin` / `user` / `guest`) selects
//! the role binding; absent / unknown roles default to `guest` (least
//! privilege). Anonymous callers (no `X-Argo-Principal`) get
//! `401 Unauthorized` before the DB layer is touched. Authenticated
//! callers without the required capability get a generic
//! `403 Forbidden` so the response does NOT leak whether the
//! `(tenant, vertical, name)` triple exists.
//!
//! ## Audit emissions
//!
//! Every mutating handler writes one audit row through the sibling
//! `routes_workflows_handlers` module:
//!
//! * `POST /workflows/{name}` → [`argo_audit::AuditKind::WorkflowDefinitionCreated`]
//! * `PUT  /workflows/{name}/head` → [`argo_audit::AuditKind::WorkflowHeadChanged`]
//! * `DELETE /workflows/{name}/versions/{id}` → [`argo_audit::AuditKind::WorkflowVersionArchived`]
//!
//! Payload shape: `{ actor, tenant_id, vertical_id, name, version_id }`.

use super::role_gate::{enforce_role, resolve_role, Action};
use super::routes_workflows_handlers as hh;
use argo_capability::{
    workflow_edit_capability, workflow_read_capability, CapabilityEngine,
};
use argo_identity::user_tenant::UserTenantRole;
use argo_store::SessionStore;
use argo_types::{Capability, Principal, TenantId};
use serde_json::json;
use std::sync::Arc;

const PREFIX: &str = "/api/v1/verticals/";
const PRINCIPAL_HEADER: &str = "X-Argo-Principal";

/// GET dispatcher. Returns `Some` for any path under
/// `/api/v1/verticals/{id}/workflows[...]`; `None` lets the parent
/// dispatcher fall through to other GET arms.
pub(super) async fn dispatch_get(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    let route = parse_route(req, "GET ")?;
    let store = match store {
        Some(s) => s,
        None => return Some(super::store_unavailable()),
    };
    Some(handle_get(&route, req, store).await)
}

/// POST / PUT / DELETE dispatcher.
pub(super) async fn dispatch_mutate(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    let verb = if req.starts_with("POST ") {
        "POST "
    } else if req.starts_with("PUT ") {
        "PUT "
    } else if req.starts_with("DELETE ") {
        "DELETE "
    } else {
        return None;
    };
    let route = parse_route(req, verb)?;
    let store = match store {
        Some(s) => s,
        None => return Some(super::store_unavailable()),
    };
    Some(handle_mutate(&route, req, store).await)
}

async fn handle_get(
    route: &Route<'_>,
    req: &str,
    store: &Arc<SessionStore>,
) -> (&'static str, String) {
    let (principal, role) = match resolve_principal(req) {
        Ok(p) => p,
        Err(resp) => return resp,
    };
    if let Err(resp) = check_capability(&principal, role, &workflow_read_capability()) {
        return resp;
    }
    let tenant = match resolve_tenant(store).await {
        Ok(t) => t,
        Err(resp) => return resp,
    };
    match *route {
        Route::List { vertical } => hh::list_workflows(store, tenant, vertical).await,
        Route::Head { vertical, name } => hh::fetch_head(store, tenant, vertical, name).await,
        Route::Versions { vertical, name } => {
            hh::list_versions(store, tenant, vertical, name).await
        }
        Route::Version {
            vertical,
            name,
            version_id,
        } => hh::fetch_version(store, tenant, vertical, name, version_id).await,
        _ => not_found("unknown workflow route"),
    }
}

async fn handle_mutate(
    route: &Route<'_>,
    req: &str,
    store: &Arc<SessionStore>,
) -> (&'static str, String) {
    if let Some(resp) = enforce_role(req, Action::WorkflowEdit) {
        return resp;
    }
    let (principal, role) = match resolve_principal(req) {
        Ok(p) => p,
        Err(resp) => return resp,
    };
    if let Err(resp) = check_capability(&principal, role, &workflow_edit_capability()) {
        return resp;
    }
    let tenant = match resolve_tenant(store).await {
        Ok(t) => t,
        Err(resp) => return resp,
    };
    let actor = principal_to_actor(&principal);
    let body = super::body_from_request(req);
    match *route {
        Route::Create { vertical, name } => {
            hh::create_version(store, tenant, vertical, name, body, &actor).await
        }
        Route::SetHead { vertical, name } => {
            hh::set_head(store, tenant, vertical, name, body, &actor).await
        }
        Route::Archive {
            vertical,
            name,
            version_id,
        } => hh::archive_version(store, tenant, vertical, name, version_id, &actor).await,
        _ => not_found("unknown workflow route"),
    }
}

// ── Route parsing ────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
enum Route<'a> {
    List { vertical: &'a str },
    Head { vertical: &'a str, name: &'a str },
    Versions { vertical: &'a str, name: &'a str },
    Version {
        vertical: &'a str,
        name: &'a str,
        version_id: &'a str,
    },
    Create { vertical: &'a str, name: &'a str },
    SetHead { vertical: &'a str, name: &'a str },
    Archive {
        vertical: &'a str,
        name: &'a str,
        version_id: &'a str,
    },
}

fn parse_route<'a>(req: &'a str, verb: &str) -> Option<Route<'a>> {
    let after_verb = req.strip_prefix(verb)?;
    let path_with_query = after_verb.split_whitespace().next()?;
    let path = path_with_query.split('?').next()?;
    let rest = path.strip_prefix(PREFIX)?;
    let mut iter = rest.splitn(2, '/');
    let vertical = iter.next().filter(|s| !s.is_empty())?;
    let tail = iter.next().unwrap_or("");
    let tail = tail.strip_prefix("workflows")?;
    if !tail.is_empty() && !tail.starts_with('/') {
        return None;
    }
    let tail = tail.trim_start_matches('/');
    let segments: Vec<&str> = if tail.is_empty() {
        Vec::new()
    } else {
        tail.split('/').collect()
    };
    let is_get = verb == "GET ";
    let is_post = verb == "POST ";
    let is_put = verb == "PUT ";
    let is_delete = verb == "DELETE ";
    match segments.as_slice() {
        [] if is_get => Some(Route::List { vertical }),
        [name] if is_get => Some(Route::Head { vertical, name }),
        [name] if is_post => Some(Route::Create { vertical, name }),
        [name, "versions"] if is_get => Some(Route::Versions { vertical, name }),
        [name, "versions", version_id] if is_get => Some(Route::Version {
            vertical,
            name,
            version_id,
        }),
        [name, "versions", version_id] if is_delete => Some(Route::Archive {
            vertical,
            name,
            version_id,
        }),
        [name, "head"] if is_put => Some(Route::SetHead { vertical, name }),
        _ => None,
    }
}

// ── Auth helpers ────────────────────────────────────────────────

fn resolve_principal(req: &str) -> Result<(Principal, &'static str), (&'static str, String)> {
    let raw = header_value(req, PRINCIPAL_HEADER).ok_or_else(unauthorized)?;
    let (kind, identity) = match raw.split_once(':') {
        Some((k, v)) if !v.is_empty() => (k.to_string(), v.to_string()),
        _ => return Err(unauthorized()),
    };
    let principal = Principal::Gateway {
        adapter: kind,
        identity,
    };
    // Role comes from the verified session JWT
    // (see [`super::role_gate::resolve_role`]). When no JWT is
    // present, fall back to `guest` so the capability engine never
    // sees a missing role — `WORKFLOW_READ` is granted to guest, so
    // anonymous read paths still work for unauthenticated dev /
    // marketing surfaces. Mutating paths are gated by the
    // [`Action::WorkflowEdit`] check in [`handle_mutate`].
    let role = match resolve_role(req).unwrap_or(UserTenantRole::Guest) {
        UserTenantRole::Owner => "owner",
        UserTenantRole::Admin => "admin",
        UserTenantRole::User => "user",
        UserTenantRole::Guest => "guest",
    };
    Ok((principal, role))
}

fn check_capability(
    principal: &Principal,
    role: &str,
    requested: &Capability,
) -> Result<(), (&'static str, String)> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let engine = CapabilityEngine::with_workspace_defaults(&cwd);
    if engine.bind(principal.clone(), role.to_string()).is_err() {
        return Err(forbidden());
    }
    engine.check(principal, requested).map_err(|_| forbidden())
}

fn header_value(req: &str, name: &str) -> Option<String> {
    let headers = req
        .split_once("\r\n\r\n")
        .map_or(req, |(headers, _body)| headers);
    headers.lines().skip(1).find_map(|line| {
        let (k, v) = line.split_once(':')?;
        k.eq_ignore_ascii_case(name).then(|| v.trim().to_string())
    })
}

fn principal_to_actor(principal: &Principal) -> String {
    match principal {
        Principal::Gateway { adapter, identity } => format!("{adapter}:{identity}"),
        Principal::Local { os_user } => format!("local:{os_user}"),
        Principal::Acp { harness, identity } => format!("{harness}:{identity}"),
        Principal::Forge => "forge".to_string(),
    }
}

async fn resolve_tenant(store: &SessionStore) -> Result<TenantId, (&'static str, String)> {
    let cwd = std::env::current_dir().map_err(|e| internal(&format!("cwd: {e}")))?;
    let name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    store
        .ensure_workspace(name, &root)
        .await
        .map(|(t, _)| t)
        .map_err(|e| internal(&format!("resolve tenant: {e}")))
}

// ── Error helpers (shared with sibling handler module) ─────────

pub(super) fn unauthorized() -> (&'static str, String) {
    (
        "401 Unauthorized",
        json!({"error": {
            "code": "unauthorized",
            "message": "X-Argo-Principal header is required"
        }})
        .to_string(),
    )
}

pub(super) fn forbidden() -> (&'static str, String) {
    (
        "403 Forbidden",
        json!({"error": {
            "code": "forbidden",
            "message": "principal lacks the required workflow capability"
        }})
        .to_string(),
    )
}

pub(super) fn not_found(message: &str) -> (&'static str, String) {
    (
        "404 Not Found",
        json!({"error": {"code": "not_found", "message": message}}).to_string(),
    )
}

pub(super) fn bad_request(message: &str) -> (&'static str, String) {
    (
        "400 Bad Request",
        json!({"error": {"code": "bad_request", "message": message}}).to_string(),
    )
}

pub(super) fn internal(message: &str) -> (&'static str, String) {
    (
        "500 Internal Server Error",
        json!({"error": {"code": "internal", "message": message}}).to_string(),
    )
}
