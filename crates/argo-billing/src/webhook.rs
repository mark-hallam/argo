//! Inbound Stripe webhook handling.
//!
//! Three concerns:
//!
//! 1. **Signature verification** — Stripe sends a `Stripe-Signature`
//!    header of the form `t=<timestamp>,v1=<sig>[,v0=…][,v1=<alt>…]`.
//!    The signature is `HMAC-SHA256(secret, "{t}.{body}")`. We accept
//!    any `v1` arm that matches and reject timestamps more than
//!    [`SIGNATURE_TOLERANCE`] outside the supplied clock — matching
//!    the 5-minute window Stripe documents.
//! 2. **Idempotency** — Stripe replays delivery on transient failure.
//!    Receivers MUST dedup by `event.id`. We check
//!    [`crate::tier_store::has_processed_event`] before applying side
//!    effects and write back on success.
//! 3. **Tier dispatch** — `customer.subscription.{created,updated,
//!    deleted}` events map to a [`SubscriptionEvent`] the gateway
//!    persists onto `workspace.tier` via
//!    [`crate::tier_store::apply_subscription_event`].
//!
//! Tier resolution favours explicit price-id metadata first, then
//! falls back to the `metadata.tier` slug Stripe lets operators set on
//! the price (and the seed script in `docs/skycot/pricing/stripe-seed.md`
//! prescribes). Unknown prices → `Tier::Pro` (the v1 default paid tier)
//! so an operator can't accidentally promote a customer to `Enterprise`
//! by mistyping a price id.

use argo_identity::tier::Tier;
use argo_types::TenantId;
use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::str::FromStr;
use thiserror::Error;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

/// Maximum age of a Stripe-signed event timestamp before we reject it.
/// Matches the 5-minute tolerance Stripe documents for `v1` signatures.
pub const SIGNATURE_TOLERANCE: i64 = 5 * 60;

/// Errors the webhook layer surfaces back to the gateway.
#[derive(Debug, Error)]
pub enum WebhookError {
    /// The `Stripe-Signature` header was absent or malformed.
    #[error("missing or malformed Stripe-Signature header")]
    MalformedSignature,
    /// No `v1` signature in the header matched the computed HMAC.
    #[error("signature verification failed")]
    BadSignature,
    /// The signed timestamp drifted more than [`SIGNATURE_TOLERANCE`].
    #[error("timestamp {0} outside {SIGNATURE_TOLERANCE}s tolerance window")]
    StaleTimestamp(i64),
    /// JSON body could not be parsed as a Stripe event envelope.
    #[error("event decode: {0}")]
    EventDecode(String),
}

/// Outcome of [`process_event`] — what the gateway should do next.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WebhookOutcome {
    /// Event already processed; no side effects required.
    Duplicate,
    /// Event was relevant — gateway should persist this tier change.
    Subscription(SubscriptionEvent),
    /// Event was structurally valid but a no-op for our tier flow.
    Ignored,
}

/// Decoded subscription event the gateway forwards to
/// [`crate::tier_store::apply_subscription_event`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubscriptionEvent {
    /// `evt_…` id — dedup key written to `stripe_event`.
    pub event_id: String,
    /// Tenant the subscription belongs to.
    pub tenant_id: TenantId,
    /// Stripe customer id (`cus_…`).
    pub customer_id: String,
    /// Stripe subscription id (`sub_…`), `None` for `deleted` events.
    pub subscription_id: Option<String>,
    /// Target tier after this event applies.
    pub tier: Tier,
    /// Whether the gateway should also surface a past-due warning email.
    pub past_due_warning: bool,
}

/// Verify the `Stripe-Signature` header against `body` using
/// `webhook_secret`. `header_value` is the raw header text;
/// `now` is the receiver clock (for testability).
pub fn verify_signature(
    header_value: &str,
    body: &[u8],
    webhook_secret: &str,
    now: DateTime<Utc>,
) -> Result<(), WebhookError> {
    let (timestamp, signatures) = parse_signature_header(header_value)?;
    let drift = (now.timestamp() - timestamp).abs();
    if drift > SIGNATURE_TOLERANCE {
        return Err(WebhookError::StaleTimestamp(drift));
    }
    let mut payload = Vec::with_capacity(body.len() + 16);
    payload.extend_from_slice(timestamp.to_string().as_bytes());
    payload.push(b'.');
    payload.extend_from_slice(body);
    let expected = hmac_hex(&payload, webhook_secret.as_bytes())?;
    let expected_bytes = expected.as_bytes();
    for candidate in signatures {
        let candidate_bytes = candidate.as_bytes();
        if constant_time_eq(candidate_bytes, expected_bytes) {
            return Ok(());
        }
    }
    Err(WebhookError::BadSignature)
}

