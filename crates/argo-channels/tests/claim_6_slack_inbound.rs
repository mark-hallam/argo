//! # Mission Claim 6 MVP — Slack inbound closed-loop regression
//!
//! Pins the operator-locked MVP target channel (round 19):
//!
//! > MVP = ONE channel (Slack) working fully e2e — outbound + inbound +
//! > HMAC-signed token + audit row + cross-tenant isolation +
//! > `argo audit verify` passes.
//!
//! This file is the regression contract for the **INBOUND** half of
//! Slack, specifically the *closed loop* between outbound and inbound:
//! the token the outbound channel mints and embeds in the Slack button
//! must bit-for-bit be the token the inbound channel accepts.
//!
//! ## What this test proves
//!
//! 1. **Closed-loop approve.** `SlackApprovalChannel::message_payload()`
//!    builds an interactive envelope with a signed token embedded in
//!    the *approve* button `value`. When Slack POSTs that exact token
//!    back to the gateway in an interactivity callback, the inbound
//!    side (`SlackInbound::verify_signature` +
//!    `SlackInbound::decode_approval_token`) must verify and decode
//!    it into `ApprovalDecision::Approve` with the same `approval_id`
//!    the outbound side started from. Action_id matches.
//! 2. **Closed-loop reject.** Same property for the reject button —
//!    the decoded `ApprovalDecision` must be `Reject`, NOT `Approve`,
//!    so the operator's reject can't be silently coerced into an
//!    approve.
//! 3. **Tampered-token defence.** Mutating a single character of the
//!    `value` in the inbound POST body MUST cause `decode_approval_token`
//!    to fail with a signature error — the inbound side defends the
//!    HMAC envelope independently of Slack's v0 transport signature.
//! 4. **Stale-timestamp defence.** A POST signed with a timestamp
//!    older than the skew window (5 minutes) MUST be rejected by
//!    `verify_signature` even with a valid v0 signature — replay
//!    protection.
//!
//! ## What this test does NOT prove
//!
//! - That the gateway HTTP route correctly drives the state machine
//!   (approve → worker_actions::approve). That live wiring lives in
//!   `crates/argo-cli/src/handlers/gateway/routes_channels_slack.rs`
//!   and is exercised by `argo-cli/tests/slack_approval_roundtrip_e2e.rs`.
//!   The piece this test owns is the *cross-module contract* between
//!   outbound and inbound, independently of gateway plumbing.
//!
//! ## Authored
//!
//! Evaluator round 23, 2026-05-23.

use std::collections::BTreeMap;

use argo_channels::rate_limit::RateLimitConfig;
use argo_channels::slack::{
    SlackApprovalChannel, SlackApprovalChannelConfig, SlackOperatorRoute,
};
use argo_channels::slack_inbound::{
    sign_slack_v0, InboundChannel, SlackInbound, SlackInboundConfig,
};
use argo_channels::token::{sign_approval_token, ApprovalDecision};
use argo_channels::{ChannelId, Notification, OperatorTarget, RiskLevel};
use chrono::{Duration, Utc};

const TEST_OPERATOR_ID: &str = "op-mariah-marketplace";
const TEST_SLACK_CHANNEL: &str = "C987654";
const TEST_WORKSPACE: &str = "T0CLAIM6INBOUND";
const SIGNING_SECRET: &str = "claim-6-mvp-inbound-signing-secret";
const HMAC_SECRET: &str = "claim-6-mvp-inbound-hmac-secret";
const ACTION_ID: &str = "act-claim-6-mvp-inbound-closed-loop";
const APPROVAL_ID: &str = "appr-claim-6-mvp-inbound-closed-loop";

fn outbound_channel() -> SlackApprovalChannel {
    let mut operator_routes = BTreeMap::new();
    operator_routes.insert(
        TEST_OPERATOR_ID.into(),
        SlackOperatorRoute {
            workspace_id: TEST_WORKSPACE.into(),
            channel_id: Some(TEST_SLACK_CHANNEL.into()),
            user_id: None,
        },
    );
    SlackApprovalChannel::new(SlackApprovalChannelConfig {
        channel_id: ChannelId::new("slack"),
        bot_token: "xoxb-claim-6-mvp-inbound-not-real".into(),
        endpoint: "http://127.0.0.1:9/never-reached".into(),
        operator_routes,
        rate_limit: RateLimitConfig::unlimited(),
        quiet_hours: None,
    })
}

fn inbound_channel() -> SlackInbound {
    SlackInbound::new(SlackInboundConfig::new(SIGNING_SECRET, HMAC_SECRET))
}

