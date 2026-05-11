//! Wire types for the Mailgun connector.
//!
//! Splits payloads into a small typed `MailgunMessage` (caller authors), a
//! `MailgunSendResponse` (Mailgun returns), and a `MailgunWebhookEvent`
//! (Mailgun POSTs back to our webhook). The wire formats are
//! `application/x-www-form-urlencoded` for outbound and `application/json`
//! (with an `event-data` envelope) for inbound — see [`crate::client`] and
//! [`crate::webhook`] for the encode/decode glue.

use serde::{Deserialize, Serialize};

/// Mailgun region. Affects the base URL only — auth and payload shape are
/// identical between regions. Operators choose `Eu` for GDPR data residency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MailgunRegion {
    /// US region: `https://api.mailgun.net`.
    #[serde(rename = "us")]
    Us,
    /// EU region: `https://api.eu.mailgun.net`.
    #[serde(rename = "eu")]
    Eu,
}

impl MailgunRegion {
    /// Base URL for this region (no trailing slash).
    #[must_use]
    pub fn base_url(self) -> &'static str {
        match self {
            Self::Us => "https://api.mailgun.net",
            Self::Eu => "https://api.eu.mailgun.net",
        }
    }
}

/// A single outbound email to send via Mailgun.
///
/// Field semantics match Mailgun's `messages` endpoint
/// (<https://documentation.mailgun.com/en/latest/api-sending.html>). Either
/// `text` or `html` (or both) should be set; Mailgun rejects messages with
/// neither. `tags` are operator-supplied labels Mailgun surfaces in webhook
/// events for fan-in analytics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailgunMessage {
    /// `From:` header value, e.g. `"Argo <noreply@example.com>"`.
    pub from: String,
    /// One or more primary recipients.
    pub to: Vec<String>,
    /// Cc recipients (visible to all). Empty vec is fine.
    pub cc: Vec<String>,
    /// Bcc recipients (hidden). Empty vec is fine.
    pub bcc: Vec<String>,
    /// `Subject:` header.
    pub subject: String,
    /// Plain-text body.
    pub text: Option<String>,
    /// HTML body.
    pub html: Option<String>,
    /// Optional `Reply-To:` header.
    pub reply_to: Option<String>,
    /// Mailgun tags (max 3, alphanumeric + `-`/`_`).
    pub tags: Vec<String>,
}

/// Mailgun's `200 OK` response shape for a successful send.
///
/// `message_id` is the angle-bracketed Message-ID Mailgun assigns (e.g.
/// `"<abc@mg.example.com>"`); we strip the brackets if present so the value
/// is comparable across Mailgun's webhook events (which omit the brackets).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailgunSendResponse {
    /// Mailgun-assigned Message-ID, brackets stripped.
    pub id: String,
    /// Mailgun confirmation message (e.g. `"Queued. Thank you."`).
    pub message: String,
}

impl MailgunSendResponse {
    /// Convenience accessor matching the brief's `message_id` naming.
    /// Mailgun's wire format uses `id`; this is the same value.
    #[must_use]
    pub fn message_id(&self) -> &str {
        &self.id
    }
}

/// Mailgun webhook event kinds.
///
/// Maps to the seven standard event names Mailgun emits over webhooks.
/// `failed` is Mailgun's permanent-failure event (distinct from `bounced`,
/// which is the bounce-specific subtype).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MailgunEventKind {
    /// Mail delivered to recipient MX.
    Delivered,
    /// Recipient opened the message.
    Opened,
    /// Recipient clicked a tracked link.
    Clicked,
    /// Hard or soft bounce.
    Bounced,
    /// Recipient unsubscribed.
    Unsubscribed,
    /// Recipient marked as spam.
    Complained,
    /// Permanent send failure.
    Failed,
}

/// Decoded Mailgun webhook event (the `event-data` payload).
///
/// Mailgun POSTs JSON like:
/// ```json
/// {
///   "signature": { "timestamp": "...", "token": "...", "signature": "..." },
///   "event-data": {
///     "event": "delivered",
///     "recipient": "to@example.com",
///     "message": { "headers": { "message-id": "abc@mg.example.com" } },
///     "timestamp": 1700000000.123
///   }
/// }
/// ```
/// This struct represents the `event-data` half. The signature half is
/// verified separately via [`crate::verify_webhook`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailgunWebhookEvent {
    /// Event kind.
    pub event: MailgunEventKind,
    /// Recipient email address.
    pub recipient: String,
    /// Mailgun Message-ID this event refers to.
    pub message_id: String,
    /// Unix epoch seconds (fractional) Mailgun captured the event.
    pub timestamp: f64,
}
