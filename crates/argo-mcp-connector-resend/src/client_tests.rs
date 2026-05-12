//! Client unit tests.
//!
//! HTTP fixtures use a one-shot in-process TCP server (the same pattern
//! `argo-mcp-connector-sendgrid` + `argo-mcp-connector-mailgun` adopt).
//! Avoids adding a `wiremock` workspace dev-dep for one crate's mocking.

use crate::client::{map_error_for_tests as map_error, ResendClient, BATCH_MAX};
use crate::types::{ResendEmail, ResendError, ResendRegion, ResendTag};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

async fn serve_once(
    status: &'static str,
    extra: &'static str,
    response: &'static str,
) -> (String, tokio::task::JoinHandle<String>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    let server = tokio::spawn(async move {
        let (mut s, _) = listener.accept().await.expect("accept");
        let mut buf = vec![0_u8; 16_384];
        let n = s.read(&mut buf).await.expect("read");
        let req = String::from_utf8_lossy(&buf[..n]).to_string();
        let wire = format!(
            "HTTP/1.1 {status}\r\ncontent-type: application/json\r\n{extra}content-length: {}\r\n\r\n{}",
            response.len(),
            response
        );
        s.write_all(wire.as_bytes()).await.expect("write");
        req
    });
    (format!("http://{addr}"), server)
}

fn email_html() -> ResendEmail {
    ResendEmail {
        from: "Argo <noreply@example.com>".into(),
        to: vec!["a@example.com".into(), "b@example.com".into()],
        subject: "Hello world".into(),
        html: Some("<p>Hi</p>".into()),
        text: None,
        reply_to: Some("ops@example.com".into()),
        cc: vec![],
        bcc: vec![],
        tags: vec![ResendTag { name: "campaign".into(), value: "onboarding".into() }],
    }
}

fn email_text() -> ResendEmail {
    ResendEmail {
        from: "noreply@example.com".into(),
        to: vec!["a@example.com".into()],
        subject: "Plain".into(),
        html: None,
        text: Some("Body".into()),
        reply_to: None,
        cc: vec![],
        bcc: vec![],
        tags: vec![],
    }
}

fn body_of(req: &str) -> serde_json::Value {
    serde_json::from_str(req.split("\r\n\r\n").nth(1).expect("body")).expect("json")
}

