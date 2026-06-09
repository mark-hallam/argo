//! Error-hygiene regression tests for the channel approval webhooks
//! (e2e finding R3-F3 / R1-F1).
//!
//! A missing/empty server-side channel secret reachable by an
//! UNAUTHENTICATED caller must NOT surface as a `500 Internal Server
//! Error` that names the unset env var (wrong status class + minor
//! information disclosure). It must be a `401`/`503` with a generic
//! message that never echoes `ARGO_CHANNEL_HMAC_SECRET`,
//! `TWILIO_AUTH_TOKEN`, or `SLACK_APPROVALS_SIGNING_SECRET`.
//!
//! These tests are deliberately NON-mutating on the channel-secret env
//! vars: those vars are process-global and other tests in this crate
//! set/remove them concurrently, so toggling them here would flake the
//! whole suite. Instead the assertions hold for BOTH reachable states —
//! secret-unset → generic `503`; secret-present → `401` on the missing
//! signature/token. Neither path may `500` or name an env var, which is
//! exactly the acceptance criterion. The dedicated `not_configured`
//! helper test below pins the `503`-generic mapping directly.

use super::routes_channels::not_configured;

const SECRET_VARS: [&str; 3] = [
    "ARGO_CHANNEL_HMAC_SECRET",
    "TWILIO_AUTH_TOKEN",
    "SLACK_APPROVALS_SIGNING_SECRET",
];

/// Assert an unauthenticated webhook hit is `401`/`503` (never `500`)
/// and leaks no env-var name, regardless of whether the secret is set.
fn assert_hygienic(channel: &str, status: &str, body: &str) {
    assert_ne!(
        status, "500 Internal Server Error",
        "{channel}: config-absence must not be a 500 (body: {body})"
    );
    assert!(
        status.starts_with("401") || status.starts_with("503"),
        "{channel}: expected 401/503, got {status} (body: {body})"
    );
    for var in SECRET_VARS {
        assert!(
            !body.contains(var),
            "{channel}: response leaked env-var name {var} (body: {body})"
        );
    }
}

#[test]
fn not_configured_helper_is_a_generic_503() {
    let (status, body) = not_configured();
    assert_eq!(status, "503 Service Unavailable");
    for var in SECRET_VARS {
        assert!(!body.contains(var), "helper leaked {var}: {body}");
    }
}

#[tokio::test]
async fn channel_webhooks_anonymous_hit_is_401_or_503_never_500() {
    let (status, body) = super::routes_channels_email::handle_email_webhook(
        "POST /channels/email/webhook HTTP/1.1\r\nHost: x\r\n\r\n{}",
    )
    .await;
    assert_hygienic("email", status, &body);

    let (status, body) = super::routes_channels_push::push_response(
        "POST /channels/push/decision/tok HTTP/1.1\r\nHost: x\r\n\r\n",
    )
    .await;
    assert_hygienic("push", status, &body);

    let (status, body) = super::routes_channels_slack::slack_webhook_response(
        "POST /channels/slack/webhook HTTP/1.1\r\nHost: x\r\n\r\n",
    )
    .await;
    assert_hygienic("slack", status, &body);

    let (status, body) = super::routes_channels_sms::sms_webhook_response(
        "POST /twilio/sms/decision HTTP/1.1\r\nHost: x\r\n\r\n",
    )
    .await;
    assert_hygienic("sms", status, &body);

    let (status, body) = super::routes_channels_voice::voice_webhook_response(
        "POST /twilio/voice/decision HTTP/1.1\r\nHost: x\r\n\r\n",
    )
    .await;
    assert_hygienic("voice", status, &body);

    let (status, body) = super::routes_channels_whatsapp::whatsapp_webhook_response(
        "POST /twilio/whatsapp/decision HTTP/1.1\r\nHost: x\r\n\r\n",
    )
    .await;
    assert_hygienic("whatsapp", status, &body);

    // Discord is the reference pattern: it pre-checks the signature
    // headers and returns 401 BEFORE any secret lookup, so an anonymous
    // probe never reaches the config state. Included as a positive
    // control (deterministic 401, no env dependency).
    let (status, body) = super::routes_channels_discord::discord_decision_response(
        "POST /channels/discord/decision HTTP/1.1\r\nHost: x\r\n\r\n",
    )
    .await;
    assert_hygienic("discord", status, &body);
}
