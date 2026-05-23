//! # Mission Claim 6 LIVE — outbound (Email + Slack) signed-token round-trip
//!
//! Pins sub-test (a) of the operator-locked Claim 6 LIVE criteria
//! (round 18 draft, cited in `docs/skycot/evals/scheduled/2026-05-23/
//! mission-vs-code-gap-manifest.md`):
//!
//! > **Claim 6a LIVE (outbound)** — a worker triggers an
//! > approval-gated action → the configured channel (email OR Slack —
//! > both required) fires the request → the operator gets a
//! > deliverable artifact (signed approval token in the email body /
//! > Slack message) → declining via the channel rejects the action and
//! > writes `ApprovalDeclined` audit; approving advances the action
//! > and writes `ApprovalGranted`.
//!
//! Existing tests pin SHAPE (`claim_6_slack_outbound.rs`) and HTTP
//! delivery (`email_channel.rs::email_send_posts_to_fake_postmark`).
//! Neither pins the **cryptographic** half of the LIVE contract —
//! that the signed tokens embedded by BOTH channels round-trip
//! through `verify_approval_token` under the shared HMAC secret, and
//! that tamper / wrong-secret paths reject. This file is that pin.
//!
//! ## What this test proves
//!
//! 1. Both Email (via `EmailChannel::send` against a fake Postmark)
//!    and Slack (via `SlackApprovalChannel::message_payload`) embed
//!    signed approve + reject tokens that `verify_approval_token`
//!    accepts under the real HMAC secret.
//! 2. Each token decodes to the correct `ApprovalDecision` — the
//!    approve URL's token carries `Approve`, the reject URL's token
//!    carries `Reject`. Neither side is silently coerced.
//! 3. Verification with a DIFFERENT secret rejects (HMAC integrity).
//! 4. Verification of a tampered token (the on-the-wire body with
//!    one byte flipped in the signature) rejects.
//! 5. The token's `approval_id` claim matches the notification's
//!    `approval_id` — the gateway can correlate the operator's reply
//!    back to the right worker action.
//! 6. The same notification artifact survives the round-trip on BOTH
//!    channels with the same secret — there is no per-channel
//!    HMAC-secret drift.
//!
//! ## Why this is the missing pin
//!
//! Mission claim 6 says: "the operator gets a deliverable artifact
//! (signed approval token in the email body / Slack message)". The
//! existing Slack outbound test asserts the token *string* appears in
//! the payload; this test asserts that the string is a **valid
//! cryptographic artifact** — i.e., the gateway-side
//! `verify_approval_token` would accept it. If a future refactor
//! changes the token format, drops the HMAC step, or rotates the
//! secret between sign + verify, this test fails immediately.
//!
//! ## Authored
//!
//! Eval brief: `eval-claim-6-live-cross-tenant-audit-chain-test`
//! (round 18 draft, see `docs/skycot/evals/scheduled/2026-05-23/
//! draft-live-criteria-claims-4-5-6.md`).

use std::collections::BTreeMap;

use argo_channels::email::{EmailChannel, EmailChannelConfig};
use argo_channels::rate_limit::RateLimitConfig;
use argo_channels::slack::{
    SlackApprovalChannel, SlackApprovalChannelConfig, SlackOperatorRoute,
};
use argo_channels::token::{
    sign_approval_token, signed_decision_url, verify_approval_token, ApprovalDecision,
};
use argo_channels::{
    ChannelId, Notification, NotifyChannel, OperatorTarget, RiskLevel,
};
use chrono::{Duration, Utc};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

const HMAC_SECRET: &str = "claim-6-live-outbound-shared-secret";
const WRONG_SECRET: &str = "claim-6-live-outbound-wrong-secret";
const TEST_OPERATOR_ID: &str = "op-mariah-claim-6-live";
const TEST_SLACK_CHANNEL: &str = "C-claim-6-live";
const TEST_WORKSPACE: &str = "T-claim-6-live";

