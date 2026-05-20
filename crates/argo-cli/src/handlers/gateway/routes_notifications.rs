//! Gateway routes for the bell-popover notifications API
//! (Stream `web-gateway-endpoints-notifications-batch`).
//!
//! Bearer auth is applied upstream in `routes::dispatch_route` (GET)
//! and `routes_dispatch_post::preflight_dispatch` (POST); this module
//! resolves the workspace tenant from cwd (the same convention as
//! `routes_team_list`), parses the `X-Argo-Principal` header to scope
//! visibility, and forwards to `argo_store::notifications`.
//!
//! Endpoints:
//!   - `GET  /api/v1/notifications[?limit=&before=]`
//!     → `{ items, has_more, cursor }`
//!   - `GET  /api/v1/notifications/unread-count`
//!     → `{ unread }`
//!   - `POST /api/v1/notifications/<id>/dismiss`
//!     → `{ ok, notificationId }`
//!   - `POST /api/v1/notifications/dismiss-all`
//!     → `{ ok, dismissed }`

use super::routes_auth_principal::principal_id_for_audit;
use argo_store::notifications::{
    dismiss_all_for_principal, dismiss_one, list_for_principal, unread_count, ListQuery,
    NotificationRow, MAX_LIMIT,
};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use serde_json::{json, Value};
use std::sync::Arc;

/// Match arm dispatched from `routes_dispatch_get_api::dispatch_api_get`
/// for the two GET endpoints. Returns `Some(resp)` when the prefix
/// matches, `None` otherwise.
pub(super) async fn dispatch_get(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    if req.starts_with("GET /api/v1/notifications/unread-count ")
        || req.starts_with("GET /api/v1/notifications/unread-count?")
    {
        return Some(unread_count_response(req, store).await);
    }
    if req.starts_with("GET /api/v1/notifications ") || req.starts_with("GET /api/v1/notifications?")
    {
        return Some(list_response(req, store).await);
    }
    None
}

/// Match arm dispatched from `routes_dispatch_post::api_v1_dispatch`
/// for the two POST endpoints.
pub(super) async fn dispatch_post(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    if req.starts_with("POST /api/v1/notifications/dismiss-all ")
        || req.starts_with("POST /api/v1/notifications/dismiss-all?")
    {
        return Some(dismiss_all_response(req, store).await);
    }
    if let Some(rest) = req.strip_prefix("POST /api/v1/notifications/") {
        if let Some((head, _)) = rest.split_once(' ') {
            let path = head.split('?').next().unwrap_or(head);
            if let Some(id) = path.strip_suffix("/dismiss") {
                if !id.is_empty() && !id.contains('/') {
                    return Some(dismiss_one_response(req, id, store).await);
                }
            }
        }
    }
    None
}

async fn list_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let store = match store {
        Some(s) => s,
        None => return super::store_unavailable(),
    };
    let tenant = match resolve_workspace_tenant(store).await {
        Ok(t) => t.to_string(),
        Err(e) => return server_error(&format!("resolve tenant: {e}")),
    };
    let principal = principal_id_for_audit(req);
    let query = match parse_list_query(req) {
        Ok(q) => q,
        Err(msg) => return bad_request(&msg),
    };
    match list_for_principal(store.pool(), &tenant, principal.as_deref(), &query).await {
        Ok(page) => {
            let items: Vec<Value> = page.items.iter().map(NotificationRow::to_wire_json).collect();
            let body = json!({
                "items": items,
                "has_more": page.has_more,
                "cursor": page.cursor,
            })
            .to_string();
            ("200 OK", body)
        }
        Err(e) => server_error(&format!("notifications list: {e}")),
    }
}