#[tokio::test]
async fn send_email_happy_html_uses_bearer_and_flat_json() {
    let (base, server) = serve_once("200 OK", "", r#"{"id":"01234567-89ab-cdef-0123-456789abcdef"}"#).await;
    let c = ResendClient::new("re_test_key".into(), ResendRegion::Us).with_base_url(base);
    let id = c.send_email(email_html()).await.expect("send");
    assert_eq!(id.as_str(), "01234567-89ab-cdef-0123-456789abcdef");
    let req = server.await.expect("server");
    assert!(req.starts_with("POST /emails "), "line: {req}");
    let lower = req.to_ascii_lowercase();
    assert!(lower.contains("authorization: bearer re_test_key"), "auth: {req}");
    assert!(lower.contains("content-type: application/json"), "ct: {req}");
    let b = body_of(&req);
    assert_eq!(b["from"], "Argo <noreply@example.com>");
    assert_eq!(b["to"][1], "b@example.com");
    assert_eq!(b["html"], "<p>Hi</p>");
    assert_eq!(b["reply_to"], "ops@example.com");
    assert_eq!(b["tags"][0]["name"], "campaign");
    assert!(b.get("text").is_none(), "text omitted: {b}");
    assert!(b.get("cc").is_none(), "cc omitted: {b}");
}

#[tokio::test]
async fn send_email_text_only_omits_html_field() {
    let (base, server) = serve_once("200 OK", "", r#"{"id":"id-text-only"}"#).await;
    let c = ResendClient::new("re_test".into(), ResendRegion::Us).with_base_url(base);
    let id = c.send_email(email_text()).await.expect("send");
    assert_eq!(id.as_str(), "id-text-only");
    let b = body_of(&server.await.expect("server"));
    assert_eq!(b["text"], "Body");
    assert!(b.get("html").is_none(), "html omitted: {b}");
}

#[tokio::test]
async fn send_batch_three_emails_returns_three_ids() {
    let (base, server) = serve_once(
        "200 OK",
        "",
        r#"{"data":[{"id":"id-a"},{"id":"id-b"},{"id":"id-c"}]}"#,
    )
    .await;
    let c = ResendClient::new("re_test".into(), ResendRegion::Us).with_base_url(base);
    let ids = c
        .send_batch(vec![email_html(), email_text(), email_html()])
        .await
        .expect("batch");
    assert_eq!(ids.len(), 3);
    assert_eq!(ids[0].as_str(), "id-a");
    assert_eq!(ids[2].as_str(), "id-c");
    let req = server.await.expect("server");
    assert!(req.starts_with("POST /emails/batch "));
    assert_eq!(body_of(&req).as_array().expect("arr").len(), 3);
}

#[tokio::test]
async fn get_email_status_delivered() {
    let response = r#"{"id":"msg-1","from":"a@e.com","to":["b@e.com"],"subject":"S","last_event":"delivered","created_at":"2026-05-12T00:00:00Z"}"#;
    let (base, server) = serve_once("200 OK", "", response).await;
    let c = ResendClient::new("re_test".into(), ResendRegion::Us).with_base_url(base);
    let status = c.get_email("msg-1").await.expect("get");
    assert_eq!(status.last_event, "delivered");
    assert_eq!(status.to, vec!["b@e.com"]);
    let req = server.await.expect("server");
    assert!(req.starts_with("GET /emails/msg-1 "), "line: {req}");
}

#[tokio::test]
async fn list_domains_2_verified_1_pending() {
    let response = r#"{"data":[
      {"id":"d1","name":"a.com","status":"verified","region":"us-east-1","created_at":"2026-01-01","records":[]},
      {"id":"d2","name":"b.com","status":"verified","region":"eu-west-1","created_at":"2026-02-01","records":[]},
      {"id":"d3","name":"c.com","status":"pending","region":"us-east-1","created_at":"2026-03-01","records":[{"record":"TXT","name":"_resend","value":"abc","status":"pending"}]}
    ]}"#;
    let (base, server) = serve_once("200 OK", "", response).await;
    let c = ResendClient::new("re_test".into(), ResendRegion::Us).with_base_url(base);
    let domains = c.list_domains().await.expect("list");
    assert_eq!(domains.len(), 3);
    assert_eq!(domains[0].status, "verified");
    assert_eq!(domains[2].status, "pending");
    assert_eq!(domains[2].records[0].record, "TXT");
    let req = server.await.expect("server");
    assert!(req.starts_with("GET /domains "));
}

#[tokio::test]
async fn rate_limited_429_honors_retry_after_header() {
    let (base, server) = serve_once(
        "429 Too Many Requests",
        "retry-after: 7\r\n",
        r#"{"name":"rate_limit_exceeded","message":"Too many requests"}"#,
    )
    .await;
    let c = ResendClient::new("re_test".into(), ResendRegion::Us).with_base_url(base);
    let err = c.send_email(email_html()).await.expect_err("429");
    match err {
        ResendError::RateLimited { retry_after_secs } => assert_eq!(retry_after_secs, 7),
        other => panic!("wrong variant: {other:?}"),
    }
    let _ = server.await;
}

#[tokio::test]
async fn auth_failed_401_bubbles_as_typed_error() {
    let (base, server) = serve_once(
        "401 Unauthorized",
        "",
        r#"{"name":"validation_error","message":"API key is invalid"}"#,
    )
    .await;
    let c = ResendClient::new("re_bad".into(), ResendRegion::Us).with_base_url(base);
    let err = c.send_email(email_html()).await.expect_err("401");
    assert!(matches!(err, ResendError::AuthFailed), "got: {err:?}");
    let _ = server.await;
}

#[test]
fn domain_not_verified_is_typed() {
    // Pure unit on the error mapper — covers the message-substring branch
    // without HTTP plumbing.
    let body =
        r#"{"name":"validation_error","message":"The 'mg.example.com' domain is not verified."}"#;
    match map_error(403, body, None) {
        ResendError::DomainNotVerified { domain } => assert_eq!(domain, "mg.example.com"),
        other => panic!("wrong variant: {other:?}"),
    }
}

#[tokio::test]
async fn batch_too_large_rejects_without_network() {
    // 101 entries → reject before any HTTP.
    let oversized = (0..=BATCH_MAX).map(|_| email_text()).collect();
    let c = ResendClient::new("re_test".into(), ResendRegion::Us);
    match c.send_batch(oversized).await.expect_err("oversized") {
        ResendError::BatchTooLarge { actual, max } => {
            assert_eq!(actual, BATCH_MAX + 1);
            assert_eq!(max, BATCH_MAX);
        }
        other => panic!("wrong variant: {other:?}"),
    }
}
