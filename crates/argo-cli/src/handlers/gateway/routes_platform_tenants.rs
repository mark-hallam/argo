//! `/api/v1/platform/tenants` — the DEC-057 Phase 1 platform-staff console.
//!
//! Account-level only (NO workspace contents):
//! - `GET  /api/v1/platform/tenants` — cross-tenant account list (id, name,
//!   billing email, lifecycle status, member/worker counts).
//! - `POST /api/v1/platform/tenants/<id>/suspend` — suspend a tenant.
//! - `POST /api/v1/platform/tenants/<id>/reactivate` — lift a suspension.
//!
//! Every route is gated by [`super::platform_staff_gate::enforce_staff`]
//! (default-deny; role taken from the verified session JWT). Suspend/reactivate
//! write a hash-chained audit row to the **affected tenant's** chain so the
//! customer can see the staff action; the account-list read writes a
//! `platform.accounts_viewed` row to the `system` chain. The suspended-tenant
//! *write lockout* itself is enforced upstream in
//! [`super::routes::dispatch_route_inner`].

use super::platform_staff_gate::enforce_staff;
use super::routes_auth_principal::session_principal_id_for_audit;
use super::{body_from_request, store_unavailable};
use argo_audit::{AuditKind, AuditSink, SYSTEM_AUDIT_TENANT_ID};
use argo_identity::platform_staff::StaffAction;
use argo_identity::tenant::{Tenant, TenantStatus};
use argo_identity::tenant_store::{get_tenant, list_tenants, set_tenant_status};
use argo_identity::user_tenant_store::list_users_in_tenant;
use argo_store::tenant_query::tenant_worker_count;
use argo_store::SessionStore;
use argo_types::{ArgoResult, TenantId};
use chrono::Utc;
use serde_json::{json, Value};
use std::sync::Arc;

/// GET fan-out for the platform console. Returns `Some(resp)` on a hit.
pub(super) async fn dispatch_get(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    if req.starts_with("GET /api/v1/platform/tenants ")
        || req.starts_with("GET /api/v1/platform/tenants?")
    {
        return Some(list_response(req, store).await);
    }
    None
}

/// `GET /api/v1/platform/tenants` — cross-tenant account list. Staff-gated
/// (read); audited as `platform.accounts_viewed`.
pub(super) async fn list_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    if let Err(deny) = enforce_staff(req, store, StaffAction::ReadAccounts).await {
        return deny;
    }
    let accounts = match build_accounts(store).await {
        Ok(a) => a,
        Err(e) => return server_error(&format!("list accounts: {e}")),
    };
    audit_view(store, req, accounts.len()).await;
    (
        "200 OK",
        json!({ "accounts": accounts, "count": accounts.len() }).to_string(),
    )
}

/// `POST /api/v1/platform/tenants/<id>/suspend`. Optional body `{ reason }`.
pub(super) async fn post_suspend(
    tenant_id_raw: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    set_lifecycle(
        tenant_id_raw,
        req,
        store,
        TenantStatus::Suspended,
        AuditKind::PlatformTenantSuspended,
    )
    .await
}

/// `POST /api/v1/platform/tenants/<id>/reactivate`. No body required.
pub(super) async fn post_reactivate(
    tenant_id_raw: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    set_lifecycle(
        tenant_id_raw,
        req,
        store,
        TenantStatus::Active,
        AuditKind::PlatformTenantReactivated,
    )
    .await
}

async fn set_lifecycle(
    tenant_id_raw: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
    status: TenantStatus,
    kind: AuditKind,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    if let Err(deny) = enforce_staff(req, store, StaffAction::SuspendTenant).await {
        return deny;
    }
    apply_lifecycle(store, req, tenant_id_raw, status, kind).await
}

/// The lifecycle *effect* — parse the target id, flip `tenant.status`, and
/// audit to the affected tenant's chain. Split from [`set_lifecycle`] so the
/// status-change + audit path is testable without minting a session JWT; only
/// ever reached AFTER [`enforce_staff`] in the live path.
async fn apply_lifecycle(
    store: &Arc<SessionStore>,
    req: &str,
    tenant_id_raw: &str,
    status: TenantStatus,
    kind: AuditKind,
) -> (&'static str, String) {
    let tenant = match parse_tenant_id(tenant_id_raw) {
        Ok(t) => t,
        Err(resp) => return resp,
    };
    // Existence check first — a 404 must not leave a spurious audit row.
    match get_tenant(store, tenant).await {
        Ok(Some(_)) => {}
        Ok(None) => return not_found(&format!("tenant {tenant} not found")),
        Err(e) => return server_error(&format!("load tenant: {e}")),
    }
    // Audit is a PREREQUISITE (DEC-029 NO-THEATER / DEC-057): the status change
    // MUST NOT persist without its hash-chained row. A sink failure fails the
    // request with the tenant UNCHANGED — mirrors the audit-as-prerequisite
    // contract in `routes_worker_capability_grant_audit.rs`. Written before the
    // mutation so a row-less suspend can never happen.
    if let Err(e) = audit_lifecycle(store, req, tenant, kind, parse_reason_opt(req)).await {
        return server_error(&format!("audit write failed; status unchanged: {e}"));
    }
    if let Err(e) = set_tenant_status(store, tenant, status, Utc::now()).await {
        return server_error(&format!("set status: {e}"));
    }
    (
        "200 OK",
        json!({
            "tenant_id": tenant.to_string(),
            "status": status.as_str(),
            "actor": session_principal_id_for_audit(req),
        })
        .to_string(),
    )
}

