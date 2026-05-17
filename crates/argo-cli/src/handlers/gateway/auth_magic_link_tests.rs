//! Tests for the magic-link signup + verify endpoints.

use super::*;
use argo_store::SessionStore;
use std::sync::Arc;
use std::time::{Duration, Instant};

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
async fn signup_rate_limit_hits_429_on_sixth_attempt() {
    // Drive the limiter directly so the test isn't racy on wall-clock.
    let now = Instant::now();
    let ip = "203.0.113.20";
    for _ in 0..IP_LIMIT {
        assert!(ip_limit_check_at(ip, now));
    }
    assert!(!ip_limit_check_at(ip, now), "6th attempt must be denied");
    let later = now + IP_WINDOW + Duration::from_secs(1);
    assert!(ip_limit_check_at(ip, later));
}

#[tokio::test]
async fn verify_with_invalid_token_returns_401() {
    let store = fresh_store().await;
    let r = req(
        "POST /api/v1/auth/verify",
        None,
        r#"{"token":"never-issued"}"#,
    );
    let (status, body) = verify_response(&r, Some(&store)).await;
    assert!(status.starts_with("401"), "status={status} body={body}");
    assert!(body.contains("\"invalid\""));
}

#[tokio::test]
async fn signup_then_verify_round_trip() {
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
    let (status, body) = verify_response(&verify_req, Some(&store)).await;
    assert!(status.starts_with("200"), "status={status} body={body}");
    assert!(body.contains("\"needs_onboarding\":true"));
    assert!(body.contains("\"jwt\""));
    assert!(body.contains("\"skycot_session\""));
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
    let (s1, _) = verify_response(&r, Some(&store)).await;
    assert!(s1.starts_with("200"));
    let (s2, b2) = verify_response(&r, Some(&store)).await;
    assert!(s2.starts_with("401"), "{s2} {b2}");
    assert!(b2.contains("already_used"));
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
    let (status, body) = verify_response(&r, Some(&store)).await;
    assert!(status.starts_with("401"), "{status} {body}");
    assert!(body.contains("expired"));
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
