//! Negative + positive tests for the role-gate.
//!
//! The procurement-grade scenario from the wave-3 RBAC audit:
//!
//! > A live demo: log in as the "guest" seed user (Naomi); open
//! > `/workers/[any-id]`; click Freeze; observe success. The guest
//! > just froze a worker.
//!
//! The matching test [`guest_jwt_blocks_worker_freeze`] proves the
//! gate now returns 403 for that case, and
//! [`x_argo_role_header_does_not_bypass_in_prod`] proves the curl
//! bypass (`-H "X-Argo-Role: owner"`) no longer works once the
//! production-mode env var is set.
#![cfg(test)]

use super::super::auth_magic_link::mint_jwt;
use super::{
    enforce_role, parse_role_header, required_role, resolve_role, role_meets, Action,
};
use argo_identity::user_tenant::UserTenantRole;
use std::sync::{Mutex, MutexGuard};

const PROD_PROFILE_ENV: &str = "ARGO_GATEWAY_PROFILE";
const JWT_SECRET_ENV: &str = "SKYCOT_AUTH_JWT_SECRET";
const TRUST_HEADER_ENV: &str = "ARGO_GATEWAY_TRUST_ROLE_HEADER";

/// Serialise tests that touch process env vars — cargo test runs
/// these in parallel by default and env state would race.
fn env_lock() -> MutexGuard<'static, ()> {
    static M: Mutex<()> = Mutex::new(());
    M.lock().unwrap_or_else(|p| p.into_inner())
}

fn set_test_secret() {
    std::env::set_var(JWT_SECRET_ENV, "test-secret-for-role-gate-tests");
}

fn lock_into_production_mode() {
    set_test_secret();
    std::env::set_var(PROD_PROFILE_ENV, "production");
    std::env::set_var("ARGO_GATEWAY_BIND_NON_LOOPBACK", "1");
    std::env::set_var("ARGO_GATEWAY_BEARER_TOKEN", "test-bearer");
    std::env::remove_var(TRUST_HEADER_ENV);
}

fn unlock_production_mode() {
    std::env::remove_var(PROD_PROFILE_ENV);
    std::env::remove_var("ARGO_GATEWAY_BIND_NON_LOOPBACK");
    std::env::remove_var("ARGO_GATEWAY_BEARER_TOKEN");
    std::env::remove_var(TRUST_HEADER_ENV);
}

fn req_with_jwt(role: Option<&str>, extra_headers: &str) -> String {
    let now = chrono::Utc::now().timestamp();
    let jwt = mint_jwt("user-1", "guest@example.com", Some("tenant-a"), role, now)
        .expect("mint");
    format!(
        "POST /api/v1/workers HTTP/1.1\r\nHost: x\r\nAuthorization: Bearer {jwt}\r\n{extra_headers}\r\n",
    )
}

#[test]
fn role_hierarchy_holds() {
    assert!(role_meets(UserTenantRole::Owner, UserTenantRole::Admin));
    assert!(role_meets(UserTenantRole::Admin, UserTenantRole::User));
    assert!(role_meets(UserTenantRole::User, UserTenantRole::Guest));
    assert!(!role_meets(UserTenantRole::Guest, UserTenantRole::User));
    assert!(!role_meets(UserTenantRole::User, UserTenantRole::Admin));
}

#[test]
fn capability_map_floors_match_brief() {
    assert_eq!(
        required_role(Action::WorkerHardDelete),
        UserTenantRole::Owner
    );
    assert_eq!(required_role(Action::AuditCrossTenant), UserTenantRole::Owner);
    assert_eq!(required_role(Action::DataExport), UserTenantRole::Owner);

    assert_eq!(required_role(Action::WorkerHire), UserTenantRole::Admin);
    assert_eq!(required_role(Action::WorkerFreeze), UserTenantRole::Admin);
    assert_eq!(required_role(Action::WorkerLifecycle), UserTenantRole::Admin);
    assert_eq!(required_role(Action::WorkerArchive), UserTenantRole::Admin);
    assert_eq!(
        required_role(Action::WorkerSettingsPatch),
        UserTenantRole::Admin
    );
    assert_eq!(required_role(Action::WorkflowEdit), UserTenantRole::Admin);
    assert_eq!(required_role(Action::BillingView), UserTenantRole::Admin);
    assert_eq!(required_role(Action::SupportTickets), UserTenantRole::Admin);

    assert_eq!(required_role(Action::ApprovalDecide), UserTenantRole::User);
    assert_eq!(required_role(Action::ApprovalUndo), UserTenantRole::User);

    assert_eq!(required_role(Action::AuditFeedRead), UserTenantRole::Guest);
    assert_eq!(required_role(Action::AuditDetailRead), UserTenantRole::Guest);
}

