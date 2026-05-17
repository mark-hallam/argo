//! Tests for the magic-link signup + verify endpoints.

use super::*;
use argo_store::SessionStore;
use std::sync::Arc;

async fn fresh_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:")
        .await
        .expect("in-memory store");
    Arc::new(store)
}

fn req(method_path: &str, ip: Option<&str>, body: &str) -> String {
    let mut headers = String::new();
    if let Some(ip) = ip {
        headers.push_str(&format!("X-Forwarded-For: {ip}\r\n"));
    }
    format!(
        "{method_path} HTTP/1.1\r\nHost: x\r\nContent-Length: {}\r\n{headers}\r\n{body}",
        body.len()
    )
}

#[tokio::test]
async fn signup_with_valid_email_returns_200() {
    let store = fresh_store().await;
    let r = req(
        "POST /api/v1/auth/signup",
        Some("203.0.113.10"),
        r#"{"email":"alice@example.com"}"#,
    );
    let (status, body) = signup_response(&r, Some(&store)).await;
    assert!(status.starts_with("200"), "status={status} body={body}");
    assert!(body.contains("\"status\":\"sent\""));
}

#[tokio::test]
async fn signup_rejects_bad_email() {
    let store = fresh_store().await;
    let r = req(
        "POST /api/v1/auth/signup",
        Some("203.0.113.11"),
        r#"{"email":"not-an-email"}"#,
    );
    let (status, _) = signup_response(&r, Some(&store)).await;
    assert!(status.starts_with("400"));
}

#[tokio::test]
async fn signup_rejects_malformed_json() {
    let store = fresh_store().await;
    let r = req(
        "POST /api/v1/auth/signup",
        Some("203.0.113.12"),
        "not json at all",
    );
    let (status, _) = signup_response(&r, Some(&store)).await;
    assert!(status.starts_with("400"));
}

#[tokio::test]
async fn signup_rate_limit_uses_shared_signup_bucket() {
    use super::rate_limit_ip::{
        signup_limiter, RateLimitIpCheckResult, SIGNUP_BUCKET_CAPACITY,
    };
    use std::net::{IpAddr, Ipv4Addr};
    use std::time::Instant;
    // Drive the shared limiter directly so the test isn't racy on
    // wall-clock and doesn't share buckets with the live `signup`
    // singleton (we use a process-unique IP).
    let ip: IpAddr = Ipv4Addr::new(192, 0, 2, 99).into();
    let now = Instant::now();
    for _ in 0..SIGNUP_BUCKET_CAPACITY {
        assert!(matches!(
            signup_limiter().check_with_now(ip, now),
            RateLimitIpCheckResult::Allow
        ));
    }
    assert!(matches!(
        signup_limiter().check_with_now(ip, now),
        RateLimitIpCheckResult::Deny { .. }
    ));
}

#[tokio::test]
async fn verify_with_invalid_token_returns_401() {
    let store = fresh_store().await;
    let r = req(
        "POST /api/v1/auth/verify",
        None,
        r#"{"token":"never-issued"}"#,
    );
    let outcome = verify_outcome(&r, Some(&store)).await;
    assert!(
        outcome.status.starts_with("401"),
        "status={} body={}",
        outcome.status,
        outcome.body
    );
    assert!(outcome.body.contains("\"invalid\""));
    assert!(outcome.set_cookie.is_none(), "no cookie on failure");
}