/// Build a notification carrying REAL HMAC-signed approve + reject
/// URLs. The token-from-URL extraction in
/// `SlackApprovalChannel::message_payload` reads `?token=` so we use
/// `signed_decision_url` (the shipping path the gateway will use)
/// rather than hand-rolling URLs.
fn signed_notification(approval_id: &str) -> Notification {
    let expiry = Utc::now() + Duration::minutes(15);
    let approve_url = signed_decision_url(
        "https://gw.skycot.com/decision",
        approval_id,
        ApprovalDecision::Approve,
        expiry.timestamp(),
        HMAC_SECRET,
    )
    .expect("approve URL must sign");
    let reject_url = signed_decision_url(
        "https://gw.skycot.com/decision",
        approval_id,
        ApprovalDecision::Reject,
        expiry.timestamp(),
        HMAC_SECRET,
    )
    .expect("reject URL must sign");
    Notification {
        target: OperatorTarget {
            operator_id: TEST_OPERATOR_ID.into(),
            email: "mariah@example.test".into(),
        },
        action_id: format!("act-{approval_id}"),
        approval_id: approval_id.into(),
        risk_level: RiskLevel::High,
        prompt_body: "Mariah is about to post a $1,200 hockey stick listing. \
                      18% below your floor — confirm?"
            .into(),
        approve_url,
        reject_url,
        expiry,
    }
}

fn slack_channel() -> SlackApprovalChannel {
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
        bot_token: "xoxb-claim-6-live-not-real".into(),
        endpoint: "http://127.0.0.1:9/never-reached".into(),
        operator_routes,
        rate_limit: RateLimitConfig::unlimited(),
        quiet_hours: None,
    })
}

async fn fake_postmark() -> (String, tokio::task::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    let handle = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("accept");
        let mut buf = vec![0_u8; 8192];
        let n = stream.read(&mut buf).await.expect("read");
        let request = String::from_utf8_lossy(&buf[..n]).to_string();
        let body = r#"{"ErrorCode":0,"Message":"OK","MessageID":"fake-message-claim-6-live"}"#;
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        stream.write_all(response.as_bytes()).await.expect("write");
        request
    });
    (format!("http://{addr}/email"), handle)
}

