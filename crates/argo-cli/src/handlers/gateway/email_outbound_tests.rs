//! Tests for the outbound-email provider abstraction.

use super::{send, OutboundMessage};
use argo_config::{EmailOutboundConfig, EmailOutboundProvider};
use argo_email_templates::MagicLinkEmail;

fn sample_rendered() -> argo_email_templates::RenderedEmail {
    MagicLinkEmail {
        magic_url: "https://app.skycot.com/auth/verify?token=abc".to_string(),
        email: "ada@example.com".to_string(),
        expires_in: "15 minutes".to_string(),
    }
    .render()
}

#[tokio::test]
async fn noop_provider_succeeds_without_network() {
    let cfg = EmailOutboundConfig {
        provider: EmailOutboundProvider::Noop,
        ..EmailOutboundConfig::default()
    };
    let rendered = sample_rendered();
    let msg = OutboundMessage {
        to: "ada@example.com",
        from: "noreply@skycot.com",
        rendered: &rendered,
    };
    send(&cfg, msg).await.expect("noop never fails");
}

#[tokio::test]
async fn postmark_requires_token() {
    let cfg = EmailOutboundConfig {
        provider: EmailOutboundProvider::Postmark,
        from_address: "noreply@skycot.com".into(),
        ..EmailOutboundConfig::default()
    };
    let rendered = sample_rendered();
    let msg = OutboundMessage {
        to: "ada@example.com",
        from: "noreply@skycot.com",
        rendered: &rendered,
    };
    let err = send(&cfg, msg).await.expect_err("missing token must error");
    assert!(err.contains("token"), "unexpected error: {err}");
}

#[tokio::test]
async fn sendgrid_requires_api_key() {
    let cfg = EmailOutboundConfig {
        provider: EmailOutboundProvider::Sendgrid,
        from_address: "noreply@skycot.com".into(),
        ..EmailOutboundConfig::default()
    };
    let rendered = sample_rendered();
    let msg = OutboundMessage {
        to: "ada@example.com",
        from: "noreply@skycot.com",
        rendered: &rendered,
    };
    let err = send(&cfg, msg)
        .await
        .expect_err("missing api key must error");
    assert!(err.contains("api key"), "unexpected error: {err}");
}

#[tokio::test]
async fn smtp_requires_host() {
    let cfg = EmailOutboundConfig {
        provider: EmailOutboundProvider::Smtp,
        ..EmailOutboundConfig::default()
    };
    let rendered = sample_rendered();
    let msg = OutboundMessage {
        to: "ada@example.com",
        from: "noreply@skycot.com",
        rendered: &rendered,
    };
    let err = send(&cfg, msg).await.expect_err("missing host must error");
    assert!(err.contains("host"), "unexpected error: {err}");
}

#[test]
fn provider_as_str_round_trips() {
    assert_eq!(EmailOutboundProvider::Noop.as_str(), "noop");
    assert_eq!(EmailOutboundProvider::Postmark.as_str(), "postmark");
    assert_eq!(EmailOutboundProvider::Sendgrid.as_str(), "sendgrid");
    assert_eq!(EmailOutboundProvider::Smtp.as_str(), "smtp");
}
