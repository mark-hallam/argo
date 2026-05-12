//! Env-gated smoke test against the real Resend API.
//!
//! Skipped unless **all three** env vars are set:
//!
//! - `ARGO_TEST_RESEND_API_KEY` — a `re_…` key with `send_email` scope
//! - `ARGO_TEST_RESEND_FROM_EMAIL` — must be a verified domain in the
//!   Resend account this key belongs to (the test cannot create
//!   verified domains; the operator must publish DNS records first)
//! - `ARGO_TEST_RESEND_TO_EMAIL` — any address you control / will not
//!   pollute (e.g. a temp inbox); send_count gets billed on success
//!
//! The test sends one HTML email and polls `get_email` every 1s until
//! `last_event ∈ {sent, delivered}` or 10s elapse (Resend usually flips
//! `sent → delivered` within a couple of seconds). It then validates
//! that the typed status fields (`id`, `from`, `to`, `subject`,
//! `last_event`, `created_at`) are populated.
//!
//! Run locally with:
//! ```bash
//! ARGO_TEST_RESEND_API_KEY=re_… \
//!   ARGO_TEST_RESEND_FROM_EMAIL='Argo <noreply@yourdomain.com>' \
//!   ARGO_TEST_RESEND_TO_EMAIL=you@example.com \
//!   cargo test -p argo-mcp-connector-resend --test resend_smoke -- --ignored --nocapture
//! ```

use argo_mcp_connector_resend::{ResendClient, ResendEmail, ResendRegion};

#[tokio::test]
#[ignore = "env-gated; requires ARGO_TEST_RESEND_API_KEY/FROM_EMAIL/TO_EMAIL"]
async fn smoke_send_and_poll_delivery() {
    let api_key = match std::env::var("ARGO_TEST_RESEND_API_KEY") {
        Ok(v) if !v.is_empty() => v,
        _ => {
            eprintln!("skip: ARGO_TEST_RESEND_API_KEY unset");
            return;
        }
    };
    let from = std::env::var("ARGO_TEST_RESEND_FROM_EMAIL")
        .expect("set ARGO_TEST_RESEND_FROM_EMAIL to a verified Resend domain");
    let to = std::env::var("ARGO_TEST_RESEND_TO_EMAIL")
        .expect("set ARGO_TEST_RESEND_TO_EMAIL to an address you control");
    let region = std::env::var("ARGO_RESEND_REGION").unwrap_or_default();
    let region = ResendRegion::parse(&region).expect("region: us|eu");
    let client = ResendClient::new(api_key, region);

    let id = client
        .send_email(ResendEmail {
            from,
            to: vec![to],
            subject: "argo-mcp-connector-resend smoke".into(),
            html: Some("<p>argo smoke test — safe to ignore.</p>".into()),
            text: None,
            reply_to: None,
            cc: vec![],
            bcc: vec![],
            tags: vec![],
        })
        .await
        .expect("send");

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    let mut last_status = None;
    while std::time::Instant::now() < deadline {
        match client.get_email(id.as_str()).await {
            Ok(status) => {
                let event = status.last_event.clone();
                last_status = Some(status);
                if matches!(event.as_str(), "sent" | "delivered") {
                    break;
                }
            }
            Err(e) => {
                // Resend may return 404 for the first poll tick before
                // the message lands in the lookup index — keep polling.
                eprintln!("get_email: {e}");
            }
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }

    let status = last_status.expect("no get_email response within 10s");
    assert!(!status.id.is_empty(), "id populated");
    assert!(!status.from.is_empty(), "from populated");
    assert!(!status.to.is_empty(), "to populated");
    assert!(!status.subject.is_empty(), "subject populated");
    assert!(!status.last_event.is_empty(), "last_event populated");
    assert!(!status.created_at.is_empty(), "created_at populated");
}
