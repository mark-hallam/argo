//! Tenant-scope helpers for `GET /api/v1/audit`.

use super::role_gate::{enforce_role, Action};
use super::routes_audit_feed::FeedParams;
use argo_audit::ExportedRow;
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
}

pub(super) fn resolve_tenant_scope(req: &str, params: &FeedParams) -> TenantScope {
    let header_tenant = header_value(req, HEADER_WORKSPACE_TENANT);
    let workspace_tenant =
        header_tenant.unwrap_or_else(|| DEFAULT_WORKSPACE_TENANT_FALLBACK.to_string());
    // Cross-tenant audit is gated by the [`Action::AuditCrossTenant`]
    // capability (owner-only). The legacy `X-Argo-Audit-Cross-Tenant`
    // header trust path is gone — `enforce_role` derives the role
    // from the verified session JWT and ignores any role-stamping
    // headers in production. See wave-3 RBAC audit finding #5.
    let cap_granted = enforce_role(req, Action::AuditCrossTenant).is_none();
    match params.as_tenant.as_deref() {
        Some(other) if cap_granted => tenant_scope(other, other == LEGACY_AUDIT_TENANT_ID),
        _ if workspace_tenant == LEGACY_AUDIT_TENANT_ID => {
            tenant_scope(&workspace_tenant, cap_granted)
        }
        _ => tenant_scope(&workspace_tenant, false),
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
