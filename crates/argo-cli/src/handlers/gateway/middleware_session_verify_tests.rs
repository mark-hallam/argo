//! Unit tests for [`super`] — the verify-middleware. Covers the
//! brief's 5 load-bearing cases:
//!
//! 1. valid cookie → `Valid(Claims)`,
//! 2. missing cookie → `NoCookie` (caller falls through to bearer),
//! 3. bad signature → `Reject("session invalid")`,
//! 4. expired `exp` → `Reject("session expired")`,
//! 5. valid sig + valid exp + `jti` revoked → `Reject("session revoked")`
//!    (the enterprise gate — without this, the revocation table is
//!    cosmetic).
//!
//! Each test sets `SKYCOT_AUTH_JWT_SECRET` via a process-shared
//! `Mutex<()>` so the env-var manipulation is serialized; the env
//! var is the secret both the test-side mint and the SUT-side verify
//! read, so the two paths share a key.

use super::{extract_session_cookie, verify_session_cookie, SessionVerifyOutcome};
use argo_store::SessionStore;
use chrono::Utc;
use jsonwebtoken::{EncodingKey, Header};
use serde::Serialize;
use std::sync::{Arc, Mutex};

const TEST_SECRET: &str = "skycot-test-secret-deadbeefcafebabe";
const OTHER_SECRET: &str = "skycot-test-secret-other-key-for-bad-sig";

/// Serializes env-var writes across the test functions so concurrent
/// tests in the same binary don't see torn `SKYCOT_AUTH_JWT_SECRET`
/// state. Each test acquires the lock for the duration of its body.
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

/// Mint a test cookie JWT — directly with `jsonwebtoken::encode`.
/// We mint here (rather than calling `auth_magic_link_token::mint_jwt`)
/// so the test surface is exactly the wire format the verify-middleware
/// receives, including the optional `jti` claim. `secret` lets us
/// flip to a non-matching key for the bad-signature test.
fn mint(secret: &str, sub: &str, jti: Option<&str>, exp: i64) -> String {
    let key = EncodingKey::from_secret(secret.as_bytes());
    jsonwebtoken::encode(
        &Header::new(jsonwebtoken::Algorithm::HS256),
        &TestClaims {
            sub,
            email: "operator@example.com",
            tnt: Some("tenant-a"),
            jti,
            iat: Utc::now().timestamp() - 10,
            exp,
        },
        &key,
    )
    .expect("mint jwt")
}

fn request_with_cookie(cookie_pair: &str) -> String {
    format!(
        "GET /api/v1/sessions HTTP/1.1\r\nHost: localhost\r\nCookie: {cookie_pair}\r\n\r\n"
    )
}

async fn fresh_store() -> Arc<SessionStore> {
    Arc::new(
        SessionStore::open_unencrypted_for_tests(":memory:")
            .await
            .expect("open memory store"),
    )
}

// Each `#[tokio::test]` below holds the `env_guard()` `Mutex<()>` across
// await points so the process-global `SKYCOT_AUTH_JWT_SECRET` env var
// stays consistent for the duration of each test. Clippy would normally
// flag the held-across-await pattern, but the alternative — async-aware
// `tokio::sync::Mutex` — is unnecessary here: tests are single-threaded
// per case and the guard is intentionally synchronous.
#[allow(clippy::await_holding_lock)]
#[tokio::test]
async fn missing_cookie_returns_no_cookie() {
    let _g = env_guard();
    std::env::set_var("SKYCOT_AUTH_JWT_SECRET", TEST_SECRET);
    let store = fresh_store().await;
    let req = "GET /api/v1/sessions HTTP/1.1\r\nHost: localhost\r\n\r\n";
    let outcome = verify_session_cookie(req, Some(&store)).await;
    assert_eq!(outcome, SessionVerifyOutcome::NoCookie);
}

#[allow(clippy::await_holding_lock)]
#[tokio::test]
async fn valid_cookie_returns_claims() {
    let _g = env_guard();
    std::env::set_var("SKYCOT_AUTH_JWT_SECRET", TEST_SECRET);
    let store = fresh_store().await;
    let now = Utc::now().timestamp();
    let jwt = mint(TEST_SECRET, "user-1", Some("jti-valid-1"), now + 3600);
    let req = request_with_cookie(&format!("__Host-session={jwt}"));
    let outcome = verify_session_cookie(&req, Some(&store)).await;
    match outcome {
        SessionVerifyOutcome::Valid(claims) => {
            assert_eq!(claims.sub, "user-1");
            assert_eq!(claims.tnt.as_deref(), Some("tenant-a"));
            assert_eq!(claims.jti.as_deref(), Some("jti-valid-1"));
        }
        other => panic!("expected Valid, got {other:?}"),
    }
}

