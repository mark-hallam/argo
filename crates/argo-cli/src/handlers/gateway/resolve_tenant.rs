//! Canonical per-request workspace-tenant resolver (sprint WS1).
//!
//! Before this module, ~24 gateway route files each carried a private
//! `resolve_workspace_tenant` copy that resolved the tenant from the
//! gateway *process's working directory* (`ensure_workspace(cwd)`).
//! That meant every signed-in user was resolved to the ONE cwd tenant
//! — the seeded demo workspace — and a stranger who signed up landed
//! in shared demo data instead of their own empty private workspace.
//!
//! [`resolve_tenant_for_request`] is the single fix: it prefers the
//! user-bound tenant carried in the `__Host-session` JWT (minted by
//! `auth_magic_link::mint_jwt`, threaded through
//! `tenant_provision::provision_for_user`), falling back to the
//! explicit override header and finally the cwd/anonymous tenant ONLY
//! when there is no session. The demo-bypass + truly-anonymous public
//! endpoints keep working unchanged.
//!
//! ## Why the JWT tenant needs no workspace-row lookup
//!
//! `provision_for_user` writes the workspace `root_path` as the
//! virtual `tenant://<id>/<ws>` string, which `ensure_workspace`
//! (built for filesystem paths) can never match — so resolving the
//! provisioned tenant by `ensure_workspace` would *create a brand-new
//! tenant* on every request. The fix is to NOT go through
//! `ensure_workspace` at all for the session path: the read routes
//! filter their SQL by `tenant_id` (e.g. `team_roster::list_teams`
//! binds `WHERE tenant_id = ?`), so returning the JWT-claim tenant
//! id directly is exactly the tenant the provisioned rows were
//! written under. Reads and writes therefore agree on the tenant
//! without ever consulting the mismatched `root_path`.

use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use uuid::Uuid;

use super::middleware_session_verify::{decode_session_bearer, decode_session_cookie, DecodeOutcome};

/// Resolve the workspace tenant this request should read against.
///
/// Precedence (highest first):
///
/// 1. A valid `__Host-session` cookie JWT carrying a `tnt` (tenant_id)
///    claim → that user-bound tenant. **This is the WS1 unlock** —
///    signed-in users read their own workspace, not the cwd demo tenant.
/// 2. A valid `Authorization: Bearer <session-jwt>` carrying a `tnt`
///    claim → that user-bound tenant. The SPA's SERVER-SIDE proxy route
///    handlers (Run-now, workflow-propose, …) forward the signed-in
///    user's session JWT as a bearer rather than the cookie; without this
///    branch they resolved the cwd/anonymous tenant — a WS1 cross-tenant
///    hazard. The bearer IS the same gateway-signed session JWT as the
///    cookie, so it only ever resolves the caller's OWN tenant.
/// 3. An explicit `?tenant=` query param / `X-Tenant-Id` /
///    `X-Argo-Tenant-Id` header → that tenant. Preserves the audit-feed
///    override contract from #2589 (cross-tenant viewing stays gated by
///    capability at the call sites that allow it). Reachable only on the
///    no-session path, so an authenticated user (cookie OR bearer) cannot
///    use it to escape their own tenant — a session branch above wins.
/// 4. The cwd/config workspace tenant via `ensure_workspace(cwd)` — the
///    demo / anonymous fallback, behaviourally unchanged from the old
///    per-file `resolve_workspace_tenant`.
pub(super) async fn resolve_tenant_for_request(
    req: &str,
    store: &SessionStore,
) -> ArgoResult<TenantId> {
    if let Some(tenant) = session_tenant_id(req) {
        return Ok(tenant);
    }
    if let Some(tenant) = bearer_session_tenant_id(req) {
        return Ok(tenant);
    }
    if let Some(tenant) = explicit_tenant_override(req) {
        if anon_tenant_override_open() {
            return Ok(tenant);
        }
        // WS1 (`fix-worker-read-routes-anon-tenant-override`, from the
        // #2858 review): with the perimeter CLOSED, an UNAUTHENTICATED
        // caller's `?tenant=`/`X-Tenant-Id` must not select an arbitrary
        // tenant — worker-read routes (detail/activity/overview/file-list)
        // ride this resolver un-gated, so honoring it was a cross-tenant
        // metadata read. Fall through to the cwd/demo tenant instead: the
        // caller can only ever see the anonymous lane's own data.
        tracing::warn!(
            target: "gateway::resolve_tenant",
            "anonymous tenant override ignored: perimeter closed (set gateway.auth_bypass for the dev/demo lane)",
        );
    }
    cwd_tenant(store).await
}

/// Whether the anonymous explicit `?tenant=`/`X-Tenant-Id` override lane
/// is open. Mirrors `role_gate::role_header_trusted`'s composition:
/// release builds fail CLOSED unless the operator explicitly opened the
/// perimeter (`gateway.auth_bypass`, the #2589 demo/no-auth lane); test
/// builds keep the legacy `!is_production_mode()` heuristic so existing
/// fixtures (which pass `X-Tenant-Id` without a session) keep working,
/// while production-mode tests still exercise the closed path.
/// `&&`-order puts `cfg!(test)` first so `perimeter_open()` (a config
/// read) is skipped on the common non-prod test path.
fn anon_tenant_override_open() -> bool {
    (cfg!(test) && !super::routes_auth_production_mode::is_production_mode())
        || super::routes_auth_production_mode::perimeter_open()
}