#[test]
fn parse_role_header_recognises_canonical_taxonomy() {
    let req = "POST / HTTP/1.1\r\nHost: x\r\nX-Argo-Role: owner\r\n\r\n";
    assert_eq!(parse_role_header(req), Some(UserTenantRole::Owner));
    let req = "POST / HTTP/1.1\r\nHost: x\r\nX-Argo-Role: guest\r\n\r\n";
    assert_eq!(parse_role_header(req), Some(UserTenantRole::Guest));
    // Other taxonomies (operator/delegator) are not recognised by the
    // canonical gate — they fall through to JWT.
    let req = "POST / HTTP/1.1\r\nHost: x\r\nX-Argo-Role: operator\r\n\r\n";
    assert_eq!(parse_role_header(req), None);
}

#[test]
fn guest_jwt_blocks_worker_freeze() {
    let _g = env_lock();
    lock_into_production_mode();
    let req = req_with_jwt(Some("guest"), "");
    let deny = enforce_role(&req, Action::WorkerFreeze);
    unlock_production_mode();
    let (status, body) = deny.expect("guest must be blocked from freeze");
    assert_eq!(status, "403 Forbidden");
    assert!(body.contains("\"denied\":true"));
    assert!(body.contains("worker.freeze"));
}

#[test]
fn admin_jwt_passes_worker_hire_in_production() {
    let _g = env_lock();
    lock_into_production_mode();
    let req = req_with_jwt(Some("admin"), "");
    let deny = enforce_role(&req, Action::WorkerHire);
    unlock_production_mode();
    assert!(deny.is_none(), "admin must pass hire gate, got {deny:?}");
}

#[test]
fn owner_jwt_required_for_audit_cross_tenant() {
    let _g = env_lock();
    lock_into_production_mode();
    let owner = req_with_jwt(Some("owner"), "");
    assert!(enforce_role(&owner, Action::AuditCrossTenant).is_none());
    let admin = req_with_jwt(Some("admin"), "");
    let deny = enforce_role(&admin, Action::AuditCrossTenant);
    unlock_production_mode();
    let (status, _) = deny.expect("admin must NOT cross tenants");
    assert_eq!(status, "403 Forbidden");
}

#[test]
fn x_argo_role_header_does_not_bypass_in_prod() {
    // Procurement-grade test: curl with `X-Argo-Role: owner` does
    // NOT bypass the gate when the actual session JWT role is guest.
    let _g = env_lock();
    lock_into_production_mode();
    let req = req_with_jwt(Some("guest"), "X-Argo-Role: owner\r\n");
    let deny = enforce_role(&req, Action::WorkerHire);
    let role = resolve_role(&req);
    unlock_production_mode();
    let (status, body) = deny.expect("header must not bypass JWT");
    assert_eq!(status, "403 Forbidden");
    assert!(body.contains("\"denied\":true"));
    // The verified JWT role (guest) wins over the bogus header (owner).
    assert_eq!(role, Some(UserTenantRole::Guest));
}

#[test]
fn x_argo_role_header_does_not_bypass_when_no_jwt_in_prod() {
    // No JWT at all: gate fails closed to guest, regardless of header.
    let _g = env_lock();
    lock_into_production_mode();
    let req = "POST / HTTP/1.1\r\nHost: x\r\nX-Argo-Role: owner\r\n\r\n";
    let deny = enforce_role(req, Action::WorkerHire);
    let role = resolve_role(req);
    unlock_production_mode();
    assert!(deny.is_some(), "missing JWT must NOT default to header trust");
    assert!(role.is_none() || role == Some(UserTenantRole::Guest));
}

#[test]
fn opt_in_trust_role_header_env_unblocks_legacy_callers() {
    // Operators behind a TLS-terminated reverse proxy can opt into
    // header trust by setting `ARGO_GATEWAY_TRUST_ROLE_HEADER=1`.
    let _g = env_lock();
    lock_into_production_mode();
    std::env::set_var(TRUST_HEADER_ENV, "1");
    let req = "POST / HTTP/1.1\r\nHost: x\r\nX-Argo-Role: owner\r\n\r\n";
    let deny = enforce_role(req, Action::WorkerHire);
    std::env::remove_var(TRUST_HEADER_ENV);
    unlock_production_mode();
    assert!(
        deny.is_none(),
        "opt-in trust env must let X-Argo-Role: owner through"
    );
}

#[test]
fn jwt_role_claim_round_trips_through_decoder() {
    let _g = env_lock();
    set_test_secret();
    let now = chrono::Utc::now().timestamp();
    let jwt = mint_jwt(
        "user-2",
        "alice@acme.com",
        Some("tenant-x"),
        Some("admin"),
        now,
    )
    .expect("mint");
    let req = format!(
        "GET /api/v1/audit HTTP/1.1\r\nHost: x\r\nAuthorization: Bearer {jwt}\r\n\r\n"
    );
    assert_eq!(resolve_role(&req), Some(UserTenantRole::Admin));
}

#[test]
fn jwt_without_role_claim_defaults_to_guest_on_enforcement() {
    let _g = env_lock();
    lock_into_production_mode();
    let req = req_with_jwt(None, "");
    let deny = enforce_role(&req, Action::WorkerHire);
    unlock_production_mode();
    assert!(
        deny.is_some(),
        "missing role claim must default to guest for production gates"
    );
}
