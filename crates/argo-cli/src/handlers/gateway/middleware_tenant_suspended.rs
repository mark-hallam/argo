//! Suspended-tenant write lockout (DEC-057 Phase 1).
//!
//! A suspended tenant (set by the platform-staff console) keeps READ access —
//! it can still log in, view its data, and reach billing to pay — but every
//! MUTATING request is refused with `403 account_suspended`. This is the
//! structural enforcement of the `tenant.status` column that previously had no
//! reader: "Suspended → reads OK, writes refused" (see
//! [`argo_identity::tenant::TenantStatus::Suspended`]).
//!
//! Mounted as a single choke-point in [`super::boot_router::try_dispatch_gates`],
//! after the IP / rate-limit gates but BEFORE the streaming / SSE / Genesis
//! peel-offs — so it covers every authenticated mutating route at once
//! (including the `/stream` + `/api/v1/genesis` variants, which `return Some(..)`
//! before `dispatch_routes` is ever reached) rather than relying on each handler
//! to remember the check.

use super::resolve_tenant::resolve_tenant_for_request;
use argo_identity::tenant::TenantStatus;
use argo_identity::tenant_store::get_tenant;
use argo_store::SessionStore;
use std::sync::Arc;

/// Returns `Some(403 account_suspended)` when `req` is a mutating request whose
/// resolved tenant is suspended; `None` to let it proceed. Fails open on any
/// resolution/lookup miss (a request we can't attribute to a suspended tenant
/// is not blocked here — auth gates upstream already rejected anonymous calls).
pub(super) async fn check_tenant_not_suspended(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    if !is_write_lockout_candidate(req) {
        return None;
    }
    let store = store?;
    let tenant = resolve_tenant_for_request(req, store).await.ok()?;
    match get_tenant(store, tenant).await {
        Ok(Some(t)) if t.status == TenantStatus::Suspended => Some((
            "403 Forbidden",
            "{\"error\":{\"code\":\"account_suspended\",\"message\":\"account suspended — writes are refused until reactivated\"}}"
                .to_string(),
        )),
        _ => None,
    }
}

/// Pure predicate: is `req` a mutating request the suspend lockout should
/// consider? Reads (`GET`/`HEAD`) and preflight (`OPTIONS`) always pass. Three
/// lanes a suspended tenant MUST still reach are exempt:
///   - `/api/v1/platform/*` — the staff console acts on OTHER tenants,
///   - `/api/v1/billing/*`  — pay to lift the suspension,
///   - `/api/v1/auth/*`     — logout / session.
pub(super) fn is_write_lockout_candidate(req: &str) -> bool {
    let Some(first_line) = req.lines().next() else {
        return false;
    };
    let mut parts = first_line.split(' ');
    let (Some(method), Some(path)) = (parts.next(), parts.next()) else {
        return false;
    };
    if !matches!(method, "POST" | "PUT" | "PATCH" | "DELETE") {
        return false;
    }
    let path = path.split('?').next().unwrap_or(path);
    !(path.starts_with("/api/v1/platform/")
        || path.starts_with("/api/v1/billing/")
        || path.starts_with("/api/v1/auth/"))
}

#[cfg(test)]
#[path = "middleware_tenant_suspended_tests.rs"]
mod tests;
