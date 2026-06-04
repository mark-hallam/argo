//! Tests for the public waitlist endpoint (`POST /api/v1/waitlist`).
//!
//! Each test uses a distinct `X-Forwarded-For` IP so the process-global
//! `signup_limiter()` (5/IP/hour) cannot leak state between tests run in the
//! same binary.

use super::*;
use argo_store::SessionStore;
use std::sync::Arc;

async fn fresh_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:")
        .await
        .expect("in-memory store");
    // Migrate so `waitlist_signups` (v88) exists.
    store.migrate().await.expect("migrate");
    Arc::new(store)
}

fn req(ip: &str, body: &str) -> String {
    format!(
        "POST /api/v1/waitlist HTTP/1.1\r\nHost: x\r\nX-Forwarded-For: {ip}\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )
}

async fn count_rows(store: &Arc<SessionStore>, email: &str) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM waitlist_signups WHERE email = ?")
        .bind(email)
        .fetch_one(store.pool())
        .await
        .expect("count rows")
}

#[test]
fn is_waitlist_request_matches_path_and_query() {
    assert!(is_waitlist_request("POST /api/v1/waitlist HTTP/1.1\r\n\r\n"));
    assert!(is_waitlist_request("POST /api/v1/waitlist?x=1 HTTP/1.1\r\n\r\n"));
    assert!(!is_waitlist_request("POST /api/v1/waitlists HTTP/1.1\r\n\r\n"));
    assert!(!is_waitlist_request("GET /api/v1/waitlist HTTP/1.1\r\n\r\n"));
}

#[tokio::test]
async fn valid_signup_returns_202_and_inserts_one_row() {
    let store = fresh_store().await;
    let r = req(
        "198.51.100.10",
        r#"{"email":"a@b.com","name":"A","company":"Acme","source":"landing-hero"}"#,
    );
    let (status, body) = waitlist_response(&r, Some(&store)).await;
    assert!(status.starts_with("202"), "status={status} body={body}");
    assert!(body.contains("\"queued\":true"), "body={body}");
    assert_eq!(count_rows(&store, "a@b.com").await, 1);
}

#[tokio::test]
async fn repeat_signup_is_idempotent_202_one_row() {
    let store = fresh_store().await;
    let r = req("198.51.100.11", r#"{"email":"dup@b.com"}"#);
    let (s1, _) = waitlist_response(&r, Some(&store)).await;
    assert!(s1.starts_with("202"));
    // Same email again — dedupe via ON CONFLICT, still 202, still one row.
    let (s2, b2) = waitlist_response(&r, Some(&store)).await;
    assert!(s2.starts_with("202"), "repeat status={s2} body={b2}");
    assert_eq!(count_rows(&store, "dup@b.com").await, 1);
}

#[tokio::test]
async fn email_is_normalized_before_dedupe() {
    let store = fresh_store().await;
    let (s1, _) = waitlist_response(
        &req("198.51.100.12", r#"{"email":"Mixed@Case.com"}"#),
        Some(&store),
    )
    .await;
    assert!(s1.starts_with("202"));
    // Differently-cased + whitespace-padded — must dedupe to the same row.
    let (s2, _) = waitlist_response(
        &req("198.51.100.13", r#"{"email":"  mixed@case.com  "}"#),
        Some(&store),
    )
    .await;
    assert!(s2.starts_with("202"));
    assert_eq!(count_rows(&store, "mixed@case.com").await, 1);
}

#[tokio::test]
async fn malformed_email_rejected_400_no_row() {
    let store = fresh_store().await;
    let (status, _) = waitlist_response(
        &req("198.51.100.14", r#"{"email":"not-an-email"}"#),
        Some(&store),
    )
    .await;
    assert!(status.starts_with("400"), "status={status}");
    assert_eq!(count_rows(&store, "not-an-email").await, 0);
}

#[tokio::test]
async fn malformed_json_rejected_400() {
    let store = fresh_store().await;
    let (status, _) =
        waitlist_response(&req("198.51.100.15", "not json at all"), Some(&store)).await;
    assert!(status.starts_with("400"), "status={status}");
}

#[tokio::test]
async fn honeypot_filled_is_silently_accepted_no_row() {
    let store = fresh_store().await;
    let (status, body) = waitlist_response(
        &req(
            "198.51.100.16",
            r#"{"email":"bot@b.com","honeypot_url":"http://spam"}"#,
        ),
        Some(&store),
    )
    .await;
    // Silent OK so the bot can't detect the trap — 202, but no row written.
    assert!(status.starts_with("202"), "status={status} body={body}");
    assert_eq!(count_rows(&store, "bot@b.com").await, 0);
}

#[tokio::test]
async fn missing_store_returns_500() {
    let (status, _) = waitlist_response(&req("198.51.100.17", r#"{"email":"a@b.com"}"#), None).await;
    assert!(status.starts_with("500"), "status={status}");
}
