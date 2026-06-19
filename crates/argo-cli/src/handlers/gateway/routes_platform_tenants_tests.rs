//! Tests for the platform-staff console routes (DEC-057 Phase 1).
//!
//! Coverage split:
//! - **Default-deny** at the HTTP layer (no session → 403; no mutation on deny).
//! - **Effect path** ([`apply_lifecycle`] / [`build_accounts`]) tested directly
//!   so the status-flip, error envelopes, and account-level-only shape are
//!   pinned without minting a JWT (race-free).
//! - **One end-to-end** authorized suspend through [`post_suspend`] with a real
//!   session JWT, proving the gate→effect wiring.

use super::super::auth_magic_link::mint_jwt;
use super::{apply_lifecycle, build_accounts, list_response, post_suspend};
use argo_audit::AuditKind;
use argo_identity::platform_staff::{PlatformRole, PlatformStaff};
use argo_identity::platform_staff_store::upsert_staff;
use argo_identity::tenant::{Tenant, TenantStatus};
use argo_identity::tenant_store::{get_tenant, insert_tenant};
use argo_identity::user::{UserAccount, UserId};
use argo_identity::user_store::insert_user;
use argo_identity::user_tenant::UserTenantRole;
use argo_identity::user_tenant_store::link_user_to_tenant;
use argo_store::SessionStore;
use std::sync::Arc;

async fn store() -> Arc<SessionStore> {
    let store = Arc::new(SessionStore::open(":memory:").await.expect("open"));
    store.migrate().await.expect("migrate");
    store
}

async fn insert_active_tenant(store: &Arc<SessionStore>) -> Tenant {
    let t = Tenant::new("Acme".into(), "billing@acme.com".into());
    insert_tenant(store, &t).await.expect("insert tenant");
    t
}

/// A request carrying NO session — the default-deny case.
fn anon_req(path_tail: &str) -> String {
    format!("POST /api/v1/platform/tenants/{path_tail} HTTP/1.1\r\nHost: x\r\n\r\n")
}

#[tokio::test]
async fn no_session_denies_list() {
    let store = store().await;
    let resp = list_response("GET /api/v1/platform/tenants HTTP/1.1\r\n\r\n", Some(&store)).await;
    assert!(resp.0.starts_with("403"), "got {}", resp.0);
}

#[tokio::test]
async fn no_session_denies_suspend_and_does_not_mutate() {
    let store = store().await;
    let t = insert_active_tenant(&store).await;
    let resp = post_suspend(&t.id.to_string(), &anon_req(&format!("{}/suspend", t.id)), Some(&store)).await;
    assert!(resp.0.starts_with("403"), "got {}", resp.0);
    // The denied request must not have flipped the status.
    let after = get_tenant(&store, t.id).await.unwrap().unwrap();
    assert_eq!(after.status, TenantStatus::Active);
}

#[tokio::test]
async fn store_unavailable_returns_503() {
    let list = list_response("GET /api/v1/platform/tenants HTTP/1.1\r\n\r\n", None).await;
    assert!(list.0.starts_with("503"), "got {}", list.0);
    let suspend = post_suspend("00000000-0000-0000-0000-000000000000", &anon_req("x/suspend"), None).await;
    assert!(suspend.0.starts_with("503"), "got {}", suspend.0);
}

#[tokio::test]
async fn apply_suspend_then_reactivate_flips_status() {
    // Serialize against the forced-audit-error test (shared process env).
    let _env = crate::handlers::gateway::shared_env_lock().await;
    let store = store().await;
    let t = insert_active_tenant(&store).await;
    let req = "POST / HTTP/1.1\r\n\r\n";

    let s = apply_lifecycle(
        &store,
        req,
        &t.id.to_string(),
        TenantStatus::Suspended,
        AuditKind::PlatformTenantSuspended,
    )
    .await;
    assert!(s.0.starts_with("200"), "got {}", s.0);
    assert_eq!(
        get_tenant(&store, t.id).await.unwrap().unwrap().status,
        TenantStatus::Suspended
    );

    let r = apply_lifecycle(
        &store,
        req,
        &t.id.to_string(),
        TenantStatus::Active,
        AuditKind::PlatformTenantReactivated,
    )
    .await;
    assert!(r.0.starts_with("200"), "got {}", r.0);
    assert_eq!(
        get_tenant(&store, t.id).await.unwrap().unwrap().status,
        TenantStatus::Active
    );
}

