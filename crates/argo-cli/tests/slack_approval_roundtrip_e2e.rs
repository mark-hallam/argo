//! End-to-end roundtrip test for the Slack approval webhook path.
//!
//! `crates/argo-cli/src/handlers/gateway/routes_channels_slack.rs`
//! already owns the HTTP wiring (signature verify → token decode →
//! `worker_actions_transitions::approve`). The gateway-internal
//! transitions API is `pub(crate)`, so this integration test exercises
//! the public seam an external integrator (or the gateway handler
//! itself) would compose: load a recorded Slack webhook payload, sign
//! it with the documented Slack v0 envelope, then prove
//! [`argo_channels::slack_inbound::SlackInbound`] verifies the
//! signature and decodes the embedded approval token into the same
//! [`argo_channels::token::ApprovalTokenClaims`] the gateway feeds to
//! the state machine. Failure modes (bad signature, stale skew) are
//! covered alongside the happy path so a regression in either the
//! envelope or the token format breaks this test.

use argo_channels::slack_inbound::{
    sign_slack_v0, InboundChannel, SlackInbound, SlackInboundConfig,
};
use argo_channels::token::{sign_approval_token, ApprovalDecision};
use chrono::Utc;
use std::path::PathBuf;

const SIGNING_SECRET: &str = "test-signing-secret";
const HMAC_SECRET: &str = "test-hmac-secret";
const ACTION_ID: &str = "act-mvp-roundtrip";

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

fn load_fixture_with_token(token: &str) -> String {
    let raw = std::fs::read_to_string(fixtures_dir().join("slack_approval_webhook.json"))
        .expect("read fixture");
    raw.replace("TOKEN_PLACEHOLDER", token)
}

fn url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 2);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn build_signed_request(token: &str) -> (String, String, String) {
    let payload_json = load_fixture_with_token(token);
    let body = format!("payload={}", url_encode(&payload_json));
    let ts = Utc::now().timestamp().to_string();
    let sig = sign_slack_v0(body.as_bytes(), &ts, SIGNING_SECRET).expect("sign v0");
    (body, ts, sig)
}

fn inbound() -> SlackInbound {
    SlackInbound::new(SlackInboundConfig::new(SIGNING_SECRET, HMAC_SECRET))
}

#[test]
fn recorded_fixture_round_trips_through_inbound_channel() {
    // Sign an approval token under the documented format used by
    // outbound notify (#931). The recorded fixture stamps it into the
    // Slack action button's `value` field — exactly what Slack echoes
    // back when an operator clicks Approve.
    let exp = Utc::now().timestamp() + 600;
    let token = sign_approval_token(ACTION_ID, ApprovalDecision::Approve, exp, HMAC_SECRET)
        .expect("sign approval token");

    let (body, ts, sig) = build_signed_request(&token);
    let inbound = inbound();
    let headers = [
        ("X-Slack-Signature", sig.as_str()),
        ("X-Slack-Request-Timestamp", ts.as_str()),
    ];
    inbound
        .verify_signature(body.as_bytes(), &headers)
        .expect("recorded fixture must verify");
    let claims = inbound
        .decode_approval_token(body.as_bytes())
        .expect("recorded fixture must decode");
    assert_eq!(claims.approval_id, ACTION_ID);
    assert_eq!(claims.decision, ApprovalDecision::Approve);
    assert_eq!(claims.exp, exp);
}

#[test]
fn tampered_body_fails_signature_check() {
    let exp = Utc::now().timestamp() + 600;
    let token = sign_approval_token(ACTION_ID, ApprovalDecision::Approve, exp, HMAC_SECRET)
        .expect("sign approval token");
    let (body, ts, sig) = build_signed_request(&token);
    // Operator tampering: append junk after signing. The Slack v0
    // envelope binds the body byte-for-byte, so this must reject.
    let tampered = format!("{body}&hax=1");
    let inbound = inbound();
    let headers = [
        ("X-Slack-Signature", sig.as_str()),
        ("X-Slack-Request-Timestamp", ts.as_str()),
    ];
    let err = inbound
        .verify_signature(tampered.as_bytes(), &headers)
        .expect_err("tampered body must reject");
    assert!(err.to_string().contains("signature"), "got: {err}");
}

#[test]
fn stale_recorded_request_rejected() {
    let exp = Utc::now().timestamp() + 600;
    let token = sign_approval_token(ACTION_ID, ApprovalDecision::Approve, exp, HMAC_SECRET)
        .expect("sign approval token");
    let payload_json = load_fixture_with_token(&token);
    let body = format!("payload={}", url_encode(&payload_json));
    // 6 minutes old — past the documented 5-minute skew window.
    let stale_ts = (Utc::now().timestamp() - 360).to_string();
    let sig = sign_slack_v0(body.as_bytes(), &stale_ts, SIGNING_SECRET).expect("sign");
    let inbound = inbound();
    let headers = [
        ("X-Slack-Signature", sig.as_str()),
        ("X-Slack-Request-Timestamp", stale_ts.as_str()),
    ];
    let err = inbound
        .verify_signature(body.as_bytes(), &headers)
        .expect_err("stale fixture must reject");
    assert!(err.to_string().contains("skew"), "got: {err}");
}
