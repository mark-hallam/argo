//! Signature-verification and event-dispatch tests for the
//! `argo-billing` webhook layer.
//!
//! Stripe documents the wire format at
//! <https://stripe.com/docs/webhooks/signatures>; these tests build
//! the expected signature inline rather than mocking Stripe so we
//! exercise the same HMAC + tolerance window the gateway sees in
//! production.

use super::*;
use argo_identity::tier::Tier;
use chrono::TimeZone;
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256Test = Hmac<Sha256>;

const SECRET: &str = "whsec_test_secret_x";

fn sign(body: &str, timestamp: i64, secret: &str) -> String {
    let payload = format!("{timestamp}.{body}");
    let mut mac =
        HmacSha256Test::new_from_slice(secret.as_bytes()).expect("hmac init");
    mac.update(payload.as_bytes());
    let signature = hex::encode(mac.finalize().into_bytes());
    format!("t={timestamp},v1={signature}")
}

fn now_at(t: i64) -> chrono::DateTime<Utc> {
    Utc.timestamp_opt(t, 0).unwrap()
}

#[test]
fn signature_round_trip_passes() {
    let body = r#"{"id":"evt_1","type":"customer.subscription.created"}"#;
    let header = sign(body, 1_700_000_000, SECRET);
    verify_signature(&header, body.as_bytes(), SECRET, now_at(1_700_000_010))
        .expect("signature should verify");
}

#[test]
fn signature_with_wrong_secret_rejected() {
    let body = "{}";
    let header = sign(body, 1_700_000_000, "other_secret");
    let err = verify_signature(&header, body.as_bytes(), SECRET, now_at(1_700_000_000))
        .expect_err("must reject");
    assert!(matches!(err, WebhookError::BadSignature));
}

#[test]
fn signature_with_tampered_body_rejected() {
    let body = "{}";
    let header = sign(body, 1_700_000_000, SECRET);
    let err = verify_signature(&header, b"{\"x\":1}", SECRET, now_at(1_700_000_000))
        .expect_err("must reject");
    assert!(matches!(err, WebhookError::BadSignature));
}

#[test]
fn signature_stale_timestamp_rejected() {
    let body = "{}";
    let header = sign(body, 1_700_000_000, SECRET);
    let err = verify_signature(
        &header,
        body.as_bytes(),
        SECRET,
        now_at(1_700_000_000 + SIGNATURE_TOLERANCE + 1),
    )
    .expect_err("must reject");
    assert!(matches!(err, WebhookError::StaleTimestamp(_)));
}

#[test]
fn signature_malformed_header_rejected() {
    assert!(matches!(
        verify_signature("not-a-header", b"{}", SECRET, now_at(0)),
        Err(WebhookError::MalformedSignature)
    ));
    assert!(matches!(
        verify_signature("t=abc,v1=xx", b"{}", SECRET, now_at(0)),
        Err(WebhookError::MalformedSignature)
    ));
    // No `v1` arm at all.
    assert!(matches!(
        verify_signature("t=1700000000,v0=xx", b"{}", SECRET, now_at(1_700_000_000)),
        Err(WebhookError::MalformedSignature)
    ));
}

#[test]
fn signature_accepts_any_v1_arm() {
    let body = "{}";
    let timestamp = 1_700_000_000_i64;
    let payload = format!("{timestamp}.{body}");
    let mut mac =
        HmacSha256Test::new_from_slice(SECRET.as_bytes()).expect("hmac init");
    mac.update(payload.as_bytes());
    let real = hex::encode(mac.finalize().into_bytes());
    let header = format!("t={timestamp},v1=ffff,v1={real}");
    verify_signature(&header, body.as_bytes(), SECRET, now_at(timestamp))
        .expect("second v1 arm should match");
}

