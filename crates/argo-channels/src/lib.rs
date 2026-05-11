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
pub mod email_inbound;
pub mod discord;
pub mod fallback;
pub mod imessage;
pub mod manifest;
pub mod operator_preferences;
pub mod operator_risk_overrides;
pub mod push;
pub mod push_format;
pub mod quiet_hours;
pub mod rate_limit;
pub mod slack;
pub mod slack_inbound;
pub mod sms;
pub mod teams;
pub mod teams_payload;
pub mod token;
pub mod voice;
pub mod voice_format;
pub mod webinbox;
pub mod whatsapp;

pub use discord::DiscordChannel;
pub use teams::{TeamsApprovalChannel, TeamsApprovalChannelConfig};
pub use teams_payload::TeamsAttachment;
pub use operator_preferences::{
    InMemoryPreferencesSource, OperatorId, OperatorPreferences, OperatorPreferencesResolver,
    OperatorPreferencesSource, OperatorQuietHours, TomlPreferencesSource,
};
pub use operator_risk_overrides::{
    DefaultRiskPolicy, InMemoryOperatorRiskOverridesSource, OperatorRiskOverrides,
    OperatorRiskOverridesSource, OverrideScope, RiskResolutionTrace, RiskResolver,
    TomlOperatorRiskOverridesSource,
};
pub use quiet_hours::QuietHours;
pub use rate_limit::{RateLimit, RateLimitConfig};
pub use slack_inbound::InboundChannel;

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
    /// Per-channel token-bucket budget exhausted. Callers should
    /// back off for `retry_after` and retry; the dispatcher caps
    /// this at 60s and bounds the retry count.
    #[error("channel rate-limited: retry after {} ms", retry_after.as_millis())]
    RateLimited {
        /// Longest wait across exhausted windows.
        retry_after: std::time::Duration,
    },
}

/// Stable identifier for a notification channel implementation.
///
/// ```
/// use argo_channels::ChannelId;
/// let id = ChannelId::new("email");
/// assert_eq!(id.to_string(), "email");
/// let json = serde_json::to_string(&id).unwrap();
/// let back: ChannelId = serde_json::from_str(&json).unwrap();
/// assert_eq!(back, id);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChannelId(pub String);

impl ChannelId {
    /// Construct a channel id from a string.
    ///
    /// ```
    /// use argo_channels::ChannelId;
    /// assert_eq!(ChannelId::new("push"), ChannelId("push".into()));
    /// ```
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
///
/// ```
/// use argo_channels::RiskLevel;
/// assert_eq!(RiskLevel::Low.to_string(), "low");
/// assert_eq!(RiskLevel::High.to_string(), "high");
/// let json = serde_json::to_string(&RiskLevel::Medium).unwrap();
/// assert_eq!(json, "\"medium\"");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

    /// Operator quiet-hours window for this channel, if configured.
    ///
    /// Default returns `None`. Channels that respect quiet hours
    /// override this to surface their config field, and their `send`
    /// implementations check it before attempting delivery — returning
    /// a `DeliveryStatus::Deferred` receipt with `deferred_reason =
    /// Some("quiet_hours".into())` when the window matches.
    fn quiet_hours(&self) -> Option<&QuietHours> {
        None
    }
}

/// Build the standard "deferred for quiet hours" delivery receipt.
///
/// Channels call this from `send` when their `quiet_hours()` window
/// matches `now`, before attempting any provider call. The
/// `deferred_reason` string is `"quiet_hours"` byte-for-byte across
/// every channel — operators grep this string in audit logs.
#[must_use]
pub fn quiet_hours_deferred_receipt(
    channel_id: ChannelId,
    notification_id: String,
) -> DeliveryReceipt {
    DeliveryReceipt {
        channel_id,
        notification_id,
        provider_message_id: None,
        status: DeliveryStatus::Deferred,
        deferred_reason: Some("quiet_hours".into()),
        delivered_at: Utc::now(),
    }
}

#[cfg(test)]
#[path = "quiet_hours_tests.rs"]
mod quiet_hours_tests;
