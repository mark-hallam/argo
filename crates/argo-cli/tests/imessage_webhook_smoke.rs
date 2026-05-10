//! End-to-end smoke for the inbound iMessage approval webhook.
//!
//! The gateway dispatcher arm in
//! `crates/argo-cli/src/handlers/gateway/routes_channels_imessage.rs`
//! is `pub(super)`, so this integration test exercises the same
//! public seam an external integrator would compose:
//! [`argo_channels::imessage::IMessageChannel::parse_webhook`] —
//! which performs the identical HMAC-SHA256 hex verification + reply
//! token decode the gateway delegates to.
//!
//! Failure modes (bad signature, missing token) are covered alongside
//! the happy path so a regression in either the envelope or the token
//! format breaks this test.

use argo_channels::imessage::{IMessageChannel, IMessageChannelConfig};
use argo_channels::token::{sign_approval_token, ApprovalDecision};
use chrono::Utc;
use sha2::{Digest, Sha256};

const WEBHOOK_SECRET: &str = "webhook-secret";
const APPROVAL_SECRET: &str = "secret";
const ACTION_ID: &str = "act-imessage-smoke";

fn hmac_sha256_hex(body: &[u8], secret: &[u8]) -> String {
    const BLOCK: usize = 64;
    let mut key = [0_u8; BLOCK];
    if secret.len() > BLOCK {
        let digest = Sha256::digest(secret);
        key[..digest.len()].copy_from_slice(&digest);
    } else {
        key[..secret.len()].copy_from_slice(secret);
    }
    let mut ipad = [0x36_u8; BLOCK];
    let mut opad = [0x5c_u8; BLOCK];
    for i in 0..BLOCK {
        ipad[i] ^= key[i];
        opad[i] ^= key[i];
    }
    let mut inner = Sha256::new();
    inner.update(ipad);
    inner.update(body);
    let inner = inner.finalize();
    let mut outer = Sha256::new();
    outer.update(opad);
    outer.update(inner);
    hex::encode(outer.finalize())
}

fn signed_webhook_body(token: &str) -> (Vec<u8>, String) {
    let body = serde_json::json!({
        "message": {
            "text": format!("YES approval_token={token}"),
            "handle": "+15551234567",
            "chatGuid": "iMessage;+;+15551234567"
        }
    })
    .to_string()
    .into_bytes();
    let signature = hmac_sha256_hex(&body, WEBHOOK_SECRET.as_bytes());
    (body, signature)
}

fn channel() -> IMessageChannel {
    IMessageChannel::new(IMessageChannelConfig::local("http://127.0.0.1:1")).expect("channel")
}

#[test]
fn signed_payload_round_trips_to_inbound_event() {
    let exp = Utc::now().timestamp() + 600;
    let token = sign_approval_token(ACTION_ID, ApprovalDecision::Approve, exp, APPROVAL_SECRET)
        .expect("sign approval token");
    let (body, sig) = signed_webhook_body(&token);
    let event = channel()
        .parse_webhook(&body, Some(&sig))
        .expect("valid signed payload must verify");
    assert_eq!(event.approval_id, ACTION_ID);
    assert_eq!(event.decision, ApprovalDecision::Approve);
    assert_eq!(event.actor, "+15551234567");
}

#[test]
fn tampered_body_rejects() {
    let exp = Utc::now().timestamp() + 600;
    let token = sign_approval_token(ACTION_ID, ApprovalDecision::Approve, exp, APPROVAL_SECRET)
        .expect("sign approval token");
    let (body, sig) = signed_webhook_body(&token);
    let mut tampered = body.clone();
    tampered.extend_from_slice(b" "); // single-byte mutation breaks HMAC
    let err = channel()
        .parse_webhook(&tampered, Some(&sig))
        .expect_err("tampered body must reject");
    assert!(err.to_string().to_lowercase().contains("auth"), "got: {err}");
}
