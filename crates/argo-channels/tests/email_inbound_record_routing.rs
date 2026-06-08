//! Inbound Postmark webhook record-type routing seam.
//!
//! The gateway handler
//! (`crates/argo-cli/src/handlers/gateway/routes_channels_email.rs`)
//! is a binary-crate internal (`pub(super)`), so — exactly as
//! `imessage_webhook_smoke.rs` does — this integration test exercises
//! the same public library seam the handler delegates to:
//!
//!  * `verify_webhook_signature` — the HMAC envelope gate (a bad
//!    signature is the route's 401).
//!  * `classify_postmark_record` — the `RecordType` branch (event
//!    records are acknowledged; inbound replies settle).
//!  * `inbound_event_from_payload` + `verify_approval_token` — the
//!    reply path that produces the same `InboundEvent` the gateway
//!    feeds into `settle`.
//!
//! These three together are the truth-bounded proof that a real signed
//! Bounce payload routes to acknowledgement (not a spurious
//! `missing approval token` 400) while a real signed Inbound reply
//! still settles.

use argo_channels::email::{classify_postmark_record, inbound_event_from_payload, PostmarkRecord};
use argo_channels::token::{sign_approval_token, verify_approval_token, verify_webhook_signature};
use argo_channels::{ChannelId, token::ApprovalDecision};
use chrono::Utc;

const SECRET: &str = "argo-channel-hmac-record-routing";

fn sign_body(body: &[u8], secret: &str) -> String {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("hmac");
    mac.update(body);
    hex::encode(mac.finalize().into_bytes())
}

#[test]
fn bad_signature_is_rejected_before_any_record_branch() {
    let body = serde_json::json!({"RecordType": "Bounce", "Email": "x@y.z"}).to_string();
    let wrong_sig = sign_body(body.as_bytes(), "the-wrong-secret");
    assert!(
        verify_webhook_signature(body.as_bytes(), &wrong_sig, SECRET).is_err(),
        "a mismatched X-Postmark-Signature must fail (the route's 401)",
    );
}

#[test]
fn signed_bounce_routes_to_acknowledgement_not_token_parse() {
    let body = serde_json::json!({
        "RecordType": "Bounce",
        "Type": "HardBounce",
        "Email": "bouncer@example.com",
        "MessageID": "msg-bounce-1",
    });
    let raw = body.to_string();
    let sig = sign_body(raw.as_bytes(), SECRET);
    // Envelope verifies …
    verify_webhook_signature(raw.as_bytes(), &sig, SECRET).expect("valid signature");
    // … and the record-type branch sends it to acknowledgement, NOT
    // the approval-token path (which previously 400'd on event records).
    assert_eq!(
        classify_postmark_record(&body),
        PostmarkRecord::Event("Bounce".into()),
    );
}

#[test]
fn signed_inbound_reply_round_trips_to_settle_inputs() {
    let exp = Utc::now().timestamp() + 600;
    let token =
        sign_approval_token("act-inbound-1", ApprovalDecision::Approve, exp, SECRET).expect("token");
    let body = serde_json::json!({
        "RecordType": "Inbound",
        "From": "ceo@acme.io",
        "StrippedTextReply": format!("approving this\napproval_token={token}"),
    });
    let raw = body.to_string();
    let sig = sign_body(raw.as_bytes(), SECRET);

    verify_webhook_signature(raw.as_bytes(), &sig, SECRET).expect("valid signature");
    assert_eq!(classify_postmark_record(&body), PostmarkRecord::InboundReply);

    let claims = verify_approval_token(&token, SECRET).expect("approval claims");
    assert_eq!(claims.approval_id, "act-inbound-1");
    assert_eq!(claims.decision, ApprovalDecision::Approve);

    // The same InboundEvent the gateway hands to `settle`.
    let event = inbound_event_from_payload(
        ChannelId::new("email"),
        body,
        claims.approval_id.clone(),
        claims.decision,
    );
    assert_eq!(event.actor, "ceo@acme.io");
    assert_eq!(event.approval_id, "act-inbound-1");
    assert_eq!(event.decision, ApprovalDecision::Approve);
    assert!(
        event.note.as_deref().unwrap_or_default().starts_with("approving this"),
        "note carries the operator reply body",
    );
}