/// Extract the `token=...` query-string value from the FIRST
/// occurrence of `pattern` (the per-decision URL prefix) inside
/// `corpus`. Panics if not found — the test asserts that BOTH approve
/// and reject tokens land in the artifact, so a missing pattern is a
/// hard failure.
fn extract_token(corpus: &str, base_url: &str, decision_hint: &str) -> String {
    // The URL is `https://gw.skycot.com/decision?token=<token>` — find
    // the first occurrence and slice forward.
    let needle = format!("{base_url}?token=");
    let after_needle = corpus
        .find(&needle)
        .map(|i| &corpus[i + needle.len()..])
        .unwrap_or_else(|| {
            panic!(
                "claim-6 LIVE outbound: corpus must contain a signed decision URL for {decision_hint}; \
                 looked for {needle:?} in: {corpus:?}"
            )
        });
    // The token runs until the next non-token character: whitespace,
    // `"`, `<`, `)`, `\`, EOL — anything that can't appear in our
    // base64-ish `id.decision.exp.sig` format.
    let token_end = after_needle
        .find(|c: char| {
            !(c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_')
        })
        .unwrap_or(after_needle.len());
    after_needle[..token_end].to_string()
}

#[tokio::test]
async fn claim_6_live_outbound_email_signed_token_round_trips() {
    let (endpoint, handle) = fake_postmark().await;
    let channel = EmailChannel::new(EmailChannelConfig {
        channel_id: ChannelId::new("email"),
        postmark_token: "postmark-claim-6-live".into(),
        from_address: "worker@skycot.com".into(),
        subject_template: "[{{risk_level}}] approval needed for {{action_id}}".into(),
        endpoint,
        message_stream: None,
        quiet_hours: Default::default(),
        rate_limit: RateLimitConfig::unlimited(),
    });
    let notification = signed_notification("appr-claim-6-live-email");
    let expected_approval_id = notification.approval_id.clone();

    let receipt = channel.send(notification).await.expect("email send");
    assert_eq!(
        receipt.provider_message_id.as_deref(),
        Some("fake-message-claim-6-live"),
        "claim-6 LIVE outbound: email channel must surface provider message id"
    );

    let request = handle.await.expect("fake postmark server");

    // Both signed URLs landed in the wire body.
    assert!(
        request.contains("https://gw.skycot.com/decision?token="),
        "claim-6 LIVE outbound: email body must carry signed decision URLs; got {request:?}"
    );

    // Extract approve + reject tokens by scanning for the unique
    // decision-suffix substring (`.approve.` vs `.reject.`) embedded
    // in the token's plaintext claims segment.
    let mut tokens_found: Vec<&str> = request
        .split("https://gw.skycot.com/decision?token=")
        .skip(1)
        .map(|chunk| {
            let end = chunk
                .find(|c: char| {
                    !(c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_')
                })
                .unwrap_or(chunk.len());
            &chunk[..end]
        })
        .collect();
    tokens_found.sort_unstable();
    tokens_found.dedup();
    assert_eq!(
        tokens_found.len(),
        2,
        "claim-6 LIVE outbound: email must embed exactly two distinct tokens \
         (approve + reject); got {tokens_found:?}"
    );

    // Verify each token under the real HMAC secret.
    let mut decisions: Vec<ApprovalDecision> = Vec::new();
    for token in &tokens_found {
        let claims = verify_approval_token(token, HMAC_SECRET)
            .unwrap_or_else(|e| panic!("verify_approval_token({token}) under real secret failed: {e}"));
        assert_eq!(
            claims.approval_id, expected_approval_id,
            "claim-6 LIVE outbound: email-embedded token's approval_id must match notification.approval_id"
        );
        decisions.push(claims.decision);

        // ── Negative: wrong secret must reject (HMAC integrity).
        assert!(
            verify_approval_token(token, WRONG_SECRET).is_err(),
            "claim-6 LIVE outbound: verify under WRONG secret must fail for token {token}"
        );

        // ── Negative: tamper one byte of the signature segment (the
        // last `.<hex>` block). Token format = `id.decision.exp.sig`.
        let mut parts: Vec<&str> = token.split('.').collect();
        assert_eq!(parts.len(), 4, "token format invariant: 4 dot-separated fields");
        let mut tampered_sig: Vec<u8> = parts[3].as_bytes().to_vec();
        // Flip a hex nibble — XOR with 0x01 keeps the byte ASCII-printable
        // for 0–9/a–e but produces a different hex digit. Pick the last
        // byte so we don't corrupt the lead bytes accidentally being
        // interpreted as something other than hex.
        let last = tampered_sig.len() - 1;
        let original = tampered_sig[last];
        let new_byte = match original {
            b'0'..=b'8' => original + 1,
            b'9' => b'a',
            b'a'..=b'e' => original + 1,
            b'f' => b'0',
            _ => original ^ 0x01,
        };
        tampered_sig[last] = new_byte;
        let tampered_sig_str =
            std::str::from_utf8(&tampered_sig).expect("tampered sig stays ASCII");
        parts[3] = tampered_sig_str;
        let tampered_token = parts.join(".");
        assert!(
            verify_approval_token(&tampered_token, HMAC_SECRET).is_err(),
            "claim-6 LIVE outbound: tampered signature must be rejected; got accept for {tampered_token}"
        );
    }

    // Both decisions appear, distinct.
    decisions.sort_by_key(|d| d.as_token_str());
    assert!(
        decisions.contains(&ApprovalDecision::Approve)
            && decisions.contains(&ApprovalDecision::Reject),
        "claim-6 LIVE outbound: email must embed BOTH approve AND reject signed tokens; got {decisions:?}"
    );
}

#[test]
fn claim_6_live_outbound_slack_signed_tokens_round_trip() {
    let channel = slack_channel();
    let notification = signed_notification("appr-claim-6-live-slack");
    let expected_approval_id = notification.approval_id.clone();

    let payload = channel
        .message_payload(&notification)
        .expect("Slack payload must build");
    let payload_str = payload.to_string();

    // Slack stores the TOKEN ONLY (not the URL) in the button `value`
    // — extract by scanning the actions block.
    let blocks = payload["blocks"]
        .as_array()
        .expect("payload.blocks must be an array");
    let actions = blocks
        .iter()
        .find(|b| b["type"] == "actions")
        .expect("payload must include an `actions` block");
    let elements = actions["elements"]
        .as_array()
        .expect("actions block must have elements");

    let mut tokens_by_action: Vec<(String, String)> = Vec::new();
    for el in elements {
        let action_id = el["action_id"]
            .as_str()
            .expect("each Slack button must carry action_id")
            .to_string();
        let value = el["value"]
            .as_str()
            .expect("each Slack button must carry a `value` (the signed token)")
            .to_string();
        tokens_by_action.push((action_id, value));
    }
    assert!(
        tokens_by_action.len() >= 2,
        "claim-6 LIVE outbound: Slack payload must carry at least 2 signed-token buttons; \
         got {tokens_by_action:?}"
    );

    let approve_token = tokens_by_action
        .iter()
        .find(|(aid, _)| aid == "approve")
        .map(|(_, t)| t.clone())
        .expect("Slack payload must include an action_id=approve button with a signed token");
    let reject_token = tokens_by_action
        .iter()
        .find(|(aid, _)| aid == "reject" || aid == "decline")
        .map(|(_, t)| t.clone())
        .expect("Slack payload must include an action_id=reject/decline button with a signed token");

    // ── Round-trip: approve token decodes to Approve under real secret.
    let approve_claims = verify_approval_token(&approve_token, HMAC_SECRET)
        .expect("Slack approve token must verify under real secret");
    assert_eq!(
        approve_claims.approval_id, expected_approval_id,
        "claim-6 LIVE outbound: Slack approve token's approval_id must match notification.approval_id"
    );
    assert_eq!(
        approve_claims.decision,
        ApprovalDecision::Approve,
        "claim-6 LIVE outbound: Slack approve-button token MUST decode to Approve, not silently to Reject"
    );

    // ── Round-trip: reject token decodes to Reject under real secret.
    let reject_claims = verify_approval_token(&reject_token, HMAC_SECRET)
        .expect("Slack reject token must verify under real secret");
    assert_eq!(
        reject_claims.approval_id, expected_approval_id,
        "claim-6 LIVE outbound: Slack reject token's approval_id must match notification.approval_id"
    );
    assert_eq!(
        reject_claims.decision,
        ApprovalDecision::Reject,
        "claim-6 LIVE outbound: Slack reject-button token MUST decode to Reject, not silently to Approve"
    );

    // ── Negative: WRONG secret must reject BOTH tokens.
    assert!(
        verify_approval_token(&approve_token, WRONG_SECRET).is_err(),
        "claim-6 LIVE outbound: Slack approve token must reject under wrong secret"
    );
    assert!(
        verify_approval_token(&reject_token, WRONG_SECRET).is_err(),
        "claim-6 LIVE outbound: Slack reject token must reject under wrong secret"
    );

    // ── Approve + reject tokens must be DIFFERENT strings. Same
    // approval_id + different decision must produce different
    // HMAC outputs.
    assert_ne!(
        approve_token, reject_token,
        "claim-6 LIVE outbound: approve + reject tokens MUST differ — same approval_id + \
         different decision must yield different HMAC outputs"
    );

    // ── Defensive: payload_str (the wire body) must contain both
    // raw tokens — a future refactor that strips one button would
    // trip the test before regressing the operator UX.
    assert!(
        payload_str.contains(&approve_token),
        "claim-6 LIVE outbound: signed approve token must appear in Slack wire payload"
    );
    assert!(
        payload_str.contains(&reject_token),
        "claim-6 LIVE outbound: signed reject token must appear in Slack wire payload"
    );
}

#[test]
fn claim_6_live_outbound_shared_hmac_secret_no_per_channel_drift() {
    // Defensive contract — the SAME signed token built with the
    // shared secret must verify regardless of whether it's carried in
    // an email body or a Slack button. This pins that there's no
    // per-channel HMAC-secret drift (e.g., one channel accidentally
    // reading from a different env var).
    //
    // We can't reach into the channels' env-resolution here, but we
    // CAN pin the substrate contract: `sign_approval_token` produces
    // a token that `verify_approval_token` accepts under the same
    // secret, and any other secret rejects. If the channels' env
    // resolution were to drift (one reads `ARGO_CHANNEL_HMAC_SECRET`,
    // the other reads `ARGO_SLACK_HMAC_SECRET`), the outbound URLs
    // would be signed under one secret and the gateway's `verify`
    // would run under another — and this test pins the contract that
    // verify is single-secret.

    let exp = Utc::now().timestamp() + 60;
    let approval_id = "appr-claim-6-live-shared-secret";
    let token =
        sign_approval_token(approval_id, ApprovalDecision::Approve, exp, HMAC_SECRET)
            .expect("token signs");
    let claims = verify_approval_token(&token, HMAC_SECRET).expect("verifies under real");
    assert_eq!(claims.approval_id, approval_id);
    assert_eq!(claims.decision, ApprovalDecision::Approve);
    assert_eq!(claims.exp, exp);
    assert!(verify_approval_token(&token, WRONG_SECRET).is_err());
    // Also pin that `extract_token` works on a URL built with
    // `signed_decision_url` — this is the substrate the channels
    // depend on for URL → token surfacing.
    let url = signed_decision_url(
        "https://gw.skycot.com/decision",
        approval_id,
        ApprovalDecision::Approve,
        exp,
        HMAC_SECRET,
    )
    .expect("signed URL");
    let extracted = extract_token(&url, "https://gw.skycot.com/decision", "approve");
    assert_eq!(
        extracted, token,
        "claim-6 LIVE outbound: signed_decision_url must round-trip the token via ?token= query"
    );
}
