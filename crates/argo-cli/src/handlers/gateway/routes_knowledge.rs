//! Workplace-knowledge gateway routes (Stream
//! `mission-workplace-knowledge-gateway`).
//!
//! Exposes 6 endpoints backed by the tenant-scoped wiki substrate in
//! `argo-workplace-knowledge` (brief 1 of Wave A) and emitting the
//! audit kinds shipped in `argo-audit` (brief 2 of Wave A):
//!
//! | Method | Path                                  | Floor       | Audit                                  |
//! |--------|---------------------------------------|-------------|----------------------------------------|
//! | GET    | /api/v1/knowledge                     | Guest       | none                                   |
//! | GET    | /api/v1/knowledge/<slug>              | Guest       | none                                   |
//! | POST   | /api/v1/knowledge                     | Admin+      | `WorkplaceKnowledgeDocCreated`         |
//! | PUT    | /api/v1/knowledge/<slug>              | Admin+      | `WorkplaceKnowledgeDocUpdated` (+diff) |
//! | DELETE | /api/v1/knowledge/<slug>              | Owner only  | `WorkplaceKnowledgeDocDeleted`         |
//! | GET    | /api/v1/knowledge/<slug>/revisions    | Guest       | none                                   |
//!
//! ## Authorization
//!
//! Reads are guest-visible (the handbook feeds every agent's system
//! prompt — a guest auditor on a shared dev workspace must be able to
//! see it). Writes gate on [`Action::KnowledgeWrite`] (Admin+); the
//! destructive head removal gates on [`Action::KnowledgeDelete`]
//! (Owner only) per the design doc §5 brief 3.
//!
//! ## Cross-tenant guard
//!
//! Every read and write binds the cwd-derived tenant id into the
//! `WHERE` clause of the underlying substrate call (see
//! [`argo_workplace_knowledge::doc::get`] and siblings). Slugs are
//! tenant-scoped — two tenants may BOTH have a `brand-voice` doc —
//! and a cross-tenant lookup returns 404 (not 403) so the response
//! never leaks whether the slug exists in some other tenant.

use super::role_gate::{enforce_role, Action};
use argo_store::SessionStore;
use argo_types::TenantId;
use serde_json::json;
use std::sync::Arc;

use super::routes_knowledge_handlers as hh;

const PREFIX: &str = "/api/v1/knowledge";

/// GET dispatcher. Returns `Some` for any path under
/// `/api/v1/knowledge[...]`; `None` lets the parent dispatcher fall
/// through to other GET arms.
pub(super) async fn dispatch_get_api(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    let route = parse_route(req, "GET ")?;
    // Reads are floor=Guest; a deny here only happens in
    // production-mode + no session JWT, which yields a 403 the SPA
    // can route on the same way it does for any other guest refusal.
    if let Some(resp) = enforce_role(req, Action::KnowledgeRead) {
        return Some(resp);
    }
    let store = match store {
        Some(s) => s,
        None => return Some(super::store_unavailable()),
    };
    let tenant = match resolve_tenant(store).await {
        Ok(t) => t,
        Err(resp) => return Some(resp),
    };
    Some(match route {
        Route::List { category } => hh::handle_list(store, tenant, category).await,
        Route::Get { slug } => hh::handle_get(store, tenant, slug).await,
        Route::Revisions { slug } => hh::handle_revisions(store, tenant, slug).await,
        _ => not_found("unknown knowledge route"),
    })
}

/// POST dispatcher.
pub(super) async fn dispatch_post(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    let route = parse_route(req, "POST ")?;
    if let Some(resp) = enforce_role(req, Action::KnowledgeWrite) {
        return Some(resp);
    }
    let store = match store {
        Some(s) => s,
        None => return Some(super::store_unavailable()),
    };
    let tenant = match resolve_tenant(store).await {
        Ok(t) => t,
        Err(resp) => return Some(resp),
    };
    let actor = principal_actor(req);
    let body = super::body_from_request(req);
    Some(match route {
        Route::Create => hh::handle_create(store, tenant, body, &actor).await,
        _ => not_found("unknown knowledge route"),
    })
}