/// Resolve the tenant a **work-path audit write** must be stamped under
/// so the row round-trips into `GET /api/v1/audit`.
///
/// The feed's default scope resolves through
/// [`resolve_tenant_for_request`] (via
/// `super::audit_query::resolve_workspace_default`). A writer that
/// instead resolved its tenant through `super::rate_limit::extract_tenant`
/// stamped rows under the config-default / `"anonymous"` tenant for a
/// signed-in caller, so the feed — scoped to that caller's
/// `__Host-session` JWT tenant — never read them back (the work-path
/// audit round-trip gap). This helper makes writers agree with the feed:
/// identical JWT → explicit header / `?tenant=` → cwd precedence. It
/// falls back to [`super::rate_limit::extract_tenant`] only when the
/// canonical resolver itself errors (e.g. `current_dir` failure on the
/// cwd branch), so an audit row is never silently dropped to an empty
/// tenant.
///
/// All work-path audit writers that previously inlined
/// `extract_tenant(req)` (pitch ask + the three genesis routes) call
/// this; the GTM-draft and marketing-draft writers already route their
/// tenant through [`resolve_tenant_for_request`] directly.
pub(super) async fn resolve_audit_write_tenant(req: &str, store: &SessionStore) -> String {
    match resolve_tenant_for_request(req, store).await {
        Ok(tenant) => tenant.to_string(),
        Err(_) => super::rate_limit::extract_tenant(req),
    }
}

/// The tenant id carried in a valid `__Host-session` cookie JWT, if
/// present. Uses [`decode_session_cookie`] (signature + `exp`), not the
/// revocation-aware verify: the session-verify middleware already
/// rejects revoked/invalid cookies upstream of any route handler, and
/// even a stale cookie only ever resolves to its *own* tenant — no
/// privilege escalation is possible here.
///
/// Returns `None` when there is no cookie, the signature/exp fails, the
/// `tnt` claim is absent (user mid-onboarding before workspace pick), or
/// the claim is not a parseable UUID.
fn session_tenant_id(req: &str) -> Option<TenantId> {
    tnt_from_decoded(decode_session_cookie(req))
}

/// The tenant id carried in a valid `Authorization: Bearer <session-jwt>`
/// header, if present. Mirrors [`session_tenant_id`] but reads the bearer
/// instead of the cookie: the SPA's server-side proxy route handlers
/// (Run-now, workflow-propose, …) forward the signed-in user's session JWT
/// as a bearer, not the `__Host-session` cookie. Without this branch those
/// proxies fell through to the cwd/anonymous tenant — a WS1 cross-tenant
/// hazard (a signed-in user's action ran against the shared demo tenant).
/// The bearer IS the same gateway-signed session JWT as the cookie, so it
/// only ever resolves the caller's OWN tenant; it sits ABOVE the explicit
/// `X-Tenant-Id` override exactly like the cookie.
fn bearer_session_tenant_id(req: &str) -> Option<TenantId> {
    tnt_from_decoded(decode_session_bearer(req))
}

/// Extract a parseable `tnt` tenant id from a session-JWT decode outcome,
/// or `None` for no-token / reject / absent-or-unparseable `tnt`. Shared
/// by the cookie and bearer session branches so they stay byte-identical.
fn tnt_from_decoded(outcome: DecodeOutcome) -> Option<TenantId> {
    match outcome {
        DecodeOutcome::Decoded(claims) => Uuid::parse_str(&claims.tnt?).ok().map(TenantId),
        DecodeOutcome::NoCookie | DecodeOutcome::Reject(_) => None,
    }
}

/// An explicit tenant override from the request, if present. Mirrors
/// `rate_limit::extract_tenant`'s precedence (`?tenant=` →
/// `X-Tenant-Id` → legacy `X-Argo-Tenant-Id`) but, unlike that helper,
/// returns `None` rather than the config-default / `"anonymous"`
/// sentinel — the cwd fallback owns that case here.
fn explicit_tenant_override(req: &str) -> Option<TenantId> {
    let raw = query_tenant(req).or_else(|| header_tenant(req))?;
    Uuid::parse_str(&raw).ok().map(TenantId)
}

/// `?tenant=<value>` from the request-line target, non-empty.
fn query_tenant(req: &str) -> Option<String> {
    let target = req.lines().next()?.split_whitespace().nth(1)?;
    let (_, qs) = target.split_once('?')?;
    qs.split('&').find_map(|kv| {
        let (k, v) = kv.split_once('=')?;
        (k == "tenant" && !v.is_empty()).then(|| v.to_string())
    })
}

/// `X-Tenant-Id` (then legacy `X-Argo-Tenant-Id`) header value,
/// non-empty. Header names are case-insensitive (RFC 7230).
fn header_tenant(req: &str) -> Option<String> {
    let header_end = req.find("\r\n\r\n").unwrap_or(req.len());
    for name in ["x-tenant-id", "x-argo-tenant-id"] {
        for line in req[..header_end].split("\r\n").skip(1) {
            let Some((key, value)) = line.split_once(':') else {
                continue;
            };
            if key.trim().eq_ignore_ascii_case(name) {
                let value = value.trim();
                if !value.is_empty() {
                    return Some(value.to_string());
                }
            }
        }
    }
    None
}

/// The cwd/config workspace tenant — the demo / anonymous fallback.
/// Identical behaviour to the per-file `resolve_workspace_tenant`
/// copies this module replaces: idempotent `ensure_workspace` keyed
/// by the gateway process's current directory.
async fn cwd_tenant(store: &SessionStore) -> ArgoResult<TenantId> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (tenant, _ws) = store.ensure_workspace(name, &root).await?;
    Ok(tenant)
}

#[cfg(test)]
#[path = "resolve_tenant_tests.rs"]
mod tests;
