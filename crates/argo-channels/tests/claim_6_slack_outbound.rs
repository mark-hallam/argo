//! # Mission Claim 6 MVP — Slack outbound approval message shape
//!
//! Pins the operator-locked MVP target channel (round 19):
//!
//! > MVP = ONE channel (Slack) working fully e2e — outbound + inbound +
//! > HMAC-signed token + audit row + cross-tenant isolation +
//! > `argo audit verify` passes.
//!
//! This file is the regression contract for the **OUTBOUND** half of
//! Slack: the channel must produce a structurally-correct Slack
//! `chat.postMessage` envelope for an approval request.
//!
//! ## What this test proves
//!
//! 1. `SlackApprovalChannel::message_payload(&notification)` returns a
//!    valid Slack interactive-message payload.
//! 2. The payload routes to the operator's configured Slack channel.
//! 3. The payload carries BOTH an approve button AND a reject button
//!    with the correct `action_id` values (so Slack's interactivity
//!    callback can demux the operator's decision).
//! 4. The signed approve_url + reject_url are reachable inside the
//!    payload — operators can also decide via URL fallback if the
//!    interactive buttons don't work in their client.
//! 5. The `action_id` from the notification is referenced in the
//!    payload — so the gateway can correlate the operator's reply to
//!    the right worker action.
//! 6. The risk level + prompt body are surfaced — the operator gets
//!    enough context to decide without a round-trip.
//!
//! ## What this test does NOT prove
//!
//! - That the POST to a real Slack endpoint succeeds — the existing
//!   inline test `posts_to_slack_when_env_present` covers that path,
//!   gated on `SLACK_APPROVALS_BOT_TOKEN` + `SLACK_APPROVALS_TEST_CHANNEL`.
//! - That Slack inbound (operator clicks approve → gateway receives
//!   the interactivity callback → action proceeds). Inbound has its
//!   own regression test in `slack_inbound_tests.rs` at the substrate
//!   level; the end-to-end inbound test is a follow-up brief.
//!
//! ## Authored
//!
//! Evaluator round 22, 2026-05-23.

use std::collections::BTreeMap;

use argo_channels::rate_limit::RateLimitConfig;
use argo_channels::slack::{
    SlackApprovalChannel, SlackApprovalChannelConfig, SlackOperatorRoute,
};
use argo_channels::{ChannelId, Notification, OperatorTarget, RiskLevel};
use chrono::{Duration, Utc};

const TEST_OPERATOR_ID: &str = "op-mariah-marketplace";
const TEST_SLACK_CHANNEL: &str = "C987654";
const TEST_WORKSPACE: &str = "T0CLAIM6";

fn fixture_channel() -> SlackApprovalChannel {
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
        bot_token: "xoxb-claim-6-mvp-test-not-real".into(),
        endpoint: "http://127.0.0.1:9/never-reached".into(),
        operator_routes,
        rate_limit: RateLimitConfig::unlimited(),
        quiet_hours: None,
    })
}

fn fixture_notification() -> Notification {
    let expiry = Utc::now() + Duration::minutes(15);
    Notification {
        target: OperatorTarget {
            operator_id: TEST_OPERATOR_ID.into(),
            email: "mariah@example.com".into(),
        },
        action_id: "act-claim-6-mvp-outbound".into(),
        approval_id: "appr-claim-6-mvp-outbound".into(),
        risk_level: RiskLevel::High,
        prompt_body: "Mariah is about to post a $1,200 hockey stick listing. \
                      The price is 18% below your floor — confirm?"
            .into(),
        approve_url: format!(
            "https://gw.skycot.com/decision?token=appr-claim-6.approve.{}.signed-hmac-aaa",
            expiry.timestamp()
        ),
        reject_url: format!(
            "https://gw.skycot.com/decision?token=appr-claim-6.reject.{}.signed-hmac-bbb",
            expiry.timestamp()
        ),
        expiry,
    }
}

