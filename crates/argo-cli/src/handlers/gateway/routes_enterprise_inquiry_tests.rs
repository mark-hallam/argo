//! Tests for the public enterprise-inquiry endpoint
//! (`POST /api/v1/enterprise-inquiry`).
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
    // Migrate so `enterprise_inquiry` (v89) + `audit_record` exist.
    store.migrate().await.expect("migrate");
    Arc::new(store)
}

fn req(ip: &str, body: &str) -> String {
    format!(
        "POST /api/v1/enterprise-inquiry HTTP/1.1\r\nHost: x\r\nX-Forwarded-For: {ip}\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )
}

async fn count_rows(store: &Arc<SessionStore>, email: &str) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM enterprise_inquiry WHERE email = ?")
        .bind(email)
        .fetch_one(store.pool())
        .await
        .expect("count rows")
}

async fn count_audit_rows(store: &Arc<SessionStore>) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM audit_record WHERE kind = 'enterprise.inquiry_received'")
        .fetch_one(store.pool())
        .await
        .expect("count audit rows")
}

#[test]
fn is_enterprise_inquiry_request_matches_path_and_query() {
    assert!(is_enterprise_inquiry_request(
        "POST /api/v1/enterprise-inquiry HTTP/1.1\r\n\r\n"
    ));
    assert!(is_enterprise_inquiry_request(
        "POST /api/v1/enterprise-inquiry?x=1 HTTP/1.1\r\n\r\n"
    ));
    assert!(!is_enterprise_inquiry_request(
        "POST /api/v1/enterprise-inquiries HTTP/1.1\r\n\r\n"
    ));
    assert!(!is_enterprise_inquiry_request(
        "GET /api/v1/enterprise-inquiry HTTP/1.1\r\n\r\n"
    ));
}

#[tokio::test]
async fn valid_inquiry_returns_202_inserts_row_and_writes_audit() {
    let store = fresh_store().await;
    let r = req(
        "198.51.100.20",
        r#"{"company":"Acme","name":"Alice","email":"alice@acme.com","seats":"250","message":"Q3 rollout"}"#,
    );
    let (status, body) = enterprise_inquiry_response(&r, Some(&store)).await;
    assert!(status.starts_with("202"), "status={status} body={body}");
    assert!(body.contains("\"queued\":true"), "body={body}");
    assert_eq!(count_rows(&store, "alice@acme.com").await, 1);
    // The moat applies to leads too — one hash-chained audit row per inquiry.
    assert_eq!(count_audit_rows(&store).await, 1);
}

#[tokio::test]
async fn append_only_keeps_repeat_inquiries() {
    let store = fresh_store().await;
    let r = req(
        "198.51.100.21",
        r#"{"company":"Acme","name":"Alice","email":"dup@acme.com"}"#,
    );
    let (s1, _) = enterprise_inquiry_response(&r, Some(&store)).await;
    assert!(s1.starts_with("202"));
    // Same lead again — append-only, so a SECOND row is written (no dedupe).
    let (s2, b2) = enterprise_inquiry_response(&r, Some(&store)).await;
    assert!(s2.starts_with("202"), "repeat status={s2} body={b2}");
    assert_eq!(count_rows(&store, "dup@acme.com").await, 2);
}

#[tokio::test]
async fn email_is_normalized_before_store() {
    let store = fresh_store().await;
    let (s1, _) = enterprise_inquiry_response(
        &req(
            "198.51.100.22",
            r#"{"company":"Acme","name":"Alice","email":"  Mixed@Case.com  "}"#,
        ),
        Some(&store),
    )
    .await;
    assert!(s1.starts_with("202"));
    assert_eq!(count_rows(&store, "mixed@case.com").await, 1);
}

#[tokio::test]
async fn missing_company_rejected_400_no_row() {
    let store = fresh_store().await;
    let (status, _) = enterprise_inquiry_response(
        &req(
            "198.51.100.23",
            r#"{"name":"Alice","email":"a@b.com"}"#,
        ),
        Some(&store),
    )
    .await;
    assert!(status.starts_with("400"), "status={status}");
    assert_eq!(count_rows(&store, "a@b.com").await, 0);
    assert_eq!(count_audit_rows(&store).await, 0);
}

#[tokio::test]
async fn missing_name_rejected_400_no_row() {
    let store = fresh_store().await;
    let (status, _) = enterprise_inquiry_response(
        &req(
            "198.51.100.24",
            r#"{"company":"Acme","email":"a@b.com"}"#,
        ),
        Some(&store),
    )
    .await;
    assert!(status.starts_with("400"), "status={status}");
    assert_eq!(count_rows(&store, "a@b.com").await, 0);
}

#[tokio::test]
async fn malformed_email_rejected_400_no_row() {
    let store = fresh_store().await;
    let (status, _) = enterprise_inquiry_response(
        &req(
            "198.51.100.25",
            r#"{"company":"Acme","name":"Alice","email":"not-an-email"}"#,
        ),
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
        enterprise_inquiry_response(&req("198.51.100.26", "not json at all"), Some(&store)).await;
    assert!(status.starts_with("400"), "status={status}");
}

#[tokio::test]
async fn honeypot_filled_is_silently_accepted_no_row() {
    let store = fresh_store().await;
    let (status, body) = enterprise_inquiry_response(
        &req(
            "198.51.100.27",
            r#"{"company":"Acme","name":"Bot","email":"bot@acme.com","honeypot_url":"http://spam"}"#,
        ),
        Some(&store),
    )
    .await;
    // Silent OK so the bot can't detect the trap — 202, but no row + no audit.
    assert!(status.starts_with("202"), "status={status} body={body}");
    assert_eq!(count_rows(&store, "bot@acme.com").await, 0);
    assert_eq!(count_audit_rows(&store).await, 0);
}

#[tokio::test]
async fn rate_limit_rejects_after_burst() {
    let store = fresh_store().await;
    // signup_limiter() allows 5/IP/hour; the 6th from the same IP is denied.
    let ip = "198.51.100.28";
    for i in 0..5 {
        let body = format!(
            r#"{{"company":"Acme","name":"Alice","email":"burst{i}@acme.com"}}"#
        );
        let (status, _) = enterprise_inquiry_response(&req(ip, &body), Some(&store)).await;
        assert!(status.starts_with("202"), "burst {i} status={status}");
    }
    let (status, _) = enterprise_inquiry_response(
        &req(ip, r#"{"company":"Acme","name":"Alice","email":"over@acme.com"}"#),
        Some(&store),
    )
    .await;
    assert!(status.starts_with("429"), "6th status={status}");
    assert_eq!(count_rows(&store, "over@acme.com").await, 0);
}

#[tokio::test]
async fn missing_store_returns_500() {
    let (status, _) = enterprise_inquiry_response(
        &req("198.51.100.29", r#"{"company":"Acme","name":"Alice","email":"a@b.com"}"#),
        None,
    )
    .await;
    assert!(status.starts_with("500"), "status={status}");
}