#[allow(clippy::await_holding_lock)]
#[tokio::test]
async fn bad_signature_rejects_as_invalid() {
    let _g = env_guard();
    std::env::set_var("SKYCOT_AUTH_JWT_SECRET", TEST_SECRET);
    let store = fresh_store().await;
    let now = Utc::now().timestamp();
    // JWT signed with OTHER_SECRET but verified under TEST_SECRET.
    let jwt = mint(OTHER_SECRET, "user-1", Some("jti-x"), now + 3600);
    let req = request_with_cookie(&format!("__Host-session={jwt}"));
    let outcome = verify_session_cookie(&req, Some(&store)).await;
    assert_eq!(
        outcome,
        SessionVerifyOutcome::Reject("session invalid"),
        "got {outcome:?}"
    );
}

#[allow(clippy::await_holding_lock)]
#[tokio::test]
async fn expired_exp_rejects_as_expired() {
    let _g = env_guard();
    std::env::set_var("SKYCOT_AUTH_JWT_SECRET", TEST_SECRET);
    let store = fresh_store().await;
    let now = Utc::now().timestamp();
    // `exp` 1h in the past — jsonwebtoken's default validation sees
    // this as ExpiredSignature.
    let jwt = mint(TEST_SECRET, "user-1", Some("jti-x"), now - 3600);
    let req = request_with_cookie(&format!("__Host-session={jwt}"));
    let outcome = verify_session_cookie(&req, Some(&store)).await;
    assert_eq!(
        outcome,
        SessionVerifyOutcome::Reject("session expired"),
        "got {outcome:?}"
    );
}

#[allow(clippy::await_holding_lock)]
#[tokio::test]
async fn session_revoked_jti_rejected() {
    // The load-bearing test — without this, the table is cosmetic
    // and the 2026-05-17 HOLD's "silent regression" framing applies.
    let _g = env_guard();
    std::env::set_var("SKYCOT_AUTH_JWT_SECRET", TEST_SECRET);
    let store = fresh_store().await;
    let now = Utc::now().timestamp();
    // Mint a happy-path cookie: same secret, future exp, definite jti.
    let jti = "jti-revoked-load-bearing";
    let jwt = mint(TEST_SECRET, "user-1", Some(jti), now + 3600);
    // Revoke the jti so the verify-middleware sees `is_revoked=true`.
    argo_store::revoked_tokens::revoke(store.pool(), jti, "tenant-a", Some("user-1"), Some("logout"), now)
        .await
        .expect("revoke jti");
    let req = request_with_cookie(&format!("__Host-session={jwt}"));
    let outcome = verify_session_cookie(&req, Some(&store)).await;
    assert_eq!(
        outcome,
        SessionVerifyOutcome::Reject("session revoked"),
        "revoked jti must 401 even with valid sig + valid exp; got {outcome:?}"
    );
}

#[test]
fn extract_session_cookie_handles_multiple_pairs() {
    // Common browser case: the `__Host-session` cookie sits next to
    // a CSRF cookie or analytics cookies. The extractor must pull
    // out the right pair regardless of ordering.
    let req = "GET / HTTP/1.1\r\nHost: x\r\nCookie: ga_id=abc; __Host-session=jwt-value; foo=bar\r\n\r\n";
    assert_eq!(extract_session_cookie(req).as_deref(), Some("jwt-value"));
    let req2 = "GET / HTTP/1.1\r\nHost: x\r\nCookie: __Host-session=first\r\n\r\n";
    assert_eq!(extract_session_cookie(req2).as_deref(), Some("first"));
    let req3 = "GET / HTTP/1.1\r\nHost: x\r\nCookie: ga_id=abc\r\n\r\n";
    assert_eq!(extract_session_cookie(req3), None);
    let req4 = "GET / HTTP/1.1\r\nHost: x\r\n\r\n";
    assert_eq!(extract_session_cookie(req4), None);
    // Empty value — refuse the pair so callers don't try to verify "".
    let req5 = "GET / HTTP/1.1\r\nHost: x\r\nCookie: __Host-session=\r\n\r\n";
    assert_eq!(extract_session_cookie(req5), None);
}
