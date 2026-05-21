//! Route tests for [`super`] — the logout endpoint. Covers the
//! brief's 4 acceptance cases:
//!
//! 1. happy logout → 200 + cookie-clear, jti landed in `revoked_tokens`,
//! 2. missing cookie → 401, no row inserted,
//! 3. invalid signature → 401, no row inserted,
//! 4. double-logout → second call still returns 200; revoked_tokens
//!    row stays single (idempotent on jti).
//!
//! Driven via [`super::logout_outcome`] so the tests don't need a
//! TcpStream — the stream-owning shim is a thin formatter on top.

use super::{logout_outcome, LogoutOutcome};
use argo_store::SessionStore;
use chrono::Utc;
use jsonwebtoken::{EncodingKey, Header};
use serde::Serialize;
use std::sync::{Arc, Mutex};

const TEST_SECRET: &str = "skycot-test-secret-deadbeefcafebabe";
const OTHER_SECRET: &str = "skycot-test-secret-other-key-for-bad-sig";

/// Shared with `middleware_session_verify_tests::env_guard` in spirit
/// — locally serialize env-var writes within this binary's test set
/// so concurrent tests don't tear `SKYCOT_AUTH_JWT_SECRET`. Each test
/// holds the lock for its whole body.
fn env_guard() -> std::sync::MutexGuard<'static, ()> {
    static ENV_LOCK: Mutex<()> = Mutex::new(());
    ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner())
}

#[derive(Debug, Serialize)]
struct TestClaims<'a> {
    sub: &'a str,
    email: &'a str,
    tnt: Option<&'a str>,
    jti: Option<&'a str>,
    iat: i64,
    exp: i64,
}

fn mint(secret: &str, sub: &str, jti: Option<&str>) -> String {
    let key = EncodingKey::from_secret(secret.as_bytes());
    let now = Utc::now().timestamp();
    jsonwebtoken::encode(
        &Header::new(jsonwebtoken::Algorithm::HS256),
        &TestClaims {
            sub,
            email: "operator@example.com",
            tnt: Some("tenant-a"),
            jti,
            iat: now - 10,
            exp: now + 3600,
        },
        &key,
    )
    .expect("mint jwt")
}

fn req_with_cookie(jwt: &str) -> String {
    format!(
        "POST /api/v1/auth/logout HTTP/1.1\r\nHost: localhost\r\nContent-Length: 0\r\nCookie: __Host-session={jwt}\r\n\r\n"
    )
}

async fn fresh_store() -> Arc<SessionStore> {
    Arc::new(
        SessionStore::open_unencrypted_for_tests(":memory:")
            .await
            .expect("open memory store"),
    )
}

#[allow(clippy::await_holding_lock)]
#[tokio::test]
async fn happy_logout_revokes_jti_and_clears_cookie() {
    let _g = env_guard();
    std::env::set_var("SKYCOT_AUTH_JWT_SECRET", TEST_SECRET);
    let store = fresh_store().await;
    let jti = "jti-logout-happy";
    let jwt = mint(TEST_SECRET, "user-1", Some(jti));
    let outcome = logout_outcome(&req_with_cookie(&jwt), Some(&store)).await;
    assert_eq!(outcome.status, "200 OK", "body={}", outcome.body);
    assert!(
        outcome.set_cookie.unwrap_or_default().contains("Max-Age=0"),
        "cookie-clear directive present"
    );
    let revoked = argo_store::revoked_tokens::is_revoked(store.pool(), jti)
        .await
        .unwrap();
    assert!(revoked, "jti landed in revoked_tokens");
}

#[allow(clippy::await_holding_lock)]
#[tokio::test]
async fn missing_cookie_returns_401() {
    let _g = env_guard();
    std::env::set_var("SKYCOT_AUTH_JWT_SECRET", TEST_SECRET);
    let store = fresh_store().await;
    let req = "POST /api/v1/auth/logout HTTP/1.1\r\nHost: localhost\r\nContent-Length: 0\r\n\r\n";
    let outcome = logout_outcome(req, Some(&store)).await;
    assert_eq!(outcome.status, "401 Unauthorized");
    assert!(outcome.set_cookie.is_none(), "no cookie-clear on 401");
    // Body contains the missing_cookie reason.
    assert!(
        outcome.body.contains("missing_cookie"),
        "body={}",
        outcome.body
    );
}

#[allow(clippy::await_holding_lock)]
#[tokio::test]
async fn invalid_signature_returns_401_and_does_not_revoke() {
    let _g = env_guard();
    std::env::set_var("SKYCOT_AUTH_JWT_SECRET", TEST_SECRET);
    let store = fresh_store().await;
    let jti = "jti-should-not-land";
    // Signed under OTHER_SECRET but verified under TEST_SECRET.
    let jwt = mint(OTHER_SECRET, "user-1", Some(jti));
    let outcome = logout_outcome(&req_with_cookie(&jwt), Some(&store)).await;
    assert_eq!(outcome.status, "401 Unauthorized");
    let revoked = argo_store::revoked_tokens::is_revoked(store.pool(), jti)
        .await
        .unwrap();
    assert!(!revoked, "bad-sig request must NOT write a revocation row");
}

#[allow(clippy::await_holding_lock)]
#[tokio::test]
async fn double_logout_is_idempotent() {
    let _g = env_guard();
    std::env::set_var("SKYCOT_AUTH_JWT_SECRET", TEST_SECRET);
    let store = fresh_store().await;
    let jti = "jti-double-logout";
    let jwt = mint(TEST_SECRET, "user-1", Some(jti));
    let first: LogoutOutcome = logout_outcome(&req_with_cookie(&jwt), Some(&store)).await;
    assert_eq!(first.status, "200 OK");
    let second: LogoutOutcome = logout_outcome(&req_with_cookie(&jwt), Some(&store)).await;
    // Second call still emits cookie-clear so a confused client that
    // re-tries logout doesn't loop on the cookie still being present.
    // The verify-middleware will 401 subsequent NON-logout requests
    // because the jti is in `revoked_tokens` (covered in the
    // middleware tests).
    assert_eq!(second.status, "200 OK");
    let list = argo_store::revoked_tokens::list_for_tenant(store.pool(), "tenant-a")
        .await
        .unwrap();
    assert_eq!(list.len(), 1, "single row across two logouts");
    assert_eq!(list[0].jti, jti);
}