#[test]
fn claim_6_mvp_slack_outbound_payload_is_well_formed() {
    let channel = fixture_channel();
    let notification = fixture_notification();

    let payload = channel
        .message_payload(&notification)
        .expect("message_payload must build the Slack envelope");

    // ── 1. Payload routes to the configured operator's Slack channel.
    assert_eq!(
        payload["channel"].as_str(),
        Some(TEST_SLACK_CHANNEL),
        "claim-6 MVP outbound: Slack payload must route to operator's configured channel; got {}",
        payload["channel"]
    );

    // ── 2. Top-level `text` references the action id (Slack uses this
    //   for notifications when blocks don't render).
    let text = payload["text"]
        .as_str()
        .expect("payload.text must be a string");
    assert!(
        text.contains(&notification.action_id),
        "claim-6 MVP outbound: payload.text must reference action_id; got {text:?}"
    );

    // ── 3. Blocks array exists, non-empty, contains both interactive
    //   buttons (approve + reject) with the right action_ids.
    let blocks = payload["blocks"]
        .as_array()
        .expect("payload.blocks must be an array");
    assert!(
        !blocks.is_empty(),
        "claim-6 MVP outbound: blocks array must not be empty"
    );

    let actions_block = blocks
        .iter()
        .find(|b| b["type"] == "actions")
        .expect("must include an `actions` block with interactive buttons");
    let elements = actions_block["elements"]
        .as_array()
        .expect("actions block must have elements");
    let action_ids: Vec<&str> = elements
        .iter()
        .filter_map(|e| e["action_id"].as_str())
        .collect();
    assert!(
        action_ids.contains(&"approve"),
        "claim-6 MVP outbound: must include an action_id=approve button; got {action_ids:?}"
    );
    assert!(
        action_ids.contains(&"reject") || action_ids.contains(&"decline"),
        "claim-6 MVP outbound: must include an action_id=reject (or decline) button; got {action_ids:?}"
    );

    // ── 4. The signed approval tokens (extracted from the URLs) appear
    //   as Slack button `value` fields. Slack's interactivity callback
    //   carries the `value` back to the gateway, so the gateway can
    //   correlate the operator's click to the right signed token. We
    //   assert the *token suffix* (everything after `?token=`)
    //   appears, since the channel strips the URL and only embeds
    //   the token in the button payload.
    let payload_str = payload.to_string();
    let approve_token = notification
        .approve_url
        .split_once("?token=")
        .expect("approve_url must have ?token= suffix")
        .1;
    let reject_token = notification
        .reject_url
        .split_once("?token=")
        .expect("reject_url must have ?token= suffix")
        .1;
    assert!(
        payload_str.contains(approve_token),
        "claim-6 MVP outbound: signed approve_token must appear in Slack button `value`; got payload {payload_str:?}"
    );
    assert!(
        payload_str.contains(reject_token),
        "claim-6 MVP outbound: signed reject_token must appear in Slack button `value`; got payload {payload_str:?}"
    );

    // ── 5. Risk level + prompt body surface so the operator can decide
    //   without a round-trip.
    assert!(
        payload_str
            .to_lowercase()
            .contains(&notification.risk_level.to_string()),
        "claim-6 MVP outbound: risk level must surface in payload; got body {payload_str:?}"
    );
    assert!(
        payload_str.contains("Mariah is about to post"),
        "claim-6 MVP outbound: prompt_body content must surface in payload"
    );

    // ── 6. Approval id appears in payload so gateway can correlate
    //   the operator's reply back to the right approval row.
    assert!(
        payload_str.contains(&notification.approval_id),
        "claim-6 MVP outbound: approval_id must appear in payload for correlation"
    );
}

#[test]
fn claim_6_mvp_slack_outbound_distinct_action_ids_per_button() {
    // Defensive — make sure approve + reject buttons have DISTINCT
    // action_ids, otherwise Slack's interactivity callback can't
    // distinguish them and the operator's reject becomes an approve.
    let channel = fixture_channel();
    let notification = fixture_notification();
    let payload = channel.message_payload(&notification).expect("payload");

    let blocks = payload["blocks"].as_array().expect("blocks");
    let actions = blocks
        .iter()
        .find(|b| b["type"] == "actions")
        .expect("actions block");
    let action_ids: Vec<&str> = actions["elements"]
        .as_array()
        .expect("elements")
        .iter()
        .filter_map(|e| e["action_id"].as_str())
        .collect();
    let mut deduped = action_ids.clone();
    deduped.sort_unstable();
    deduped.dedup();
    assert_eq!(
        action_ids.len(),
        deduped.len(),
        "claim-6 MVP outbound: every button must have a unique action_id; got {action_ids:?}"
    );
    assert!(
        action_ids.len() >= 2,
        "claim-6 MVP outbound: payload must include at least 2 distinct buttons (approve + reject); got {action_ids:?}"
    );
}
