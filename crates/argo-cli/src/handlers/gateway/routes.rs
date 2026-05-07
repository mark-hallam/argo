//! HTTP route dispatch entry point. Per-method dispatch is split
//! into siblings to keep this file under the 500-LOC budget:
//!   - `routes_dispatch_get_admin` — introspection/health/catalog
//!     GETs (`/`, `/version`, `/health/*`, `/doctor`, `/db/*`,
//!     `/metrics/json`, `/routes`,
//!     `/audit{,/breaks,/recent,/stream,/count}`,
//!     `/search{,/count}`, `/workspace/{info,artifacts}`).
//!   - `routes_dispatch_get` — per-resource GETs (sessions, skills,
//!     memory, providers, tools, verticals, cron, forge/runs,
//!     audit/sessions/<id>; also routes /costs/*, /forge/runs/<id>/*,
//!     and rollups via their own sub-dispatchers).
//!   - `routes_dispatch_post` — every POST verb.
//!   - `routes_session_get` — GET /sessions/<uuid>/*.
//!
//! Each handler returns a `(status_line, body)` pair the listener
//! loop formats into a single-line HTTP/1.1 response.
use super::routes_dispatch_get::dispatch_get;
use super::routes_dispatch_get_admin::dispatch_get_admin;
use super::routes_dispatch_post::dispatch_post;
use super::routes_session_get::dispatch_session_get;
use argo_store::SessionStore;
use std::sync::Arc;

pub(super) async fn dispatch_route(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    if let Some(resp) = dispatch_get_admin(req, store).await {
        return resp;
    }
    if let Some(resp) = dispatch_get(req, store).await {
        return resp;
    }
    if let Some(resp) = dispatch_post(req, store).await {
        return resp;
    }
    // GET /sessions/<uuid>/{spend,spend/total,messages,messages/last,skills,judge}
    // and bare GET /sessions/<uuid> are all dispatched in one helper.
    if let Some(resp) = dispatch_session_get(req, store).await {
        return resp;
    }
    ("404 Not Found", "{\"error\":\"not found\"}".to_string())
}
