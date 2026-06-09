//! Unit tests for [`super::resolve_tenant_for_request`] — pins the
//! JWT-session branch, the explicit-override branch, and the
//! cwd/anonymous fallback, plus the precedence between them.
//!
//! Env handling mirrors `middleware_session_verify_tests`: a
//! process-shared `Mutex<()>` serializes the `SKYCOT_AUTH_JWT_SECRET`
//! manipulation so parallel tests in the same binary don't see a torn
//! secret. Tests are single-threaded across their await points while
//! holding the guard, so a `std::sync::Mutex` is sufficient.

use super::*;
use argo_identity::user::UserAccount;
use argo_identity::user_store::insert_user;
use argo_identity::{provision_for_user, ProvisionRequest};
use argo_store::SessionStore;
use chrono::Utc;
use jsonwebtoken::{EncodingKey, Header};
use serde::Serialize;
use std::sync::{Mutex, MutexGuard};

const TEST_SECRET: &str = "ws1-resolve-tenant-test-secret";

/// Serialize `SKYCOT_AUTH_JWT_SECRET` mutation across tests in this
/// binary.
fn env_guard() -> MutexGuard<'static, ()> {
    static ENV_LOCK: Mutex<()> = Mutex::new(());
    ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner())
}

/// Minimal cookie-JWT claim shape — matches the encode side in
/// `auth_magic_link::mint_jwt` for the fields the resolver reads.
#[derive(Serialize)]
struct TestClaims<'a> {
    sub: &'a str,
    email: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    tnt: Option<String>,
    iat: i64,
    exp: i64,
}

