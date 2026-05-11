//! End-to-end smoke test against the live Mailgun API.
//!
//! Runs only when all three of these env vars are set:
//!
//! - `ARGO_MAILGUN_TEST_KEY`       — Mailgun private API key (`key-…`)
//! - `ARGO_MAILGUN_TEST_DOMAIN`    — Mailgun verified sending domain
//! - `ARGO_MAILGUN_TEST_RECIPIENT` — recipient address that won't bounce
//!
//! Optionally set `ARGO_MAILGUN_TEST_REGION=eu` to target the EU endpoint.
//! Skipped silently otherwise so default `cargo test` doesn't hit the
//! network or surprise developers without credentials.

use argo_mcp_connector_mailgun::{MailgunConnector, MailgunMessage, MailgunRegion};

fn env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|s| !s.trim().is_empty())
}

#[tokio::test]
async fn live_send_email_returns_message_id() {
    let (Some(api_key), Some(domain), Some(recipient)) = (
        env("ARGO_MAILGUN_TEST_KEY"),
        env("ARGO_MAILGUN_TEST_DOMAIN"),
        env("ARGO_MAILGUN_TEST_RECIPIENT"),
    ) else {
        eprintln!(
            "live_send_email_returns_message_id: missing env (KEY/DOMAIN/RECIPIENT) — skipping"
        );
        return;
    };
    let region = match env("ARGO_MAILGUN_TEST_REGION").as_deref() {
        Some("eu") | Some("EU") => MailgunRegion::Eu,
        _ => MailgunRegion::Us,
    };
    let connector = MailgunConnector::new(api_key, domain, region);
    let msg = MailgunMessage {
        from: format!("argo-smoke <noreply@{}>", connector_domain_for_from(&connector)),
        to: vec![recipient],
        cc: vec![],
        bcc: vec![],
        subject: "argo-mcp-connector-mailgun smoke".into(),
        text: Some("This message was sent by the Argo Mailgun connector smoke test.".into()),
        html: None,
        reply_to: None,
        tags: vec!["argo-smoke".into()],
    };
    let resp = connector
        .send_email(msg)
        .await
        .expect("smoke send should succeed");
    assert!(
        !resp.message_id().is_empty(),
        "Mailgun returned empty message id"
    );
    assert!(
        resp.message.contains("Queued") || !resp.message.is_empty(),
        "unexpected confirmation message: {}",
        resp.message
    );
}

/// We don't expose `domain` on the public connector type — the smoke test
/// just re-reads it from the env to build a plausible `From:` header.
fn connector_domain_for_from(_c: &MailgunConnector) -> String {
    std::env::var("ARGO_MAILGUN_TEST_DOMAIN").unwrap_or_else(|_| "example.com".into())
}