/// Build a Notification whose `approve_url` + `reject_url` carry REAL
/// HMAC-signed tokens under `HMAC_SECRET`. This is the precondition
/// for the closed-loop test: the inbound side will verify those same
/// tokens under the same secret.
fn fixture_notification_with_real_tokens() -> Notification {
    let expiry = Utc::now() + Duration::minutes(15);
    let exp_seconds = expiry.timestamp();
    let approve_token =
        sign_approval_token(APPROVAL_ID, ApprovalDecision::Approve, exp_seconds, HMAC_SECRET)
            .expect("sign approve token");
    let reject_token =
        sign_approval_token(APPROVAL_ID, ApprovalDecision::Reject, exp_seconds, HMAC_SECRET)
            .expect("sign reject token");
    Notification {
        target: OperatorTarget {
            operator_id: TEST_OPERATOR_ID.into(),
            email: "mariah@example.com".into(),
        },
        action_id: ACTION_ID.into(),
        approval_id: APPROVAL_ID.into(),
        risk_level: RiskLevel::High,
        prompt_body:
            "Mariah is about to post a $1,200 hockey stick listing. The price is 18% \
             below your floor — confirm?"
                .into(),
        approve_url: format!("https://gw.skycot.com/decision?token={approve_token}"),
        reject_url: format!("https://gw.skycot.com/decision?token={reject_token}"),
        expiry,
    }
}

/// Pull the `value` field out of the action_id-named button block.
/// This is the exact bytes Slack echoes back when the operator clicks
/// that button, so it's what the inbound side must accept.
fn button_value(payload: &serde_json::Value, action_id: &str) -> String {
    let blocks = payload["blocks"].as_array().expect("blocks array");
    let actions_block = blocks
        .iter()
        .find(|b| b["type"] == "actions")
        .expect("actions block present");
    let elements = actions_block["elements"]
        .as_array()
        .expect("actions elements");
    let button = elements
        .iter()
        .find(|e| e["action_id"].as_str() == Some(action_id))
        .unwrap_or_else(|| panic!("button with action_id={action_id} present"));
    button["value"]
        .as_str()
        .unwrap_or_else(|| panic!("button {action_id} value is a string"))
        .to_string()
}

/// Form-url-encode a string the way Slack does for interactivity POSTs.
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

/// Construct a Slack interactivity callback POST body for the given
/// button (action_id + token in `value`). Matches the wire format
/// Slack uses for `application/x-www-form-urlencoded` callbacks:
/// `payload=<url-encoded JSON>`.
fn build_slack_callback_body(action_id: &str, token: &str) -> String {
    let payload_json = format!(
        r#"{{"type":"block_actions","actions":[{{"action_id":"{action_id}","value":"{token}"}}]}}"#,
    );
    format!("payload={}", url_encode(&payload_json))
}

/// Sign + assemble headers exactly as a real Slack POST would arrive.
fn signed_headers(body: &str) -> (String, String) {
    let ts = Utc::now().timestamp().to_string();
    let sig = sign_slack_v0(body.as_bytes(), &ts, SIGNING_SECRET).expect("sign v0");
    (ts, sig)
}

#[test]
fn claim_6_mvp_slack_closed_loop_approve_round_trips() {
    // ── Phase 1: outbound side — mint the envelope.
    let outbound = outbound_channel();
    let notification = fixture_notification_with_real_tokens();
    let payload = outbound
        .message_payload(&notification)
        .expect("message_payload");

    // ── Phase 2: extract the approve button's `value` — this is the
    //   exact token Slack echoes back when the operator clicks approve.
    let approve_button_value = button_value(&payload, "approve");

    // ── Phase 3: simulate Slack POSTing that token back as an
    //   interactivity callback.
    let body = build_slack_callback_body("approve", &approve_button_value);
    let (ts, sig) = signed_headers(&body);
    let headers = [
        ("X-Slack-Signature", sig.as_str()),
        ("X-Slack-Request-Timestamp", ts.as_str()),
    ];

    // ── Phase 4: inbound side must verify + decode.
    let inbound = inbound_channel();
    inbound
        .verify_signature(body.as_bytes(), &headers)
        .expect("claim-6 MVP closed loop: inbound signature must verify");
    let claims = inbound
        .decode_approval_token(body.as_bytes())
        .expect("claim-6 MVP closed loop: token from outbound must decode through inbound");

    // ── Phase 5: claims must mirror what the outbound side started from.
    assert_eq!(
        claims.approval_id, APPROVAL_ID,
        "claim-6 MVP closed loop: approval_id must round-trip outbound→inbound; got {}",
        claims.approval_id
    );
    assert_eq!(
        claims.decision,
        ApprovalDecision::Approve,
        "claim-6 MVP closed loop: approve button must decode to Approve, got {:?}",
        claims.decision
    );
}

