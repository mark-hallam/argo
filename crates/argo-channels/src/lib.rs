//! Notification channel primitives for SKYCOT approval gates.
//!
//! The crate owns the transport-neutral [`NotifyChannel`] trait,
//! common notification/event data, HMAC approval tokens, and the
//! first concrete channel: Postmark-backed email.
//!
//! ```no_run
//! # async fn demo() -> argo_channels::ChannelResult<()> {
//! use argo_channels::{NotifyChannel, Notification};
//! use argo_channels::email::{EmailChannel, EmailChannelConfig};
//!
//! let channel = EmailChannel::new(EmailChannelConfig::disabled_for_tests());
//! let _ = channel.poll().await?;
//! # let _ = std::mem::size_of::<Notification>();
//! # Ok(()) }
//! ```
#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

pub mod email;
pub mod imessage;
pub mod manifest;
pub mod push;
pub mod slack;
pub mod token;
pub mod voice;
pub mod webinbox;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Result type used by channel implementations.
pub type ChannelResult<T> = Result<T, ChannelError>;

/// Errors returned by notification channels.
#[derive(Debug, Error)]
pub enum ChannelError {
    /// Static configuration is invalid or missing.
    #[error("channel config: {0}")]
    Config(String),
    /// Signing or signature verification failed.
    #[error("channel auth: {0}")]
    Auth(String),
    /// Provider request failed.
    #[error("channel provider: {0}")]
    Provider(String),
    /// Payload parse failed.
    #[error("channel parse: {0}")]
    Parse(String),
}

/// Stable identifier for a notification channel implementation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChannelId(pub String);

impl ChannelId {
    /// Construct a channel id from a string.
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl std::fmt::Display for ChannelId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Operator endpoint targeted by a notification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorTarget {
    /// Stable operator id, when known.
    pub operator_id: String,
    /// Email address for email delivery.
    pub email: String,
}

/// Risk level attached to the action awaiting approval.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RiskLevel {
    /// Read-only or draft-only work.
    Low,
    /// Reversible public or monetary side effects.
    Medium,
    /// Human-touching, irreversible, or high-cost side effects.
    High,
}

impl std::fmt::Display for RiskLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        };
        f.write_str(s)
    }
}

/// Approval request to send over a notification channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    /// Target operator.
    pub target: OperatorTarget,
    /// Worker action id.
    pub action_id: String,
    /// Approval id carried by signed decision URLs.
    pub approval_id: String,
    /// Manifest-stamped risk level.
    pub risk_level: RiskLevel,
    /// Human-readable prompt body.
    pub prompt_body: String,
    /// Signed approve URL.
    pub approve_url: String,
    /// Signed reject URL.
    pub reject_url: String,
    /// Approval expiry.
    pub expiry: DateTime<Utc>,
}

/// Provider delivery status for a notification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryStatus {
    /// Provider accepted the message.
    Sent,
    /// Send was intentionally deferred.
    Deferred,
}

/// Receipt returned from [`NotifyChannel::send`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryReceipt {
    /// Channel that handled the send.
    pub channel_id: ChannelId,
    /// Approval id or channel-specific notification id.
    pub notification_id: String,
    /// Provider message id, when one exists.
    pub provider_message_id: Option<String>,
    /// Delivery status.
    pub status: DeliveryStatus,
    /// Reason when status is [`DeliveryStatus::Deferred`].
    pub deferred_reason: Option<String>,
    /// Time the channel returned the receipt.
    pub delivered_at: DateTime<Utc>,
}

/// Inbound approval-related event received from a channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboundEvent {
    /// Channel that received the event.
    pub channel_id: ChannelId,
    /// Approval id from the signed token.
    pub approval_id: String,
    /// Operator decision.
    pub decision: token::ApprovalDecision,
    /// Human actor extracted from the inbound payload.
    pub actor: String,
    /// Optional note or reply body.
    pub note: Option<String>,
    /// Time the event was parsed.
    pub received_at: DateTime<Utc>,
    /// Provider raw payload for audit/debug.
    pub raw: serde_json::Value,
}

/// Shared trait implemented by every approval notification channel.
#[async_trait]
pub trait NotifyChannel: Send + Sync {
    /// Return the stable channel id.
    fn id(&self) -> ChannelId;

    /// Send an approval notification.
    async fn send(&self, notification: Notification) -> ChannelResult<DeliveryReceipt>;

    /// Poll for inbound events.
    ///
    /// Email returns an empty list because inbound replies are
    /// webhook-driven.
    async fn poll(&self) -> ChannelResult<Vec<InboundEvent>>;
}
