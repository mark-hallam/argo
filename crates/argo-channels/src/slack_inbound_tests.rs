//! Unit tests for [`super::SlackInbound`].
//!
//! Covers the four documented invariants: valid signature accepts,
//! bad signature rejects, stale timestamp rejects, valid token decodes
//! into [`super::ApprovalTokenClaims`].

use super::{sign_slack_v0, InboundChannel, SlackInbound, SlackInboundConfig};
use crate::token::{sign_approval_token, ApprovalDecision};
use chrono::Utc;

fn config() -> SlackInboundConfig {
    SlackInboundConfig::new("test-signing-secret", "test-hmac-secret")
}

fn signed_body(payload_json: &str) -> String {
    let mut encoded = String::from("payload=");
    for b in payload_json.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(b as char);
            }
            _ => encoded.push_str(&format!("%{b:02X}")),
        }
    }
    encoded
}

#[test]
fn valid_signature_accepts() {
    let inbound = SlackInbound::new(config());
    let body = signed_body(r#"{"actions":[{"action_id":"approve","value":"x"}]}"#);
    let ts = Utc::now().timestamp().to_string();
    let sig = sign_slack_v0(body.as_bytes(), &ts, &inbound.config().signing_secret).unwrap();
    let headers = [
        ("X-Slack-Signature", sig.as_str()),
        ("X-Slack-Request-Timestamp", ts.as_str()),
    ];
    inbound
        .verify_signature(body.as_bytes(), &headers)
        .expect("signature should verify");
}

#[test]
fn bad_signature_rejects() {
    let inbound = SlackInbound::new(config());
    let body = signed_body(r#"{"actions":[{"action_id":"approve","value":"x"}]}"#);
    let ts = Utc::now().timestamp().to_string();
    let bad_sig = "v0=deadbeef";
    let headers = [
        ("X-Slack-Signature", bad_sig),
        ("X-Slack-Request-Timestamp", ts.as_str()),
    ];
    let err = inbound
        .verify_signature(body.as_bytes(), &headers)
        .expect_err("bad signature must reject");
    let msg = err.to_string();
    assert!(msg.contains("signature"), "got: {msg}");
}

#[test]
fn stale_timestamp_rejects() {
    let inbound = SlackInbound::new(config());
    let body = signed_body(r#"{"actions":[{"action_id":"approve","value":"x"}]}"#);
    let stale_ts = (Utc::now().timestamp() - 600).to_string();
    let sig = sign_slack_v0(body.as_bytes(), &stale_ts, &inbound.config().signing_secret).unwrap();
    let headers = [
        ("X-Slack-Signature", sig.as_str()),
        ("X-Slack-Request-Timestamp", stale_ts.as_str()),
    ];
    let err = inbound
        .verify_signature(body.as_bytes(), &headers)
        .expect_err("stale timestamp must reject");
    assert!(err.to_string().contains("skew"), "got: {err}");
}

#[test]
fn token_decode_round_trips() {
    let inbound = SlackInbound::new(config());
    let exp = Utc::now().timestamp() + 600;
    let token = sign_approval_token(
        "act-42",
        ApprovalDecision::Approve,
        exp,
        &inbound.config().hmac_secret,
    )
    .unwrap();
    let payload = format!(
        r#"{{"actions":[{{"action_id":"approve","value":"{token}"}}]}}"#
    );
    let body = signed_body(&payload);
    let claims = inbound
        .decode_approval_token(body.as_bytes())
        .expect("token decode");
    assert_eq!(claims.approval_id, "act-42");
    assert_eq!(claims.decision, ApprovalDecision::Approve);
    assert_eq!(claims.exp, exp);
}

#[test]
fn missing_actions_returns_parse_error() {
    let inbound = SlackInbound::new(config());
    let body = signed_body(r#"{"actions":[]}"#);
    let err = inbound
        .decode_approval_token(body.as_bytes())
        .expect_err("empty actions must error");
    assert!(err.to_string().contains("actions"), "got: {err}");
}
