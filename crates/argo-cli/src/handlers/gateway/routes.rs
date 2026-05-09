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
use super::rate_limit;
use super::routes_auth::{check_bearer_auth, is_public_get};
use super::routes_dispatch_get::dispatch_get;
use super::routes_dispatch_get_admin::dispatch_get_admin;
use super::routes_dispatch_post::dispatch_post;
use super::routes_peer::dispatch as dispatch_peer;
use super::routes_session_get::dispatch_session_get;
use super::shutdown::{shutting_down_response, ShutdownController};
use argo_store::SessionStore;
use std::sync::Arc;

pub(super) async fn dispatch_route(
    req: &str,
    store: Option<&Arc<SessionStore>>,
    shutdown: &ShutdownController,
) -> (&'static str, String) {
    // Race window: SIGTERM may have flipped the shutdown flag while
    // this connection was already accepted+read. Reject with the
    // documented 503 body so external clients (k8s readiness probes,
    // load balancers) can back off cleanly during the drain.
    if shutdown.is_shutting_down() {
        return shutting_down_response();
    }
    // Per-route, per-tenant token-bucket gate
    // (claude-rate-limit-per-route). Independent + additive to the
    // per-IP limiter (#1051) which runs upstream in `boot::start`.
    if let Some(resp) = rate_limit_check(req) {
        return resp;
    }
    // Drop-guard increments the in-flight counter for the lifetime of
    // this future and decrements on any return path (including panic
    // unwind). The drain loop blocks on this counter.
    let _guard = shutdown.request_guard();
    // Peer mesh runs ahead of the operator-facing bearer gate: it
    // carries its own ARGO_PEER_SECRET-bearer check, distinct from
    // ARGO_GATEWAY_BEARER_TOKEN. Routing /peer/* through the operator
    // gate would require peer hosts to know both secrets.
    if let Some(resp) = dispatch_peer(req, store).await {
        return resp;
    }
    // Bearer-token gate on every non-allowlisted GET. POST has its own
    // gate inside `dispatch_post` (#946); allowlist members (`/`,
    // `/health[/...]`, `/version`, `/routes`) stay anonymous so liveness
    // probes + load balancers + catalog-discovery keep working.
    if req.starts_with("GET ") && !is_public_get(req) {
        if let Some(resp) = check_bearer_auth(req) {
            return resp;
        }
    }
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

/// Apply the per-route, per-tenant rate limit. Returns `Some(resp)`
/// when the bucket is empty (caller should short-circuit), `None` when
/// the request may proceed. The 429 body is built inline so that the
/// `error: rate_limited` shape stays distinguishable from
/// `error: quota_exceeded` (the separate
/// `claude-tenant-quota-enforcement-stricter` brief).
fn rate_limit_check(req: &str) -> Option<(&'static str, String)> {
    let limiter = rate_limit::global()?;
    let route = rate_limit::classify_request(req);
    let tenant = rate_limit::extract_tenant(req);
    match limiter.acquire(&route, &tenant) {
        Ok(()) => None,
        Err(retry) => {
            tracing::warn!(
                target: "gateway::rate_limit",
                route = %route,
                tenant = %tenant,
                retry_after_ms = retry.retry_after_ms,
                "per-route rate limit triggered"
            );
            let body = format!(
                "{{\"error\":\"rate_limited\",\"scope\":\"route\",\"retry_after_ms\":{}}}",
                retry.retry_after_ms
            );
            Some(("429 Too Many Requests", body))
        }
    }
}
