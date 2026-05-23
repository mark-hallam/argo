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
    // `mvp-gateway-request-audit`: capture wall-clock around the
    // inner dispatcher so every non-skip-list response — including
    // 401/403/5xx — appends one `http.request` row to the audit
    // chain. Short-circuits cleanly when the gateway booted without
    // a store, or when the path matches `/health*` / `/metrics*`.
    let started = std::time::Instant::now();
    // mission-gateway-cors-preflight: OPTIONS on `/api/v1/*` is a
    // pure header check that carries no auth and must run ahead of
    // the bearer / cookie / origin / csrf middleware (browsers send
    // preflight with no credentials). Every other path falls
    // through to the inner dispatcher, then has CORS headers
    // stamped onto its status line via `apply_cors`.
    let result = if super::cors::is_preflight(req) {
        super::cors::preflight_response(req)
    } else {
        apply_cors(req, dispatch_route_inner(req, store, shutdown).await)
    };
    super::routes_request_audit::record(req, store, &result, started.elapsed()).await;
    result
}

/// Stamp CORS response headers on `/api/v1/*` responses by looking
/// up the (status, request-Origin) pair in the global registry.
/// Non-`/api/v1/*` paths and requests from non-allowlisted Origins
/// pass through unchanged — the latter is the fail-closed default.
fn apply_cors(
    req: &str,
    (status, body): (&'static str, String),
) -> (&'static str, String) {
    if super::cors::is_api_v1(req) {
        (super::cors::enhance_status(req, status), body)
    } else {
        (status, body)
    }
}

async fn dispatch_route_inner(
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
    if let Some(resp) = super::region_gate::reject_cross_region(req, store).await {
        return resp;
    }
    // Drop-guard increments the in-flight counter for the lifetime of
    // this future and decrements on any return path (including panic
    // unwind). The drain loop blocks on this counter.
    let _guard = shutdown.request_guard();
    // Origin allow-list + CSRF double-submit
    // (Stream `claude-rebase-pr-1835-cookies`). Origin-check runs
    // ahead of the auth-magic-link branch so a foreign-origin POST
    // to `/api/v1/auth/signup` is rejected before a magic-link is
    // minted. CSRF skips the anonymous auth endpoints (no session
    // cookie yet).
    if let Some(resp) = super::middleware_origin_check::check_origin(req) {
        return resp;
    }
    if let Some(resp) = super::middleware_csrf::check_csrf(req) {
        return resp;
    }
    // Cookie-JWT verify gate (mission-revocation-with-verify-middleware-v1x).
    // Runs AFTER origin + csrf so cross-origin / CSRF-missing requests are
    // rejected before the more expensive jsonwebtoken decode + revoked_tokens
    // lookup. Treats a missing cookie as fall-through so the existing
    // bearer-token gate still handles non-browser API callers. Magic-link
    // verify (`/auth/verify`), signup (`/auth/signup`), and the WS Ed25519
    // path are exempted upstream — `/auth/verify` and `/auth/signup` are
    // anonymous-by-design (no session yet), and ws_upgrade owns its own
    // Ed25519-bound auth in `boot_router::try_dispatch_gates`. The logout
    // endpoint (`/auth/logout`) is also exempt because it owns its own
    // decode path in `boot_router` to emit the cookie-clear `Set-Cookie`
    // header; it MUST remain reachable even when the cookie is already
    // in `revoked_tokens` so a confused client can still reach the
    // cookie-clear path.
    if let Some(resp) = check_session_cookie(req, store).await {
        return resp;
    }
    // Peer mesh runs ahead of the operator-facing bearer gate: it
    // carries its own ARGO_PEER_SECRET-bearer check, distinct from
    // ARGO_GATEWAY_BEARER_TOKEN. Routing /peer/* through the operator
    // gate would require peer hosts to know both secrets.
    if let Some(resp) = dispatch_peer(req, store).await {
        return resp;
    }
    // Stripe webhook (claude-stripe-subscription-integration). The
    // request is authenticated by `Stripe-Signature` HMAC, not the
    // operator bearer — so it MUST run ahead of `check_bearer_auth`.
    if req.starts_with("POST /api/v1/billing/webhook ")
        || req.starts_with("POST /api/v1/billing/webhook?")
    {
        return super::billing::webhook_response(req, store).await;
    }
    // Magic-link signup (claude-magic-link-auth-backend). Mounted
    // ahead of the bearer-auth gate so anonymous browsers can sign
    // up at `app.skycot.com/signup`. The companion `verify` endpoint
    // is handled by `boot_router::try_dispatch_gates` because it
    // owns its own TcpStream to emit `Set-Cookie: __Host-session=...`
    // (mission-magic-link-v1x-hardening).
    if let Some(resp) = super::auth_magic_link::dispatch(req, store).await {
        return resp;
    }
    // Demo-mode magic-link bypass
    // (demo-ops-add-magic-link-bypass-for-recording). Active only
    // when `ARGO_DEMO_MODE=1` is set at gateway startup; mounted
    // ahead of bearer-auth so the SPA's signup form can call it
    // anonymously. The startup-check refuses to bind when this
    // env var co-exists with bearer-auth (see startup_check.rs).
    if let Some(resp) = super::demo_mode::dispatch(req, store).await {
        return resp;
    }
    // CSRF token-fetch surface — same-origin XHR endpoint that
    // mirrors the `__Host-csrf` cookie value back as JSON. Anonymous
    // (no bearer gate) so the SPA can call it on first-load.
    if let Some(resp) = super::routes_auth_csrf::dispatch(req).await {
        return resp;
    }
    // Skill marketplace browsing stays anonymous so prospects can
    // inspect the catalog. Marketplace install POSTs are mutating
    // actions, so the marketplace dispatcher handles them only after
    // anonymous GET routing; `install_action` accepts either the
    // configured bearer token or a verified JWT before side effects.
    if super::marketplace::is_marketplace_get(req) {
        if let Some(resp) = super::marketplace::dispatch(req, store).await {
            return resp;
        }
    }
    if super::bootstrap_token::is_wizard_request(req) {
        if req.starts_with("OPTIONS ") {
            return super::bootstrap_token::wizard_options_response();
        }
        if let Some(resp) = super::bootstrap_token::check_bootstrap_token(req) {
            return resp;
        }
        if let Some(resp) = dispatch_get(req, store).await {
            return resp;
        }
        if let Some(resp) = dispatch_post(req, store).await {
            return resp;
        }
        return ("404 Not Found", "{\"error\":\"not found\"}".to_string());
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
    // Legacy server-rendered dashboards (`claude-web-dashboard-mvp`
    // + `claude-dashboard-fleet-view-html`). Mounted AFTER the
    // bearer-auth gate so spend ledger, audit records, pending
    // approvals, sessions, and worker IDs are never reachable
    // without a valid `Authorization: Bearer <token>` when
    // `ARGO_GATEWAY_BEARER_TOKEN` is configured
    // (mission-audit-gateway-dashboard-auth-gate).
    if super::dashboard_fleet::is_fleet(req) {
        return super::dashboard_fleet::handle(store).await;
    }
    if super::routes_dashboard::is_dashboard(req) {
        if let Some(resp) = super::routes_dashboard::dispatch(req, store).await {
            return resp;
        }
    }
    if let Some(resp) = dispatch_get_admin(req, store).await {
        return resp;
    }
    if let Some(resp) = dispatch_get(req, store).await {
        return resp;
    }
    // web-gateway-endpoints-audit-mutations-batch — signed
    // evidence-bundle download. Mounted alongside the other GETs so
    // the HMAC-signed URL returned by `POST /api/v1/audit/export`
    // resolves without going through `dispatch_session_get`.
    if let Some(resp) = super::routes_audit_export::try_dispatch_get(req, store).await {
        return resp;
    }
    if super::marketplace::is_marketplace(req) {
        if let Some(resp) = super::marketplace::dispatch(req, store).await {
            return resp;
        }
    }
    // Bearer-gated billing POSTs (checkout, portal). Auth runs here
    // because dispatch_post (which carries the standard POST bearer
    // gate) hasn't been reached yet.
    if req.starts_with("POST /api/v1/billing/checkout ")
        || req.starts_with("POST /api/v1/billing/checkout?")
        || req.starts_with("POST /api/v1/billing/portal ")
        || req.starts_with("POST /api/v1/billing/portal?")
    {
        if let Some(resp) = check_bearer_auth(req) {
            return resp;
        }
        if req.starts_with("POST /api/v1/billing/checkout") {
            return super::billing::checkout_response(req, store).await;
        }
        return super::billing::portal_response(req, store).await;
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

/// Apply the cookie-JWT verify gate. Returns `Some(401-resp)` when a
/// `__Host-session` cookie is present but invalid / expired / revoked;
/// returns `None` to let the request fall through to the bearer-token
/// gate (also `None` when no cookie is present). Live behind the
/// `middleware_session_verify` module — mounted ahead of the bearer
/// gate so a revoked cookie cannot side-step into bearer auth, but
/// fall-through preserves non-browser callers.
///
/// Exempt paths (per the brief):
/// - `POST /api/v1/auth/signup` — anonymous sign-up; a returning
///   user with a stale cookie must still be able to re-enrol.
/// - CSRF token-fetch (`/api/v1/auth/csrf`) — anonymous so the SPA
///   can read the double-submit token on first load.
/// - `OPTIONS *` — CORS preflights carry no auth.
///
/// Magic-link verify (`/api/v1/auth/verify`) and the WS Ed25519 path
/// are upstream in `boot_router::try_dispatch_gates`, so they never
/// reach this gate.
async fn check_session_cookie(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    if is_session_verify_exempt(req) {
        return None;
    }
    use super::middleware_session_verify::{verify_session_cookie, SessionVerifyOutcome};
    match verify_session_cookie(req, store).await {
        SessionVerifyOutcome::NoCookie | SessionVerifyOutcome::Valid(_) => None,
        SessionVerifyOutcome::Reject(reason) => Some((
            "401 Unauthorized",
            format!("{{\"error\":\"{reason}\"}}"),
        )),
    }
}

/// Anonymous-by-design endpoints that must not be 401d by the
/// cookie-verify gate even if a stale `__Host-session` cookie is
/// present.
fn is_session_verify_exempt(req: &str) -> bool {
    req.starts_with("OPTIONS ")
        || req.starts_with("POST /api/v1/auth/signup ")
        || req.starts_with("POST /api/v1/auth/signup?")
        || req.starts_with("GET /api/v1/auth/csrf ")
        || req.starts_with("GET /api/v1/auth/csrf?")
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
