//! Postmark-backed email notification channel.

// Back-compat re-export: prior to the channel-wide quiet-hours lift,
// `QuietHours` lived in this module. Downstream crates and siblings
// (notably `crate::manifest`) imported it from `crate::email`. Keep
// the alias here so that those import paths still resolve.
pub use crate::quiet_hours::QuietHours;
use crate::rate_limit::{RateLimit, RateLimitConfig};
use crate::{
    quiet_hours_deferred_receipt, ChannelError, ChannelId, ChannelResult, DeliveryReceipt,
    DeliveryStatus, InboundEvent, Notification, NotifyChannel,
};
use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};

/// Configuration for [`EmailChannel`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailChannelConfig {
    /// Channel id.
    pub channel_id: ChannelId,
    /// Postmark server token.
    pub postmark_token: String,
    /// From address.
    pub from_address: String,
    /// Subject template.
    pub subject_template: String,
    /// Postmark `/email` endpoint.
    pub endpoint: String,
    /// Optional Postmark message stream.
    pub message_stream: Option<String>,
    /// Operator quiet hours.
    pub quiet_hours: QuietHours,
    /// Operator override for the per-channel rate-limit budget.
    /// Missing fields fall back to the email default
    /// (5/sec, 30/min).
    #[serde(default)]
    pub rate_limit: RateLimitConfig,
}

impl EmailChannelConfig {
    /// Test config that never calls Postmark when quiet hours match.
    #[must_use]
    pub fn disabled_for_tests() -> Self {
        Self {
            channel_id: ChannelId::new("email"),
            postmark_token: "test-token".into(),
            from_address: "worker@example.com".into(),
            subject_template: "[{{risk_level}}] Approval needed: {{action_id}}".into(),
            endpoint: "http://127.0.0.1:9/email".into(),
            message_stream: None,
            quiet_hours: QuietHours::default(),
            rate_limit: RateLimitConfig::unlimited(),
        }
    }
}

/// Email approval channel backed by Postmark.
pub struct EmailChannel {
    config: EmailChannelConfig,
    client: reqwest::Client,
    rate_limit: RateLimit,
}

impl EmailChannel {
    /// Create an email channel from config.
    #[must_use]
    pub fn new(config: EmailChannelConfig) -> Self {
        let rate_limit = RateLimit::from_config(
            RateLimitConfig::email_default().with_override(config.rate_limit),
        );
        Self {
            config,
            client: reqwest::Client::new(),
            rate_limit,
        }
    }

    /// Parse an inbound Postmark webhook into a generic inbound event.
    pub fn parse_inbound_event(
        &self,
        raw: serde_json::Value,
        approval_id: String,
        decision: crate::token::ApprovalDecision,
    ) -> InboundEvent {
        let actor = raw
            .get("From")
            .and_then(|v| v.as_str())
            .or_else(|| raw.get("FromFull")?.get("Email").and_then(|v| v.as_str()))
            .unwrap_or("email-operator")
            .to_string();
        let note = raw
            .get("StrippedTextReply")
            .and_then(|v| v.as_str())
            .or_else(|| raw.get("TextBody").and_then(|v| v.as_str()))
            .map(str::to_string);
        InboundEvent {
            channel_id: self.id(),
            approval_id,
            decision,
            actor,
            note,
            received_at: Utc::now(),
            raw,
        }
    }

    fn render_subject(&self, n: &Notification) -> String {
        self.config
            .subject_template
            .replace("{{action_id}}", &n.action_id)
            .replace("{{risk_level}}", &n.risk_level.to_string())
    }

    fn text_body(&self, n: &Notification) -> String {
        format!(
            "{}\n\nApprove: {}\nReject: {}\n\napproval_id: {}\nexpires: {}\n",
            n.prompt_body,
            n.approve_url,
            n.reject_url,
            n.approval_id,
            n.expiry.to_rfc3339()
        )
    }

    fn html_body(&self, n: &Notification) -> String {
        format!(
            "<p>{}</p><p><a href=\"{}\">Approve</a> | \
             <a href=\"{}\">Reject</a></p><p>approval_id: {}</p>",
            n.prompt_body, n.approve_url, n.reject_url, n.approval_id
        )
    }
}

#[async_trait]
impl NotifyChannel for EmailChannel {
    fn id(&self) -> ChannelId {
        self.config.channel_id.clone()
    }

    async fn send(&self, notification: Notification) -> ChannelResult<DeliveryReceipt> {
        if let Err(retry_after) = self.rate_limit.try_acquire() {
            return Err(ChannelError::RateLimited { retry_after });
        }
        if self.config.quiet_hours.contains(Utc::now()) {
            tracing::info!(
                channel = %self.config.channel_id,
                approval_id = %notification.approval_id,
                "deferred-send: quiet_hours"
            );
            return Ok(quiet_hours_deferred_receipt(
                self.id(),
                notification.approval_id,
            ));
        }
        let mut payload = serde_json::json!({
            "From": &self.config.from_address,
            "To": &notification.target.email,
            "Subject": self.render_subject(&notification),
            "TextBody": self.text_body(&notification),
            "HtmlBody": self.html_body(&notification),
        });
        if let Some(stream) = self.config.message_stream.as_deref() {
            if let Some(map) = payload.as_object_mut() {
                map.insert(
                    "MessageStream".into(),
                    serde_json::Value::String(stream.into()),
                );
            }
        }
        let resp = self
            .client
            .post(&self.config.endpoint)
            .header("X-Postmark-Server-Token", &self.config.postmark_token)
            .header("Accept", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| ChannelError::Provider(format!("postmark send: {e}")))?;
        let status = resp.status();
        let body = resp
            .text()
            .await
            .map_err(|e| ChannelError::Provider(format!("postmark response: {e}")))?;
        if !status.is_success() {
            return Err(ChannelError::Provider(format!(
                "postmark returned {status}: {body}"
            )));
        }
        let provider_message_id = serde_json::from_str::<serde_json::Value>(&body)
            .ok()
            .and_then(|v| {
                v.get("MessageID")
                    .and_then(|id| id.as_str())
                    .map(str::to_string)
            });
        Ok(DeliveryReceipt {
            channel_id: self.id(),
            notification_id: notification.approval_id,
            provider_message_id,
            status: DeliveryStatus::Sent,
            deferred_reason: None,
            delivered_at: Utc::now(),
        })
    }

    async fn poll(&self) -> ChannelResult<Vec<InboundEvent>> {
        Ok(Vec::new())
    }

    fn quiet_hours(&self) -> Option<&QuietHours> {
        Some(&self.config.quiet_hours)
    }
}
