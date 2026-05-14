//! Tenant-scope helpers for `GET /api/v1/audit`.

use super::routes_audit_feed::FeedParams;
use argo_audit::ExportedRow;
use serde_json::Value;

const DEFAULT_WORKSPACE_TENANT_FALLBACK: &str = "ws-default";
const HEADER_WORKSPACE_TENANT: &str = "x-argo-tenant-id";
const HEADER_CROSS_TENANT_CAP: &str = "x-argo-audit-cross-tenant";

/// Tenant-scoping decision for one request.
#[derive(Debug, Clone)]
pub(super) struct TenantScope {
    /// The tenant id whose rows we surface.
    effective: String,
    /// Whether to UNION legacy rows into the default workspace view.
    include_legacy: bool,
}

pub(super) fn resolve_tenant_scope(req: &str, params: &FeedParams) -> TenantScope {
    let workspace_tenant = header_value(req, HEADER_WORKSPACE_TENANT)
        .unwrap_or_else(|| DEFAULT_WORKSPACE_TENANT_FALLBACK.to_string());
    let cap_granted = header_value(req, HEADER_CROSS_TENANT_CAP)
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    match params.as_tenant.as_deref() {
        Some(other) if cap_granted => TenantScope {
            effective: other.to_string(),
            include_legacy: false,
        },
        _ => TenantScope {
            effective: workspace_tenant,
            include_legacy: true,
        },
    }
}

pub(super) fn row_in_tenant_scope(row: &ExportedRow, scope: &TenantScope) -> bool {
    let payload_tenant = row.payload.get("tenant_id").and_then(Value::as_str);
    match payload_tenant {
        Some(t) if t == scope.effective => true,
        Some(_) => false,
        None => scope.include_legacy,
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