/// Mint a `__Host-session` cookie JWT carrying `tnt = tenant`.
fn mint_session(secret: &str, tenant: Option<String>) -> String {
    let now = Utc::now().timestamp();
    jsonwebtoken::encode(
        &Header::new(jsonwebtoken::Algorithm::HS256),
        &TestClaims {
            sub: "11111111-1111-1111-1111-111111111111",
            email: "operator@example.com",
            tnt: tenant,
            iat: now - 10,
            exp: now + 3600,
        },
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .expect("mint session jwt")
}

fn req_with_cookie(jwt: &str) -> String {
    format!("GET /api/v1/team HTTP/1.1\r\nHost: localhost\r\nCookie: __Host-session={jwt}\r\n\r\n")
}

fn req_with_bearer(jwt: &str) -> String {
    format!("GET /api/v1/team HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer {jwt}\r\n\r\n")
}

async fn fresh_store() -> SessionStore {
    let store = SessionStore::open(":memory:").await.expect("open store");
    store.migrate().await.expect("migrate");
    store
}

/// Provision a fresh user → returns the tenant they own.
async fn provision(store: &SessionStore, email: &str, ws: &str) -> TenantId {
    let user = UserAccount::new(email.to_string());
    insert_user(store, &user).await.expect("insert user");
    provision_for_user(
        store,
        ProvisionRequest {
            user_id: user.id,
            user_email: &user.email,
            workspace_name: ws,
            company_name: None,
        },
    )
    .await
    .expect("provision")
    .tenant_id
}

/// Branch 1 — a valid `__Host-session` JWT resolves to the user-bound
/// tenant from its `tnt` claim, never the cwd/demo tenant and never
/// another user's tenant.
#[allow(clippy::await_holding_lock)]
#[tokio::test]
async fn jwt_session_resolves_to_user_tenant_not_cwd_or_other_user() {
    let _g = env_guard();
    std::env::set_var("SKYCOT_AUTH_JWT_SECRET", TEST_SECRET);

    let store = fresh_store().await;
    let tenant_a = provision(&store, "a@acme.test", "Acme A").await;
    let tenant_b = provision(&store, "b@beta.test", "Beta B").await;
    assert_ne!(tenant_a, tenant_b, "two users must get distinct tenants");

    // The cwd/demo tenant the no-session path would resolve to.
    let cwd_tenant = resolve_tenant_for_request("GET /api/v1/team HTTP/1.1\r\n\r\n", &store)
        .await
        .expect("cwd resolve");

    let jwt = mint_session(TEST_SECRET, Some(tenant_a.to_string()));
    let resolved = resolve_tenant_for_request(&req_with_cookie(&jwt), &store)
        .await
        .expect("jwt resolve");

    assert_eq!(resolved, tenant_a, "JWT session must resolve to A's tenant");
    assert_ne!(resolved, tenant_b, "must never resolve to B's tenant");
    assert_ne!(resolved, cwd_tenant, "must not fall back to the cwd tenant");

    std::env::remove_var("SKYCOT_AUTH_JWT_SECRET");
}

/// Branch 3 — no session cookie → the cwd/anonymous tenant. This is
/// the demo / public-endpoint path and must be a *different* tenant
/// from any provisioned user (proving a stranger lands in the shared
/// demo workspace, not a user's private one).
#[allow(clippy::await_holding_lock)]
#[tokio::test]
async fn no_session_falls_back_to_cwd_tenant() {
    let _g = env_guard();
    std::env::set_var("SKYCOT_AUTH_JWT_SECRET", TEST_SECRET);

    let store = fresh_store().await;
    let tenant_a = provision(&store, "a@acme.test", "Acme A").await;

    let resolved = resolve_tenant_for_request("GET /api/v1/team HTTP/1.1\r\n\r\n", &store)
        .await
        .expect("cwd resolve");
    assert_ne!(resolved, tenant_a, "no-session must not resolve to a user tenant");

    // Idempotent: same cwd resolves to the same tenant on re-call.
    let again = resolve_tenant_for_request("GET /api/v1/team HTTP/1.1\r\n\r\n", &store)
        .await
        .expect("cwd resolve 2");
    assert_eq!(resolved, again, "cwd tenant must be stable across calls");

    std::env::remove_var("SKYCOT_AUTH_JWT_SECRET");
}

/// Branch 2 — explicit `X-Tenant-Id` header (no cookie) resolves to
/// that tenant, preserving the #2589 override contract.
#[allow(clippy::await_holding_lock)]
#[tokio::test]
async fn explicit_header_override_resolves_when_no_session() {
    let _g = env_guard();
    std::env::set_var("SKYCOT_AUTH_JWT_SECRET", TEST_SECRET);

    let store = fresh_store().await;
    let tenant_b = provision(&store, "b@beta.test", "Beta B").await;

    let req = format!(
        "GET /api/v1/team HTTP/1.1\r\nHost: localhost\r\nX-Tenant-Id: {tenant_b}\r\n\r\n"
    );
    let resolved = resolve_tenant_for_request(&req, &store)
        .await
        .expect("header resolve");
    assert_eq!(resolved, tenant_b, "X-Tenant-Id must select that tenant");

    // `?tenant=` query form also works.
    let req_q = format!("GET /api/v1/team?tenant={tenant_b} HTTP/1.1\r\nHost: localhost\r\n\r\n");
    let resolved_q = resolve_tenant_for_request(&req_q, &store)
        .await
        .expect("query resolve");
    assert_eq!(resolved_q, tenant_b, "?tenant= must select that tenant");

    std::env::remove_var("SKYCOT_AUTH_JWT_SECRET");
}

/// Precedence — a signed-in user cannot use `X-Tenant-Id` to escape
/// their own tenant: the JWT session (branch 1) wins over the explicit
/// header (branch 2).
#[allow(clippy::await_holding_lock)]
#[tokio::test]
async fn jwt_session_wins_over_explicit_header() {
    let _g = env_guard();
    std::env::set_var("SKYCOT_AUTH_JWT_SECRET", TEST_SECRET);

    let store = fresh_store().await;
    let tenant_a = provision(&store, "a@acme.test", "Acme A").await;
    let tenant_b = provision(&store, "b@beta.test", "Beta B").await;

    let jwt = mint_session(TEST_SECRET, Some(tenant_a.to_string()));
    let req = format!(
        "GET /api/v1/team HTTP/1.1\r\nHost: localhost\r\n\
         Cookie: __Host-session={jwt}\r\nX-Tenant-Id: {tenant_b}\r\n\r\n"
    );
    let resolved = resolve_tenant_for_request(&req, &store)
        .await
        .expect("resolve");
    assert_eq!(resolved, tenant_a, "JWT tenant must win over header override");

    std::env::remove_var("SKYCOT_AUTH_JWT_SECRET");
}

/// R4-F1 repro — the Genesis generate is a POST with a JSON body (every
/// other test here is a bodyless GET). If header/cookie extraction breaks
/// when a body is present, the genesis write resolves the cwd tenant while
/// the (GET-shaped) hire resolves the session tenant → "unknown vertical".
#[allow(clippy::await_holding_lock)]
#[tokio::test]
async fn jwt_session_resolves_on_post_with_body_like_genesis_generate() {
    let _g = env_guard();
    std::env::set_var("SKYCOT_AUTH_JWT_SECRET", TEST_SECRET);

    let store = fresh_store().await;
    let tenant_a = provision(&store, "a@acme.test", "Acme A").await;
    let jwt = mint_session(TEST_SECRET, Some(tenant_a.to_string()));

    let body = r#"{"role_description":"A billing operations assistant."}"#;
    let req = format!(
        "POST /api/v1/genesis HTTP/1.1\r\nHost: localhost\r\n\
         Content-Type: application/json\r\n\
         Cookie: __Host-session={jwt}\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    let resolved = resolve_tenant_for_request(&req, &store)
        .await
        .expect("resolve");
    assert_eq!(
        resolved, tenant_a,
        "POST+body+cookie (genesis generate shape) must resolve the session tenant, not cwd"
    );

    std::env::remove_var("SKYCOT_AUTH_JWT_SECRET");
}

/// A cookie whose `tnt` claim is absent (user mid-onboarding) falls
/// through to the cwd fallback rather than erroring.
#[allow(clippy::await_holding_lock)]
#[tokio::test]
async fn session_without_tnt_claim_falls_through() {
    let _g = env_guard();
    std::env::set_var("SKYCOT_AUTH_JWT_SECRET", TEST_SECRET);

    let store = fresh_store().await;
    let jwt = mint_session(TEST_SECRET, None);
    let resolved = resolve_tenant_for_request(&req_with_cookie(&jwt), &store)
        .await
        .expect("resolve");
    // Equals the no-session cwd tenant.
    let cwd = resolve_tenant_for_request("GET /api/v1/team HTTP/1.1\r\n\r\n", &store)
        .await
        .expect("cwd resolve");
    assert_eq!(resolved, cwd, "tnt-less cookie must use the cwd fallback");

    std::env::remove_var("SKYCOT_AUTH_JWT_SECRET");
}

/// Bearer branch — a signed-in user's session JWT presented as an
/// `Authorization: Bearer` (the form the SPA's server-side proxy route
/// handlers forward — Run-now, workflow-propose) resolves to the user's
/// OWN tenant, never the cwd/demo tenant. Without this the bearer-only
/// proxies ran against the shared demo workspace (WS1 cross-tenant hazard).
#[allow(clippy::await_holding_lock)]
#[tokio::test]
async fn bearer_session_resolves_to_user_tenant_not_cwd() {
    let _g = env_guard();
    std::env::set_var("SKYCOT_AUTH_JWT_SECRET", TEST_SECRET);

    let store = fresh_store().await;
    let tenant_a = provision(&store, "a@acme.test", "Acme A").await;
    let cwd_tenant = resolve_tenant_for_request("GET /api/v1/team HTTP/1.1\r\n\r\n", &store)
        .await
        .expect("cwd resolve");

    let jwt = mint_session(TEST_SECRET, Some(tenant_a.to_string()));
    let resolved = resolve_tenant_for_request(&req_with_bearer(&jwt), &store)
        .await
        .expect("bearer resolve");
    assert_eq!(resolved, tenant_a, "bearer session must resolve to A's tenant");
    assert_ne!(resolved, cwd_tenant, "bearer must not fall back to the cwd tenant");

    std::env::remove_var("SKYCOT_AUTH_JWT_SECRET");
}

/// Precedence — the cookie session wins over the bearer when both are
/// present (the same user in practice; this pins the order so a future
/// change can't silently flip it).
#[allow(clippy::await_holding_lock)]
#[tokio::test]
async fn cookie_session_wins_over_bearer() {
    let _g = env_guard();
    std::env::set_var("SKYCOT_AUTH_JWT_SECRET", TEST_SECRET);

    let store = fresh_store().await;
    let tenant_a = provision(&store, "a@acme.test", "Acme A").await;
    let tenant_b = provision(&store, "b@beta.test", "Beta B").await;

    let cookie_jwt = mint_session(TEST_SECRET, Some(tenant_a.to_string()));
    let bearer_jwt = mint_session(TEST_SECRET, Some(tenant_b.to_string()));
    let req = format!(
        "GET /api/v1/team HTTP/1.1\r\nHost: localhost\r\n\
         Cookie: __Host-session={cookie_jwt}\r\nAuthorization: Bearer {bearer_jwt}\r\n\r\n"
    );
    let resolved = resolve_tenant_for_request(&req, &store)
        .await
        .expect("resolve");
    assert_eq!(resolved, tenant_a, "cookie session must win over the bearer");

    std::env::remove_var("SKYCOT_AUTH_JWT_SECRET");
}

/// The bearer session (like the cookie) wins over the explicit
/// `X-Tenant-Id` override — a signed-in user proxied via the bearer cannot
/// be redirected to another tenant by a stray header.
#[allow(clippy::await_holding_lock)]
#[tokio::test]
async fn bearer_session_wins_over_explicit_header() {
    let _g = env_guard();
    std::env::set_var("SKYCOT_AUTH_JWT_SECRET", TEST_SECRET);

    let store = fresh_store().await;
    let tenant_a = provision(&store, "a@acme.test", "Acme A").await;
    let tenant_b = provision(&store, "b@beta.test", "Beta B").await;

    let bearer_jwt = mint_session(TEST_SECRET, Some(tenant_a.to_string()));
    let req = format!(
        "GET /api/v1/team HTTP/1.1\r\nHost: localhost\r\n\
         Authorization: Bearer {bearer_jwt}\r\nX-Tenant-Id: {tenant_b}\r\n\r\n"
    );
    let resolved = resolve_tenant_for_request(&req, &store)
        .await
        .expect("resolve");
    assert_eq!(
        resolved, tenant_a,
        "bearer session must win over header override"
    );

    std::env::remove_var("SKYCOT_AUTH_JWT_SECRET");
}