#[tokio::test]
async fn signup_then_verify_round_trip_sets_host_cookie() {
    let store = fresh_store().await;
    // Signup mints a token; we can't fish the raw token out of the
    // response (it goes only into the email/log), so insert one
    // directly via the store API to mimic the post-email click.
    let raw_token = "0123456789abcdef0123456789abcdef";
    let token_hash = sha256_hex(raw_token);
    let now = chrono::Utc::now().timestamp();
    argo_store::user_account::insert_magic_link(
        store.pool(),
        &token_hash,
        "bob@example.com",
        now + 900,
    )
    .await
    .expect("insert");
    let verify_req = req(
        "POST /api/v1/auth/verify",
        None,
        &format!(r#"{{"token":"{raw_token}"}}"#),
    );
    let outcome = verify_outcome(&verify_req, Some(&store)).await;
    assert!(
        outcome.status.starts_with("200"),
        "status={} body={}",
        outcome.status,
        outcome.body
    );
    assert!(outcome.body.contains("\"needs_onboarding\":true"));
    // JWT is no longer in the body; cookie is the canonical carrier.
    assert!(!outcome.body.contains("\"jwt\""));
    let cookie = outcome.set_cookie.expect("verify must emit Set-Cookie");
    assert!(cookie.starts_with("__Host-session="), "cookie={cookie}");
    assert!(cookie.contains("HttpOnly"), "cookie={cookie}");
    assert!(cookie.contains("Secure"), "cookie={cookie}");
    assert!(cookie.contains("SameSite=Lax"), "cookie={cookie}");
    assert!(cookie.contains("Path=/"), "cookie={cookie}");
    assert!(cookie.contains("Max-Age=604800"), "cookie={cookie}");
}

#[tokio::test]
async fn verify_is_single_use() {
    let store = fresh_store().await;
    let raw_token = "single-use-1234567890";
    let token_hash = sha256_hex(raw_token);
    let now = chrono::Utc::now().timestamp();
    argo_store::user_account::insert_magic_link(
        store.pool(),
        &token_hash,
        "carol@example.com",
        now + 900,
    )
    .await
    .unwrap();
    let r = req(
        "POST /api/v1/auth/verify",
        None,
        &format!(r#"{{"token":"{raw_token}"}}"#),
    );
    let first = verify_outcome(&r, Some(&store)).await;
    assert!(first.status.starts_with("200"));
    let second = verify_outcome(&r, Some(&store)).await;
    assert!(
        second.status.starts_with("401"),
        "{} {}",
        second.status,
        second.body
    );
    assert!(second.body.contains("already_used"));
    assert!(second.set_cookie.is_none());
}

#[tokio::test]
async fn verify_expired_token_returns_401() {
    let store = fresh_store().await;
    let raw_token = "expired-token-xxxxxxxxxxxxxxxxxxxxxxxxx";
    let token_hash = sha256_hex(raw_token);
    let now = chrono::Utc::now().timestamp();
    argo_store::user_account::insert_magic_link(
        store.pool(),
        &token_hash,
        "dan@example.com",
        now - 100,
    )
    .await
    .unwrap();
    let r = req(
        "POST /api/v1/auth/verify",
        None,
        &format!(r#"{{"token":"{raw_token}"}}"#),
    );
    let outcome = verify_outcome(&r, Some(&store)).await;
    assert!(
        outcome.status.starts_with("401"),
        "{} {}",
        outcome.status,
        outcome.body
    );
    assert!(outcome.body.contains("expired"));
    assert!(outcome.set_cookie.is_none());
}

#[test]
fn is_verify_request_matches_only_verify() {
    assert!(is_verify_request(
        "POST /api/v1/auth/verify HTTP/1.1\r\n\r\n"
    ));
    assert!(is_verify_request(
        "POST /api/v1/auth/verify?x=1 HTTP/1.1\r\n\r\n"
    ));
    assert!(!is_verify_request(
        "POST /api/v1/auth/signup HTTP/1.1\r\n\r\n"
    ));
    assert!(!is_verify_request("GET /api/v1/auth/verify HTTP/1.1\r\n\r\n"));
}

#[test]
fn looks_like_email_accepts_normal_addresses() {
    assert!(looks_like_email("a@b.co"));
    assert!(looks_like_email("first.last+tag@example.co.uk"));
}

#[test]
fn looks_like_email_rejects_garbage() {
    assert!(!looks_like_email(""));
    assert!(!looks_like_email("nope"));
    assert!(!looks_like_email("@example.com"));
    assert!(!looks_like_email("user@.tld"));
    assert!(!looks_like_email("user@no-dot"));
}