#[test]
fn classify_subscription_created_maps_to_pro_by_default() {
    let tenant = TenantId::new();
    let body = serde_json::json!({
        "id": "evt_a",
        "type": "customer.subscription.created",
        "data": {
            "object": {
                "id": "sub_a",
                "customer": "cus_a",
                "status": "active",
                "metadata": { "tenant_id": tenant.to_string() }
            }
        }
    });
    let event = decode_event(body.to_string().as_bytes()).expect("decode");
    let outcome = classify_event(&event);
    let WebhookOutcome::Subscription(sub) = outcome else {
        panic!("expected subscription outcome, got {outcome:?}");
    };
    assert_eq!(sub.tenant_id, tenant);
    assert_eq!(sub.customer_id, "cus_a");
    assert_eq!(sub.subscription_id.as_deref(), Some("sub_a"));
    assert_eq!(sub.tier, Tier::Pro);
    assert!(!sub.past_due_warning);
}

#[test]
fn classify_subscription_created_honours_metadata_tier() {
    let tenant = TenantId::new();
    let body = serde_json::json!({
        "id": "evt_b",
        "type": "customer.subscription.created",
        "data": {
            "object": {
                "id": "sub_b",
                "customer": "cus_b",
                "status": "active",
                "metadata": {
                    "tenant_id": tenant.to_string(),
                    "tier": "enterprise"
                }
            }
        }
    });
    let event = decode_event(body.to_string().as_bytes()).expect("decode");
    let WebhookOutcome::Subscription(sub) = classify_event(&event) else {
        panic!("expected subscription outcome");
    };
    assert_eq!(sub.tier, Tier::Enterprise);
}

#[test]
fn classify_subscription_updated_past_due_flags_downgrade() {
    let tenant = TenantId::new();
    let body = serde_json::json!({
        "id": "evt_c",
        "type": "customer.subscription.updated",
        "data": {
            "object": {
                "id": "sub_c",
                "customer": "cus_c",
                "status": "past_due",
                "metadata": { "tenant_id": tenant.to_string(), "tier": "pro" }
            }
        }
    });
    let event = decode_event(body.to_string().as_bytes()).expect("decode");
    let WebhookOutcome::Subscription(sub) = classify_event(&event) else {
        panic!("expected subscription outcome");
    };
    assert_eq!(sub.tier, Tier::Free);
    assert!(sub.past_due_warning);
}

#[test]
fn classify_subscription_deleted_downgrades_to_free() {
    let tenant = TenantId::new();
    let body = serde_json::json!({
        "id": "evt_d",
        "type": "customer.subscription.deleted",
        "data": {
            "object": {
                "id": "sub_d",
                "customer": "cus_d",
                "status": "canceled",
                "metadata": { "tenant_id": tenant.to_string() }
            }
        }
    });
    let event = decode_event(body.to_string().as_bytes()).expect("decode");
    let WebhookOutcome::Subscription(sub) = classify_event(&event) else {
        panic!("expected subscription outcome");
    };
    assert_eq!(sub.tier, Tier::Free);
    assert_eq!(sub.subscription_id, None);
}

#[test]
fn classify_unrelated_event_ignored() {
    let body = serde_json::json!({
        "id": "evt_e",
        "type": "invoice.payment_succeeded",
        "data": { "object": { "id": "in_e" } }
    });
    let event = decode_event(body.to_string().as_bytes()).expect("decode");
    assert!(matches!(classify_event(&event), WebhookOutcome::Ignored));
}

#[test]
fn classify_subscription_missing_tenant_ignored() {
    let body = serde_json::json!({
        "id": "evt_f",
        "type": "customer.subscription.created",
        "data": {
            "object": { "id": "sub_f", "customer": "cus_f", "status": "active" }
        }
    });
    let event = decode_event(body.to_string().as_bytes()).expect("decode");
    assert!(matches!(classify_event(&event), WebhookOutcome::Ignored));
}

#[test]
fn classify_picks_up_tier_from_price_metadata() {
    let tenant = TenantId::new();
    let body = serde_json::json!({
        "id": "evt_g",
        "type": "customer.subscription.created",
        "data": {
            "object": {
                "id": "sub_g",
                "customer": "cus_g",
                "status": "active",
                "metadata": { "tenant_id": tenant.to_string() },
                "items": {
                    "data": [{
                        "price": { "metadata": { "tier": "enterprise" } }
                    }]
                }
            }
        }
    });
    let event = decode_event(body.to_string().as_bytes()).expect("decode");
    let WebhookOutcome::Subscription(sub) = classify_event(&event) else {
        panic!("expected subscription outcome");
    };
    assert_eq!(sub.tier, Tier::Enterprise);
}