/// Build the account-level view for every tenant. Account-level ONLY — no
/// workspace contents (messages / audit bodies / documents) are read here.
async fn build_accounts(store: &Arc<SessionStore>) -> ArgoResult<Vec<Value>> {
    let mut out = Vec::new();
    for t in list_tenants(store).await? {
        out.push(account_json(store, &t).await?);
    }
    Ok(out)
}

async fn account_json(store: &Arc<SessionStore>, t: &Tenant) -> ArgoResult<Value> {
    let member_count = list_users_in_tenant(store, t.id).await?.len();
    let worker_count = tenant_worker_count(store, t.id).await?;
    Ok(json!({
        "tenant_id": t.id.to_string(),
        "name": t.name,
        "billing_email": t.billing_email,
        "status": t.status.as_str(),
        "created_at": t.created_at,
        "member_count": member_count,
        "worker_count": worker_count,
    }))
}

/// Write the lifecycle audit row to the AFFECTED tenant's hash-chain. Returns
/// the sink error so the caller can treat the row as a hard prerequisite (the
/// status change must not persist without it). The `#[cfg(test)]` seam forces a
/// sink failure to pin that contract.
async fn audit_lifecycle(
    store: &Arc<SessionStore>,
    req: &str,
    tenant: TenantId,
    kind: AuditKind,
    reason: Option<String>,
) -> ArgoResult<()> {
    #[cfg(test)]
    if std::env::var("SKYCOT_TEST_FORCE_AUDIT_SINK_ERROR").is_ok() {
        return Err(argo_types::ArgoError::Io(
            "forced audit sink error (DEC-057 test seam)".to_string(),
        ));
    }
    let sink = AuditSink::new_with_tenant(Arc::clone(store), tenant.to_string());
    let payload = json!({
        "tenant_id": tenant.to_string(),
        "actor": session_principal_id_for_audit(req),
        "reason": reason,
    });
    let result = sink
        .write(None, kind, payload, session_principal_id_for_audit(req))
        .await;
    if let Err(e) = &result {
        tracing::warn!(
            target: "gateway::platform_tenants",
            error = %e,
            "platform tenant lifecycle audit write failed"
        );
    }
    result
}

async fn audit_view(store: &Arc<SessionStore>, req: &str, count: usize) {
    let sink = AuditSink::new_with_tenant(Arc::clone(store), SYSTEM_AUDIT_TENANT_ID.to_string());
    let payload = json!({
        "actor": session_principal_id_for_audit(req),
        "account_count": count,
    });
    if let Err(e) = sink
        .write(
            None,
            AuditKind::PlatformAccountsViewed,
            payload,
            session_principal_id_for_audit(req),
        )
        .await
    {
        tracing::warn!(
            target: "gateway::platform_tenants",
            error = %e,
            "platform accounts-viewed audit write failed"
        );
    }
}

fn parse_reason_opt(req: &str) -> Option<String> {
    let body = body_from_request(req).trim();
    if body.is_empty() {
        return None;
    }
    serde_json::from_str::<Value>(body)
        .ok()?
        .get("reason")
        .and_then(Value::as_str)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn parse_tenant_id(raw: &str) -> Result<TenantId, (&'static str, String)> {
    let cleaned = raw.split('?').next().unwrap_or(raw);
    cleaned
        .parse::<uuid::Uuid>()
        .map(TenantId)
        .map_err(|e| bad_request(&format!("invalid tenant id '{cleaned}': {e}")))
}

fn bad_request(message: &str) -> (&'static str, String) {
    error_envelope("400 Bad Request", "bad_request", message)
}

fn not_found(message: &str) -> (&'static str, String) {
    error_envelope("404 Not Found", "not_found", message)
}

fn server_error(message: &str) -> (&'static str, String) {
    error_envelope("500 Internal Server Error", "internal_error", message)
}

fn error_envelope(status: &'static str, code: &str, message: &str) -> (&'static str, String) {
    (
        status,
        json!({ "error": { "code": code, "message": message } }).to_string(),
    )
}

#[cfg(test)]
#[path = "routes_platform_tenants_tests.rs"]
mod tests;