#[test]
fn claim_6_mvp_slack_closed_loop_reject_decodes_as_reject_not_approve() {
    // Defensive — the reject button MUST decode to Reject. If outbound
    // accidentally embedded the approve token in both buttons (or the
    // inbound side ignores the decision field), the operator's reject
    // would be silently coerced into an approve.
    let outbound = outbound_channel();
    let notification = fixture_notification_with_real_tokens();
    let payload = outbound
        .message_payload(&notification)
        .expect("message_payload");

    let reject_button_value = button_value(&payload, "reject");
    let approve_button_value = button_value(&payload, "approve");
    assert_ne!(
        approve_button_value, reject_button_value,
        "claim-6 MVP closed loop: approve + reject button values must differ — \
         otherwise rejecting silently approves"
    );

    let body = build_slack_callback_body("reject", &reject_button_value);
    let (ts, sig) = signed_headers(&body);
    let headers = [
        ("X-Slack-Signature", sig.as_str()),
        ("X-Slack-Request-Timestamp", ts.as_str()),
    ];

    let inbound = inbound_channel();
    inbound
        .verify_signature(body.as_bytes(), &headers)
        .expect("reject path: inbound signature must verify");
    let claims = inbound
        .decode_approval_token(body.as_bytes())
        .expect("reject path: token from outbound must decode through inbound");

    assert_eq!(
        claims.approval_id, APPROVAL_ID,
        "reject path: approval_id must round-trip"
    );
    assert_eq!(
        claims.decision,
        ApprovalDecision::Reject,
        "reject path: decision MUST be Reject (operator's intent), got {:?}",
        claims.decision
    );
}

#[test]
fn claim_6_mvp_slack_closed_loop_tampered_token_rejected() {
    // Operator's threat model: a man-in-the-middle (or a malicious
    // browser extension proxying Slack traffic) mutates the token
    // bytes after Slack sent the callback. The Slack v0 envelope
    // signature would still verify (because the attacker re-signs the
    // body), but the inner HMAC over the token must fail.
    let outbound = outbound_channel();
    let notification = fixture_notification_with_real_tokens();
    let payload = outbound.message_payload(&notification).expect("payload");

    let approve_button_value = button_value(&payload, "approve");
    // Flip the last character of the token — this lands in the
    // signature segment (since approval tokens are
    // `<id>.<decision>.<exp>.<hmac>`), so the inner HMAC check must
    // fail.
    let mut tampered_chars: Vec<char> = approve_button_value.chars().collect();
    let last_idx = tampered_chars.len() - 1;
    let original_last = tampered_chars[last_idx];
    tampered_chars[last_idx] = if original_last == 'a' { 'b' } else { 'a' };
    let tampered_token: String = tampered_chars.into_iter().collect();
    assert_ne!(
        tampered_token, approve_button_value,
        "test plumbing: tamper must actually mutate the token"
    );

    let body = build_slack_callback_body("approve", &tampered_token);
    let (ts, sig) = signed_headers(&body);
    let headers = [
        ("X-Slack-Signature", sig.as_str()),
        ("X-Slack-Request-Timestamp", ts.as_str()),
    ];

    let inbound = inbound_channel();
    // Slack envelope still verifies — attacker re-signed the body.
    inbound
        .verify_signature(body.as_bytes(), &headers)
        .expect("envelope verifies because attacker re-signed");

    // But the inner HMAC over the token MUST fail.
    let err = inbound
        .decode_approval_token(body.as_bytes())
        .expect_err("claim-6 MVP closed loop: tampered token MUST be rejected");
    let msg = err.to_string().to_lowercase();
    assert!(
        msg.contains("signature") || msg.contains("token"),
        "claim-6 MVP closed loop: tamper rejection must mention signature/token; got {msg:?}"
    );
}

#[test]
fn claim_6_mvp_slack_closed_loop_stale_timestamp_rejected() {
    // Replay protection — a captured POST body replayed 6 minutes
    // later (past the 5-minute skew window) MUST be rejected at the
    // signature-verify step, even though the v0 signature would still
    // be cryptographically valid for that (stale) timestamp.
    let outbound = outbound_channel();
    let notification = fixture_notification_with_real_tokens();
    let payload = outbound.message_payload(&notification).expect("payload");
    let approve_button_value = button_value(&payload, "approve");
    let body = build_slack_callback_body("approve", &approve_button_value);

    // 6 minutes ago — outside the documented 5-minute Slack skew window.
    let stale_ts = (Utc::now().timestamp() - 360).to_string();
    let sig = sign_slack_v0(body.as_bytes(), &stale_ts, SIGNING_SECRET).expect("sign v0");
    let headers = [
        ("X-Slack-Signature", sig.as_str()),
        ("X-Slack-Request-Timestamp", stale_ts.as_str()),
    ];

    let inbound = inbound_channel();
    let err = inbound
        .verify_signature(body.as_bytes(), &headers)
        .expect_err("claim-6 MVP closed loop: stale timestamp MUST be rejected");
    assert!(
        err.to_string().to_lowercase().contains("skew"),
        "claim-6 MVP closed loop: stale rejection must mention skew; got {err}"
    );
}