async fn unread_count_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let store = match store {
        Some(s) => s,
        None => return super::store_unavailable(),
    };
    let tenant = match resolve_workspace_tenant(store).await {
        Ok(t) => t.to_string(),
        Err(e) => return server_error(&format!("resolve tenant: {e}")),
    };
    let principal = principal_id_for_audit(req);
    match unread_count(store.pool(), &tenant, principal.as_deref()).await {
        Ok(n) => ("200 OK", json!({ "unread": n }).to_string()),
        Err(e) => server_error(&format!("notifications unread-count: {e}")),
    }
}

async fn dismiss_one_response(
    req: &str,
    id: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    if !is_safe_id(id) {
        return bad_request("invalid notification id");
    }
    let store = match store {
        Some(s) => s,
        None => return super::store_unavailable(),
    };
    let tenant = match resolve_workspace_tenant(store).await {
        Ok(t) => t.to_string(),
        Err(e) => return server_error(&format!("resolve tenant: {e}")),
    };
    let principal = principal_id_for_audit(req);
    let now = chrono::Utc::now().timestamp();
    match dismiss_one(store.pool(), &tenant, principal.as_deref(), id, now).await {
        Ok(_) => (
            "200 OK",
            json!({ "ok": true, "notificationId": id }).to_string(),
        ),
        Err(e) => server_error(&format!("notifications dismiss: {e}")),
    }
}

async fn dismiss_all_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let store = match store {
        Some(s) => s,
        None => return super::store_unavailable(),
    };
    let tenant = match resolve_workspace_tenant(store).await {
        Ok(t) => t.to_string(),
        Err(e) => return server_error(&format!("resolve tenant: {e}")),
    };
    let principal = principal_id_for_audit(req);
    let now = chrono::Utc::now().timestamp();
    match dismiss_all_for_principal(store.pool(), &tenant, principal.as_deref(), now).await {
        Ok(n) => (
            "200 OK",
            json!({ "ok": true, "dismissed": n }).to_string(),
        ),
        Err(e) => server_error(&format!("notifications dismiss-all: {e}")),
    }
}

/// Pull `?limit=&before=` from the request line. Anything else is
/// ignored. Defaults: limit → store default, before → none. Both
/// values clamp at the store's `MAX_LIMIT` ceiling.
fn parse_list_query(req: &str) -> Result<ListQuery, String> {
    let head = req.lines().next().unwrap_or("");
    let target = head.split_whitespace().nth(1).unwrap_or("");
    let query_str = target.split_once('?').map(|(_, q)| q).unwrap_or("");
    let mut limit: Option<u32> = None;
    let mut before: Option<i64> = None;
    for (k, v) in iter_query_pairs(query_str) {
        match k {
            "limit" => {
                let n: u32 = v
                    .parse()
                    .map_err(|_| format!("invalid limit '{v}'"))?;
                limit = Some(n.min(MAX_LIMIT));
            }
            "before" => {
                let n: i64 = v
                    .parse()
                    .map_err(|_| format!("invalid before '{v}'"))?;
                before = Some(n);
            }
            _ => {}
        }
    }
    Ok(ListQuery {
        limit,
        before,
        exclude_dismissed: true,
    })
}

fn iter_query_pairs(query_str: &str) -> impl Iterator<Item = (&str, &str)> {
    query_str.split('&').filter_map(|kv| kv.split_once('='))
}

async fn resolve_workspace_tenant(store: &SessionStore) -> ArgoResult<TenantId> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (tenant, _ws) = store.ensure_workspace(name, &root).await?;
    Ok(tenant)
}

/// Accept ids matching `[A-Za-z0-9_-]{1,128}` so the URL path cannot
/// smuggle separators or SQL metacharacters. The store still scopes
/// the UPDATE by `(tenant, principal)` independently — this is a
/// belt-and-braces validation at the gateway boundary.
fn is_safe_id(id: &str) -> bool {
    if id.is_empty() || id.len() > 128 {
        return false;
    }
    id.bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

fn bad_request(message: &str) -> (&'static str, String) {
    error_envelope("400 Bad Request", "bad_request", message)
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
#[path = "routes_notifications_tests.rs"]
mod tests;
