//! Tests for the suspended-tenant write lockout (DEC-057 Phase 1).

use super::super::auth_magic_link::mint_jwt;
use super::{check_tenant_not_suspended, is_write_lockout_candidate};
use argo_identity::tenant::{Tenant, TenantStatus};
use argo_identity::tenant_store::{insert_tenant, set_tenant_status};
use argo_store::SessionStore;
use chrono::Utc;
use std::sync::Arc;

#[test]
fn reads_and_preflight_are_never_candidates() {
    for req in [
        "GET /api/v1/sessions HTTP/1.1\r\n\r\n",
        "HEAD /api/v1/sessions HTTP/1.1\r\n\r\n",
        "OPTIONS /api/v1/sessions HTTP/1.1\r\n\r\n",
    ] {
        assert!(!is_write_lockout_candidate(req), "{req}");
    }
}

#[test]
fn mutating_app_routes_are_candidates() {
    for req in [
        "POST /api/v1/sessions HTTP/1.1\r\n\r\n",
        "POST /api/v1/sessions?x=1 HTTP/1.1\r\n\r\n",
        "DELETE /api/v1/workers/abc HTTP/1.1\r\n\r\n",
        "PATCH /api/v1/workers/abc HTTP/1.1\r\n\r\n",
    ] {
        assert!(is_write_lockout_candidate(req), "{req}");
    }
}

#[test]
fn orchestrator_tier_mutations_are_candidates() {
    // These dispatch via routes_orchestrator BEFORE routes::dispatch_route, so
    // the lockout must run at the boot_router top tier to cover them. Pin that
    // the predicate flags them (regression for the adversarial-review CRITICAL).
    for req in [
        "POST /sessions HTTP/1.1\r\n\r\n",
        "PATCH /sessions/abc/status HTTP/1.1\r\n\r\n",
        "POST /slas HTTP/1.1\r\n\r\n",
        "DELETE /slas/abc HTTP/1.1\r\n\r\n",
        "PATCH /handoffs/abc/status HTTP/1.1\r\n\r\n",
    ] {
        assert!(is_write_lockout_candidate(req), "{req}");
    }
}

#[test]
fn exempt_lanes_pass_even_when_mutating() {
    for req in [
        "POST /api/v1/platform/tenants/x/suspend HTTP/1.1\r\n\r\n",
        "POST /api/v1/billing/checkout HTTP/1.1\r\n\r\n",
        "POST /api/v1/auth/logout HTTP/1.1\r\n\r\n",
    ] {
        assert!(!is_write_lockout_candidate(req), "{req}");
    }
}

#[tokio::test]
async fn suspended_tenant_blocks_write_allows_read() {
    let _env = crate::handlers::gateway::shared_env_lock().await;
    std::env::set_var("SKYCOT_AUTH_JWT_SECRET", "test-secret-suspend-lockout");
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();

    let t = Tenant::new("Acme".into(), "b@acme.com".into());
    insert_tenant(&store, &t).await.unwrap();
    set_tenant_status(&store, t.id, TenantStatus::Suspended, Utc::now())
        .await
        .unwrap();

    // A session JWT scoped to the suspended tenant.
    let now = Utc::now().timestamp();
    let (jwt, _jti) = mint_jwt(
        "user-x",
        "u@acme.com",
        Some(&t.id.to_string()),
        Some("owner"),
        now,
    )
    .expect("mint");

    let post = format!(
        "POST /api/v1/sessions HTTP/1.1\r\nHost: x\r\nAuthorization: Bearer {jwt}\r\n\r\n"
    );
    let blocked = check_tenant_not_suspended(&post, Some(&store)).await;
    assert!(
        blocked.as_ref().map(|r| r.0).unwrap_or("").starts_with("403"),
        "suspended tenant write must be 403, got {blocked:?}"
    );

    // A read from the same suspended tenant is allowed (reads OK).
    let get = format!(
        "GET /api/v1/sessions HTTP/1.1\r\nHost: x\r\nAuthorization: Bearer {jwt}\r\n\r\n"
    );
    assert!(check_tenant_not_suspended(&get, Some(&store)).await.is_none());
}

#[tokio::test]
async fn active_tenant_write_passes() {
    let _env = crate::handlers::gateway::shared_env_lock().await;
    std::env::set_var("SKYCOT_AUTH_JWT_SECRET", "test-secret-suspend-lockout");
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();

    let t = Tenant::new("Acme".into(), "b@acme.com".into());
    insert_tenant(&store, &t).await.unwrap();

    let now = Utc::now().timestamp();
    let (jwt, _jti) = mint_jwt(
        "user-x",
        "u@acme.com",
        Some(&t.id.to_string()),
        Some("owner"),
        now,
    )
    .expect("mint");
    let post = format!(
        "POST /api/v1/sessions HTTP/1.1\r\nHost: x\r\nAuthorization: Bearer {jwt}\r\n\r\n"
    );
    assert!(check_tenant_not_suspended(&post, Some(&store)).await.is_none());
}
