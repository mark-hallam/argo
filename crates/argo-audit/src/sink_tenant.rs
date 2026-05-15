//! Tenant identifiers and payload stamping for audit writes.

/// Default tenant id used by [`crate::AuditSink::new`] when no explicit id
/// is supplied (Stream `mission-enterprise-cross-tenant-audit`). New rows lose
/// the `__legacy__` sentinel and self-name under one stable workspace bucket;
/// the audit feed's default tenant unions this constant into its read scope so
/// single-tenant deployments behave identically to pre-tenant audits. Callers
/// that know their workspace tenant should use [`crate::AuditSink::new_with_tenant`]
/// or [`crate::AuditSink::write_with_tenant`]; `argo-config` exposes
/// `workspace_tenant_id` for the canonical resolution path.
pub const DEFAULT_AUDIT_TENANT_ID: &str = "ws-default";

/// Tenant id for system-owned audit rows that are not bound to one tenant.
///
/// Use this for background sweepers, watchdogs, periodic maintenance, and
/// operator/admin commands that operate across tenants or before a workspace
/// tenant can be selected. Prefer a real tenant id whenever local scope already
/// has one.
pub const SYSTEM_AUDIT_TENANT_ID: &str = "system";

/// Tenant id for pre-authentication audit rows.
///
/// Use this only while no authenticated principal or tenant-scoped session is
/// available, such as login, SSO, registration, and failed authentication
/// exchanges. Once a request resolves a tenant or principal, stamp that specific
/// tenant instead.
pub const ANONYMOUS_AUDIT_TENANT_ID: &str = "anonymous";

/// Stamp `tenant_id` into a payload at the publish edge (Stream
/// `mission-enterprise-cross-tenant-audit`). Objects gain a top-level
/// `"tenant_id"` key; non-object payloads are rewritten to
/// `{ "_": <original>, "tenant_id": <id> }` so the column-vs-payload invariant
/// holds for every row. An existing `tenant_id` key in an object payload is
/// overwritten because the sink is authoritative.
pub(crate) fn stamp_tenant_id(payload: serde_json::Value, tenant_id: &str) -> serde_json::Value {
    if let serde_json::Value::Object(mut map) = payload {
        map.insert(
            "tenant_id".to_string(),
            serde_json::Value::String(tenant_id.to_string()),
        );
        serde_json::Value::Object(map)
    } else {
        let mut map = serde_json::Map::new();
        map.insert("_".to_string(), payload);
        map.insert(
            "tenant_id".to_string(),
            serde_json::Value::String(tenant_id.to_string()),
        );
        serde_json::Value::Object(map)
    }
}