fn parse_signature_header(value: &str) -> Result<(i64, Vec<String>), WebhookError> {
    let mut timestamp: Option<i64> = None;
    let mut signatures: Vec<String> = Vec::new();
    for part in value.split(',') {
        let (k, v) = part
            .split_once('=')
            .ok_or(WebhookError::MalformedSignature)?;
        match k.trim() {
            "t" => {
                timestamp = Some(v.trim().parse::<i64>().map_err(|_| WebhookError::MalformedSignature)?);
            }
            "v1" => signatures.push(v.trim().to_string()),
            _ => {}
        }
    }
    let timestamp = timestamp.ok_or(WebhookError::MalformedSignature)?;
    if signatures.is_empty() {
        return Err(WebhookError::MalformedSignature);
    }
    Ok((timestamp, signatures))
}

fn hmac_hex(payload: &[u8], secret: &[u8]) -> Result<String, WebhookError> {
    let mut mac = HmacSha256::new_from_slice(secret)
        .map_err(|_| WebhookError::MalformedSignature)?;
    mac.update(payload);
    Ok(hex::encode(mac.finalize().into_bytes()))
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0_u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// Decode the JSON body Stripe POSTs into a typed event the gateway
/// can act on. The gateway calls [`verify_signature`] FIRST.
pub fn decode_event(body: &[u8]) -> Result<StripeEventEnvelope, WebhookError> {
    serde_json::from_slice::<StripeEventEnvelope>(body)
        .map_err(|e| WebhookError::EventDecode(e.to_string()))
}

/// Map a decoded [`StripeEventEnvelope`] to a [`WebhookOutcome`]. The
/// gateway then writes the dedup row + (for `Subscription`) the tier
/// row via [`crate::tier_store`].
pub fn classify_event(event: &StripeEventEnvelope) -> WebhookOutcome {
    let data = &event.data.object;
    match event.event_type.as_str() {
        "customer.subscription.created" | "customer.subscription.updated" => {
            let Some(tenant_id) = parse_tenant(data) else {
                tracing::warn!(target: "billing::webhook", event_id = %event.id, "missing tenant_id metadata");
                return WebhookOutcome::Ignored;
            };
            let status = data
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let subscription_id = data.get("id").and_then(|v| v.as_str()).map(str::to_string);
            let customer_id = data
                .get("customer")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let (tier, past_due) = tier_for_status(data, status);
            WebhookOutcome::Subscription(SubscriptionEvent {
                event_id: event.id.clone(),
                tenant_id,
                customer_id,
                subscription_id,
                tier,
                past_due_warning: past_due,
            })
        }
        "customer.subscription.deleted" => {
            let Some(tenant_id) = parse_tenant(data) else {
                return WebhookOutcome::Ignored;
            };
            let customer_id = data
                .get("customer")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            WebhookOutcome::Subscription(SubscriptionEvent {
                event_id: event.id.clone(),
                tenant_id,
                customer_id,
                subscription_id: None,
                tier: Tier::Free,
                past_due_warning: false,
            })
        }
        _ => WebhookOutcome::Ignored,
    }
}

fn parse_tenant(data: &serde_json::Value) -> Option<TenantId> {
    let metadata = data.get("metadata")?;
    let raw = metadata.get("tenant_id").and_then(|v| v.as_str())?;
    Uuid::parse_str(raw).ok().map(TenantId)
}

fn tier_for_status(data: &serde_json::Value, status: &str) -> (Tier, bool) {
    if matches!(status, "past_due" | "unpaid" | "canceled" | "incomplete_expired") {
        return (Tier::Free, matches!(status, "past_due" | "unpaid"));
    }
    let metadata_tier = data
        .get("metadata")
        .and_then(|m| m.get("tier"))
        .and_then(|v| v.as_str());
    let items_tier = data
        .get("items")
        .and_then(|i| i.get("data"))
        .and_then(|d| d.as_array())
        .and_then(|arr| arr.first())
        .and_then(|item| item.get("price"))
        .and_then(|p| p.get("metadata"))
        .and_then(|m| m.get("tier"))
        .and_then(|v| v.as_str());
    let tier = metadata_tier
        .or(items_tier)
        .and_then(|raw| Tier::from_str(raw).ok())
        .unwrap_or(Tier::Pro);
    (tier, false)
}

/// Top-level Stripe event envelope as posted to the webhook.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StripeEventEnvelope {
    /// Stripe event id (`evt_…`).
    pub id: String,
    /// Event type — e.g. `customer.subscription.created`.
    #[serde(rename = "type")]
    pub event_type: String,
    /// Wrapped data envelope holding the affected object.
    pub data: StripeEventData,
}

/// `data` field of [`StripeEventEnvelope`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StripeEventData {
    /// The object Stripe is reporting on (subscription, invoice, …).
    pub object: serde_json::Value,
}

#[cfg(test)]
#[path = "webhook_tests.rs"]
mod tests;
