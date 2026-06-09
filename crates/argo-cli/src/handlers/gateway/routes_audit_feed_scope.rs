//! Tenant-scope helpers for `GET /api/v1/audit`.

use super::role_gate::{enforce_role, resolve_role, Action};
use super::routes_audit_feed::FeedParams;
use argo_audit::ExportedRow;
use argo_identity::user_tenant::UserTenantRole;
use argo_redact::RedactionPolicy;
use serde_json::Value;

const DEFAULT_WORKSPACE_TENANT_FALLBACK: &str = "ws-default";
const HEADER_WORKSPACE_TENANT: &str = "x-argo-tenant-id";
const LEGACY_AUDIT_TENANT_ID: &str = "__legacy__";
const DENIED_LEGACY_TENANT_ID: &str = "__denied_legacy_scope__";

/// Tenant-scoping decision for one request.
#[derive(Debug, Clone)]
pub(super) struct TenantScope {
    /// The tenant id whose rows we surface.
    effective: String,
    /// Whether to surface every tenant's rows.
    include_all_tenants: bool,
    /// Whether to surface rows whose payload predates tenant stamping.
    include_legacy: bool,
    /// Whether the resolved scope is the caller's OWN workspace tenant
    /// (an absent or self-targeted `as_tenant`), as opposed to a
    /// cross-tenant `as_tenant=<other>` override or the legacy-admin
    /// all-tenants lane. Read by [`is_own_tenant`] to decide whether an
    /// owner may see their own rows unredacted.
    own_tenant: bool,
}

pub(super) fn resolve_tenant_scope(
    req: &str,
    params: &FeedParams,
    workspace_default: &str,
) -> TenantScope {
    let header_tenant = header_value(req, HEADER_WORKSPACE_TENANT);
    // Callers pass the cwd-bound workspace tenant UUID (matches what
    // /team, /skills, /workflows, /verticals resolve). The literal
    // `ws-default` constant is the absolute last resort if the caller
    // could not resolve a workspace (kept here for the call-site
    // fallback below, not as a default).
    let workspace_tenant = header_tenant
        .unwrap_or_else(|| {
            if workspace_default.is_empty() {
                DEFAULT_WORKSPACE_TENANT_FALLBACK.to_string()
            } else {
                workspace_default.to_string()
            }
        });
    // Cross-tenant audit is gated by the [`Action::AuditCrossTenant`]
    // capability (owner-only). The legacy `X-Argo-Audit-Cross-Tenant`
    // header trust path is gone — `enforce_role` derives the role
    // from the verified session JWT and ignores any role-stamping
    // headers in production. See wave-3 RBAC audit finding #5.
    let cap_granted = enforce_role(req, Action::AuditCrossTenant).is_none();
    let mut scope = match params.as_tenant.as_deref() {
        Some(other) if cap_granted => tenant_scope(other, other == LEGACY_AUDIT_TENANT_ID),
        _ if workspace_tenant == LEGACY_AUDIT_TENANT_ID => {
            tenant_scope(&workspace_tenant, cap_granted)
        }
        _ => tenant_scope(&workspace_tenant, false),
    };
    // Own-tenant = NOT the legacy-admin all-tenants lane AND the tenant
    // we resolved is the caller's own workspace tenant. A cross-tenant
    // `as_tenant=<other>` redirects `effective` away from
    // `workspace_tenant`, so this is false there; a denied-legacy scope
    // rewrites `effective` to a sentinel, so it is false there too.
    scope.own_tenant = !scope.include_all_tenants && scope.effective == workspace_tenant;
    scope
}

/// Whether this request reads the caller's OWN workspace tenant (not a
/// cross-tenant override or the legacy-admin all-tenants lane). The
/// feed's redaction-policy selector keys off this so an owner sees their
/// own tenant's rows unredacted while cross-tenant views stay masked.
pub(super) fn is_own_tenant(scope: &TenantScope) -> bool {
    scope.own_tenant
}

/// Select the redaction policy for one audit-feed response.
///
/// A workspace OWNER reading their OWN tenant's rows sees them
/// unredacted (`show_all`) — the buyer who paid for the hash-chained
/// audit must not have their own actions hidden from them. Every other
/// case keeps the strict mask-everything `default()`: a non-owner role
/// on the own tenant, any cross-tenant (`as_tenant=<other>`) view, and
/// the legacy-admin all-tenants lane. `resolve_role` returns `Owner`
/// only for a verified owner session (production JWT or demo-mode
/// owner-mint), so an unauthenticated header-less caller fails closed.
pub(super) fn feed_redaction_policy(req: &str, scope: &TenantScope) -> RedactionPolicy {
    let is_owner = resolve_role(req) == Some(UserTenantRole::Owner);
    if is_owner && is_own_tenant(scope) {
        RedactionPolicy::show_all()
    } else {
        RedactionPolicy::default()
    }
}

pub(super) fn row_in_tenant_scope(row: &ExportedRow, scope: &TenantScope) -> bool {
    if scope.include_all_tenants {
        return true;
    }
    let payload_tenant = row.payload.get("tenant_id").and_then(Value::as_str);
    match payload_tenant {
        Some(t) if t == scope.effective => true,
        Some(_) => false,
        None => scope.include_legacy,
    }
}

fn tenant_scope(effective: &str, include_legacy: bool) -> TenantScope {
    let legacy_admin_lane = effective == LEGACY_AUDIT_TENANT_ID && include_legacy;
    TenantScope {
        effective: if effective == LEGACY_AUDIT_TENANT_ID && !include_legacy {
            DENIED_LEGACY_TENANT_ID.to_string()
        } else {
            effective.to_string()
        },
        include_all_tenants: legacy_admin_lane,
        include_legacy,
        // Set by `resolve_tenant_scope` once the workspace tenant is
        // known; the constructor defaults it to the safe (redacted) value.
        own_tenant: false,
    }
}

fn header_value(req: &str, name_lower: &str) -> Option<String> {
    let headers = req
        .split_once("\r\n\r\n")
        .map_or(req, |(headers, _body)| headers);
    headers.lines().skip(1).find_map(|line| {
        let (name, value) = line.split_once(':')?;
        name.trim()
            .eq_ignore_ascii_case(name_lower)
            .then(|| value.trim().to_string())
    })
}
