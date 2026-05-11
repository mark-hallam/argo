//! Client + webhook unit tests.
//!
//! HTTP fixtures use a one-shot in-process TCP server (the same pattern
//! `argo-mcp-connector-sendgrid` adopts) rather than `wiremock`. Avoids
//! adding a workspace dev-dep for one crate's worth of mocking.

use crate::client::{percent_encode_for_tests as percent_encode, MailgunConnector};
use crate::types::{MailgunEventKind, MailgunMessage, MailgunRegion, MailgunWebhookEvent};
use crate::webhook::{compute_signature, verify_webhook_with_skew};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

async fn serve_once(
    status: &'static str,
    response: &'static str,
) -> (String, tokio::task::JoinHandle<String>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("accept");
        let mut buf = vec![0_u8; 16_384];
        let n = socket.read(&mut buf).await.expect("read");
        let req = String::from_utf8_lossy(&buf[..n]).to_string();
        let wire = format!(
            "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{}",
            response.len(),
            response
        );
        socket.write_all(wire.as_bytes()).await.expect("write");
        req
    });
    (format!("http://{addr}"), server)
}

fn fixture_message() -> MailgunMessage {
    MailgunMessage {
        from: "Argo <noreply@example.com>".into(),
        to: vec!["a@example.com".into(), "b@example.com".into()],
        cc: vec![],
        bcc: vec![],
        subject: "Hello world".into(),
        text: Some("Body text".into()),
        html: None,
        reply_to: Some("ops@example.com".into()),
        tags: vec!["onboarding".into()],
    }
}

#[tokio::test]
async fn send_email_success_uses_basic_auth_form_body_and_repeats_recipients() {
    let (base, server) = serve_once(
        "200 OK",
        r#"{"id":"<abc@mg.example.com>","message":"Queued. Thank you."}"#,
    )
    .await;
    let client = MailgunConnector::new(
        "key-private".into(),
        "mg.example.com".into(),
        MailgunRegion::Us,
    )
    .with_base_url(base);
    let resp = client.send_email(fixture_message()).await.expect("send");
    assert_eq!(resp.message_id(), "abc@mg.example.com");
    assert_eq!(resp.message, "Queued. Thank you.");
    let req = server.await.expect("server");
    assert!(
        req.starts_with("POST /v3/mg.example.com/messages "),
        "request line: {req}"
    );
    let lower = req.to_ascii_lowercase();
    assert!(
        lower.contains("authorization: basic "),
        "missing basic auth header in: {req}"
    );
    // user `api`, pass `key-private` → base64("api:key-private") = "YXBpOmtleS1wcml2YXRl"
    // reqwest emits header names lowercased; the base64 value is case-sensitive.
    assert!(
        req.contains("YXBpOmtleS1wcml2YXRl"),
        "expected base64('api:key-private') in request: {req}"
    );
    assert!(
        lower.contains("content-type: application/x-www-form-urlencoded"),
        "wrong content-type: {req}"
    );
    let body = req.split("\r\n\r\n").nth(1).unwrap_or("");
    assert!(body.contains("to=a%40example.com"), "to[0] missing: {body}");
    assert!(body.contains("to=b%40example.com"), "to[1] missing: {body}");
    assert!(body.contains("subject=Hello+world"), "subject: {body}");
    assert!(body.contains("h%3AReply-To=ops%40example.com"), "reply-to");
    assert!(body.contains("o%3Atag=onboarding"), "tag: {body}");
}

#[tokio::test]
async fn send_email_401_surfaces_provider_error() {
    let (base, server) = serve_once("401 Unauthorized", r#"{"message":"Forbidden"}"#).await;
    let client = MailgunConnector::new(
        "bad-key".into(),
        "mg.example.com".into(),
        MailgunRegion::Us,
    )
    .with_base_url(base);
    let err = client.send_email(fixture_message()).await.expect_err("401");
    let txt = err.to_string();
    assert!(txt.contains("provider error"), "wrong variant: {txt}");
    assert!(txt.contains("401"), "status missing: {txt}");
    assert!(txt.contains("Forbidden"), "body missing: {txt}");
    let _ = server.await;
}

#[test]
fn verify_webhook_round_trip_passes() {
    let api_key = "key-private";
    let timestamp = "1700000000";
    let token = "01234567890abcdef";
    let signature = compute_signature(api_key, timestamp, token).expect("compute");
    let now = 1_700_000_000_u64; // exact match → 0 skew
    assert!(verify_webhook_with_skew(
        api_key, timestamp, token, &signature, 900, now,
    ));
}

#[test]
fn verify_webhook_rejects_bad_signature_and_replay() {
    let api_key = "key-private";
    let timestamp = "1700000000";
    let token = "tok";
    let good = compute_signature(api_key, timestamp, token).expect("compute");
    let now = 1_700_000_000_u64;

    // Bad signature → reject.
    let mut tampered = good.clone();
    tampered.replace_range(0..1, if &good[0..1] == "a" { "b" } else { "a" });
    assert!(!verify_webhook_with_skew(
        api_key, timestamp, token, &tampered, 900, now,
    ));

    // Replay older than 15min → reject even with valid signature.
    let stale_now = now + 16 * 60;
    assert!(!verify_webhook_with_skew(
        api_key, timestamp, token, &good, 900, stale_now,
    ));

    // Wrong api_key → reject (independent failure mode).
    assert!(!verify_webhook_with_skew(
        "different-key",
        timestamp,
        token,
        &good,
        900,
        now,
    ));
}

#[test]
fn region_routing_picks_correct_base_url() {
    assert_eq!(MailgunRegion::Us.base_url(), "https://api.mailgun.net");
    assert_eq!(MailgunRegion::Eu.base_url(), "https://api.eu.mailgun.net");
    let eu = MailgunConnector::new("k".into(), "d".into(), MailgunRegion::Eu);
    assert_eq!(eu.region(), MailgunRegion::Eu);
}

#[test]
fn webhook_event_deserializes_all_seven_kinds() {
    let kinds = [
        ("delivered", MailgunEventKind::Delivered),
        ("opened", MailgunEventKind::Opened),
        ("clicked", MailgunEventKind::Clicked),
        ("bounced", MailgunEventKind::Bounced),
        ("unsubscribed", MailgunEventKind::Unsubscribed),
        ("complained", MailgunEventKind::Complained),
        ("failed", MailgunEventKind::Failed),
    ];
    for (wire, want) in kinds {
        let json = format!(
            r#"{{"event":"{wire}","recipient":"r@x.com","message_id":"m1","timestamp":1.0}}"#
        );
        let evt: MailgunWebhookEvent = serde_json::from_str(&json).expect("decode");
        assert_eq!(evt.event, want, "{wire} mismatch");
    }
}

#[test]
fn percent_encode_handles_reserved_and_unicode() {
    assert_eq!(percent_encode("a b"), "a+b");
    assert_eq!(percent_encode("a@b.com"), "a%40b.com");
    assert_eq!(percent_encode("h:Reply-To"), "h%3AReply-To");
    assert_eq!(percent_encode("café"), "caf%C3%A9");
}