/// PUT dispatcher.
pub(super) async fn dispatch_put(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    let route = parse_route(req, "PUT ")?;
    if let Some(resp) = enforce_role(req, Action::KnowledgeWrite) {
        return Some(resp);
    }
    let store = match store {
        Some(s) => s,
        None => return Some(super::store_unavailable()),
    };
    let tenant = match resolve_tenant(store).await {
        Ok(t) => t,
        Err(resp) => return Some(resp),
    };
    let actor = principal_actor(req);
    let body = super::body_from_request(req);
    Some(match route {
        Route::Update { slug } => hh::handle_update(store, tenant, slug, body, &actor).await,
        _ => not_found("unknown knowledge route"),
    })
}

/// DELETE dispatcher.
pub(super) async fn dispatch_delete(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    let route = parse_route(req, "DELETE ")?;
    if let Some(resp) = enforce_role(req, Action::KnowledgeDelete) {
        return Some(resp);
    }
    let store = match store {
        Some(s) => s,
        None => return Some(super::store_unavailable()),
    };
    let tenant = match resolve_tenant(store).await {
        Ok(t) => t,
        Err(resp) => return Some(resp),
    };
    let actor = principal_actor(req);
    Some(match route {
        Route::Delete { slug } => hh::handle_delete(store, tenant, slug, &actor).await,
        _ => not_found("unknown knowledge route"),
    })
}

// ── Route parsing ────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
pub(super) enum Route<'a> {
    List { category: Option<&'a str> },
    Get { slug: &'a str },
    Revisions { slug: &'a str },
    Create,
    Update { slug: &'a str },
    Delete { slug: &'a str },
}

pub(super) fn parse_route<'a>(req: &'a str, verb: &str) -> Option<Route<'a>> {
    let after_verb = req.strip_prefix(verb)?;
    let path_with_query = after_verb.split_whitespace().next()?;
    let (path, query) = path_with_query.split_once('?').unwrap_or((path_with_query, ""));
    let rest = path.strip_prefix(PREFIX)?;
    let tail = rest.trim_start_matches('/');
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
        [] if is_get => Some(Route::List {
            category: extract_category_filter(query),
        }),
        [] if is_post => Some(Route::Create),
        [slug] if is_get && !slug.is_empty() => Some(Route::Get { slug }),
        [slug] if is_put && !slug.is_empty() => Some(Route::Update { slug }),
        [slug] if is_delete && !slug.is_empty() => Some(Route::Delete { slug }),
        [slug, "revisions"] if is_get && !slug.is_empty() => Some(Route::Revisions { slug }),
        _ => None,
    }
}

/// Pull `category=<value>` out of a raw query string. Returns the
/// first matching segment so duplicate keys collapse to "first wins".
fn extract_category_filter(query: &str) -> Option<&str> {
    if query.is_empty() {
        return None;
    }
    for pair in query.split('&') {
        let (k, v) = pair.split_once('=')?;
        if k == "category" && !v.is_empty() {
            return Some(v);
        }
    }
    None
}

// ── Auth + tenant helpers ──────────────────────────────────────

/// Extract a stable actor handle from the `X-Argo-Principal` header.
/// The bearer-auth gate upstream already validated the header shape,
/// so we keep the parse forgiving: anything that comes through goes
/// into the audit row verbatim.
fn principal_actor(req: &str) -> String {
    let headers = req
        .split_once("\r\n\r\n")
        .map_or(req, |(headers, _body)| headers);
    for line in headers.lines().skip(1) {
        let Some((k, v)) = line.split_once(':') else {
            continue;
        };
        if k.eq_ignore_ascii_case("X-Argo-Principal") {
            return v.trim().to_string();
        }
    }
    String::new()
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

pub(super) fn conflict(message: &str) -> (&'static str, String) {
    (
        "409 Conflict",
        json!({"error": {"code": "conflict", "message": message}}).to_string(),
    )
}

pub(super) fn internal(message: &str) -> (&'static str, String) {
    (
        "500 Internal Server Error",
        json!({"error": {"code": "internal", "message": message}}).to_string(),
    )
}

#[cfg(test)]
#[path = "routes_knowledge_tests.rs"]
mod routes_knowledge_tests;