#[tokio::test]
async fn apply_unknown_tenant_is_404() {
    let store = store().await;
    let resp = apply_lifecycle(
        &store,
        "POST / HTTP/1.1\r\n\r\n",
        "018f55b7-89ab-7cde-8123-456789abcdef",
        TenantStatus::Suspended,
        AuditKind::PlatformTenantSuspended,
    )
    .await;
    assert!(resp.0.starts_with("404"), "got {}", resp.0);
}

#[tokio::test]
async fn apply_bad_tenant_id_is_400() {
    let store = store().await;
    let resp = apply_lifecycle(
        &store,
        "POST / HTTP/1.1\r\n\r\n",
        "not-a-uuid",
        TenantStatus::Suspended,
        AuditKind::PlatformTenantSuspended,
    )
    .await;
    assert!(resp.0.starts_with("400"), "got {}", resp.0);
}

#[tokio::test]
async fn build_accounts_is_account_level_only() {
    let store = store().await;
    let t = insert_active_tenant(&store).await;
    // Two members, zero workers.
    for email in ["a@acme.com", "b@acme.com"] {
        let u = UserAccount::new(email.into());
        insert_user(&store, &u).await.unwrap();
        link_user_to_tenant(&store, u.id, t.id, UserTenantRole::User)
            .await
            .unwrap();
    }

    let accounts = build_accounts(&store).await.expect("build accounts");
    let acct = accounts
        .iter()
        .find(|a| a["tenant_id"] == t.id.to_string())
        .expect("our tenant present");

    assert_eq!(acct["name"], "Acme");
    assert_eq!(acct["billing_email"], "billing@acme.com");
    assert_eq!(acct["status"], "active");
    assert_eq!(acct["member_count"], 2);
    assert_eq!(acct["worker_count"], 0);

    // Phase 1 is account-level ONLY — workspace contents must never leak.
    for forbidden in ["messages", "documents", "audit", "memory", "sessions"] {
        assert!(
            acct.get(forbidden).is_none(),
            "account view leaked workspace content key '{forbidden}'"
        );
    }
}

#[tokio::test]
async fn audit_failure_blocks_the_status_change() {
    // Audit is a prerequisite: if the hash-chained row can't be written, the
    // status change must NOT persist (DEC-029 NO-THEATER). Hold the shared env
    // lock so the forced-error var can't leak into a concurrent test.
    let _env = crate::handlers::gateway::shared_env_lock().await;
    let store = store().await;
    let t = insert_active_tenant(&store).await;
    std::env::set_var("SKYCOT_TEST_FORCE_AUDIT_SINK_ERROR", "1");
    let resp = apply_lifecycle(
        &store,
        "POST / HTTP/1.1\r\n\r\n",
        &t.id.to_string(),
        TenantStatus::Suspended,
        AuditKind::PlatformTenantSuspended,
    )
    .await;
    std::env::remove_var("SKYCOT_TEST_FORCE_AUDIT_SINK_ERROR");
    assert!(resp.0.starts_with("500"), "expected 500, got {}", resp.0);
    // The tenant must remain Active — no silent suspend without an audit row.
    assert_eq!(
        get_tenant(&store, t.id).await.unwrap().unwrap().status,
        TenantStatus::Active
    );
}

#[tokio::test]
async fn authorized_staff_can_suspend_end_to_end() {
    let _env = crate::handlers::gateway::shared_env_lock().await;
    std::env::set_var("SKYCOT_AUTH_JWT_SECRET", "test-secret-platform-staff-e2e");
    let store = store().await;

    // Seed a super-admin staff member with a UUID user id.
    let staff_uid = UserId::new();
    upsert_staff(&store, &PlatformStaff::new(staff_uid, PlatformRole::SuperAdmin, None))
        .await
        .unwrap();

    // Target tenant to suspend.
    let target = insert_active_tenant(&store).await;

    // Mint a session JWT whose `sub` is the staff member's user id. The
    // tenant role ("owner") is irrelevant to the staff gate.
    let now = chrono::Utc::now().timestamp();
    let (jwt, _jti) = mint_jwt(
        &staff_uid.to_string(),
        "staff@skycot.com",
        Some("staff-tenant"),
        Some("owner"),
        now,
    )
    .expect("mint jwt");
    let req = format!(
        "POST /api/v1/platform/tenants/{}/suspend HTTP/1.1\r\nHost: x\r\nAuthorization: Bearer {jwt}\r\n\r\n",
        target.id
    );

    let resp = post_suspend(&target.id.to_string(), &req, Some(&store)).await;
    assert!(resp.0.starts_with("200"), "expected 200, got {} / {}", resp.0, resp.1);
    assert_eq!(
        get_tenant(&store, target.id).await.unwrap().unwrap().status,
        TenantStatus::Suspended
    );
}
