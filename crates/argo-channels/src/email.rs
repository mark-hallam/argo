//! Postmark-backed email notification channel.

use crate::rate_limit::{RateLimit, RateLimitConfig};
use crate::{
    ChannelError, ChannelId, ChannelResult, DeliveryReceipt, DeliveryStatus, InboundEvent,
    Notification, NotifyChannel,
};
use async_trait::async_trait;
use chrono::{DateTime, Datelike, Duration, Utc};
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
            return Ok(DeliveryReceipt {
                channel_id: self.id(),
                notification_id: notification.approval_id,
                provider_message_id: None,
                status: DeliveryStatus::Deferred,
                deferred_reason: Some("quiet_hours".into()),
                delivered_at: Utc::now(),
            });
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
}

/// Quiet-hours preference for an operator.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct QuietHours {
    /// Whether quiet hours are active.
    pub enabled: bool,
    /// Start time, `HH:MM`, in the configured fixed offset.
    pub start: String,
    /// End time, `HH:MM`, in the configured fixed offset.
    pub end: String,
    /// Fixed offset from UTC in minutes.
    pub timezone_offset_minutes: i32,
    /// Days where quiet hours apply. Empty means every day.
    pub days: Vec<String>,
}

impl Default for QuietHours {
    fn default() -> Self {
        Self {
            enabled: false,
            start: "18:00".into(),
            end: "08:00".into(),
            timezone_offset_minutes: 0,
            days: Vec::new(),
        }
    }
}

impl QuietHours {
    /// Return true when `now` lands inside the configured window.
    #[must_use]
    pub fn contains(&self, now: DateTime<Utc>) -> bool {
        if !self.enabled {
            return false;
        }
        let Some(start) = parse_hhmm(&self.start) else {
            return false;
        };
        let Some(end) = parse_hhmm(&self.end) else {
            return false;
        };
        let local = now + Duration::minutes(i64::from(self.timezone_offset_minutes));
        if !self.days.is_empty() {
            let day = format!("{:?}", local.weekday()).to_lowercase();
            let short = day.chars().take(3).collect::<String>();
            if !self.days.iter().any(|d| d.eq_ignore_ascii_case(&short)) {
                return false;
            }
        }
        let mins = local.hour_minute();
        if start <= end {
            mins >= start && mins < end
        } else {
            mins >= start || mins < end
        }
    }
}

trait HourMinute {
    fn hour_minute(&self) -> u32;
}

impl HourMinute for DateTime<Utc> {
    fn hour_minute(&self) -> u32 {
        use chrono::Timelike;
        self.hour() * 60 + self.minute()
    }
}

fn parse_hhmm(s: &str) -> Option<u32> {
    let (h, m) = s.split_once(':')?;
    let h = h.parse::<u32>().ok()?;
    let m = m.parse::<u32>().ok()?;
    if h < 24 && m < 60 {
        Some(h * 60 + m)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn quiet_hours_handles_overnight_window() {
        let q = QuietHours {
            enabled: true,
            start: "18:00".into(),
            end: "08:00".into(),
            timezone_offset_minutes: 0,
            days: Vec::new(),
        };
        assert!(q.contains(Utc.with_ymd_and_hms(2026, 5, 6, 22, 0, 0).unwrap()));
        assert!(q.contains(Utc.with_ymd_and_hms(2026, 5, 6, 7, 59, 0).unwrap()));
        assert!(!q.contains(Utc.with_ymd_and_hms(2026, 5, 6, 12, 0, 0).unwrap()));
    }
}
