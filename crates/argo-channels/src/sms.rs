//! Twilio-backed SMS approval channel.

use crate::quiet_hours::QuietHours;
use crate::rate_limit::{RateLimit, RateLimitConfig};
use crate::{
    quiet_hours_deferred_receipt, ChannelError, ChannelId, ChannelResult, DeliveryReceipt,
    DeliveryStatus, InboundEvent, Notification, NotifyChannel, RiskLevel,
};
use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Configuration for [`SmsApprovalChannel`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmsApprovalChannelConfig {
    /// Channel id.
    pub channel_id: ChannelId,
    /// Twilio Account SID.
    pub account_sid: String,
    /// Twilio auth token.
    pub auth_token: String,
    /// Twilio sender phone number.
    pub from_number: String,
    /// Twilio Messages API endpoint.
    pub endpoint: String,
    /// Per-operator phone-number map.
    pub operator_phone_numbers: BTreeMap<String, String>,
    /// Operator override for the per-channel rate-limit budget.
    /// Missing fields fall back to the SMS default (1/sec, 10/min).
    #[serde(default)]
    pub rate_limit: RateLimitConfig,
    /// Operator quiet hours. `None` means quiet hours are not configured
    /// for this channel — today's behavior preserved.
    #[serde(default)]
    pub quiet_hours: Option<QuietHours>,
}

impl SmsApprovalChannelConfig {
    /// Build config from environment variables and an operator map.
    pub fn from_env(operator_phone_numbers: BTreeMap<String, String>) -> ChannelResult<Self> {
        let account_sid = env_required("TWILIO_ACCOUNT_SID")?;
        let auth_token = env_required("TWILIO_AUTH_TOKEN")?;
        let from_number = env_required("TWILIO_FROM_NUMBER")?;
        Ok(Self {
            endpoint: format!(
                "https://api.twilio.com/2010-04-01/Accounts/{account_sid}/Messages.json"
            ),
            channel_id: ChannelId::new("sms"),
            account_sid,
            auth_token,
            from_number,
            operator_phone_numbers,
            rate_limit: RateLimitConfig::default(),
            quiet_hours: None,
        })
    }

    /// Test config that points at a closed local port.
    #[must_use]
    pub fn disabled_for_tests(operator_phone_numbers: BTreeMap<String, String>) -> Self {
        Self {
            channel_id: ChannelId::new("sms"),
            account_sid: "AC_test".into(),
            auth_token: "test-token".into(),
            from_number: "+15555550100".into(),
            endpoint: "http://127.0.0.1:9/Messages.json".into(),
            operator_phone_numbers,
            rate_limit: RateLimitConfig::unlimited(),
            quiet_hours: None,
        }
    }
}

/// SMS approval channel backed by Twilio.
pub struct SmsApprovalChannel {
    config: SmsApprovalChannelConfig,
    client: reqwest::Client,
    rate_limit: RateLimit,
}

impl SmsApprovalChannel {
    /// Create an SMS approval channel.
    #[must_use]
    pub fn new(config: SmsApprovalChannelConfig) -> Self {
        let rate_limit = RateLimit::from_config(
            RateLimitConfig::sms_default().with_override(config.rate_limit),
        );
        Self {
            config,
            client: reqwest::Client::new(),
            rate_limit,
        }
    }

    /// Render the SMS body containing one approve and one reject link.
    #[must_use]
    pub fn message_body(&self, n: &Notification) -> String {
        format!(
            "{}\nApprove: {}\nReject: {}\nExpires: {}",
            n.prompt_body,
            n.approve_url,
            n.reject_url,
            n.expiry.to_rfc3339()
        )
    }

    fn phone_for(&self, n: &Notification) -> ChannelResult<String> {
        self.config
            .operator_phone_numbers
            .get(&n.target.operator_id)
            .cloned()
            .ok_or_else(|| {
                ChannelError::Config(format!(
                    "missing SMS phone number for operator {}",
                    n.target.operator_id
                ))
            })
    }
}

#[async_trait]
impl NotifyChannel for SmsApprovalChannel {
    fn id(&self) -> ChannelId {
        self.config.channel_id.clone()
    }

    async fn send(&self, notification: Notification) -> ChannelResult<DeliveryReceipt> {
        if let Err(retry_after) = self.rate_limit.try_acquire() {
            return Err(ChannelError::RateLimited { retry_after });
        }
        if let Some(quiet) = self.config.quiet_hours.as_ref() {
            if quiet.contains(Utc::now()) {
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
        }
        if notification.risk_level != RiskLevel::High {
            return Ok(DeliveryReceipt {
                channel_id: self.id(),
                notification_id: notification.approval_id,
                provider_message_id: None,
                status: DeliveryStatus::Deferred,
                deferred_reason: Some("sms_reserved_for_high_risk".into()),
                delivered_at: Utc::now(),
            });
        }
        let to = self.phone_for(&notification)?;
        let body = self.message_body(&notification);
        let params = [
            ("From", self.config.from_number.as_str()),
            ("To", to.as_str()),
            ("Body", body.as_str()),
        ];
        let resp = self
            .client
            .post(&self.config.endpoint)
            .basic_auth(&self.config.account_sid, Some(&self.config.auth_token))
            .form(&params)
            .send()
            .await
            .map_err(|e| ChannelError::Provider(format!("twilio send: {e}")))?;
        let status = resp.status();
        let raw = resp
            .text()
            .await
            .map_err(|e| ChannelError::Provider(format!("twilio response: {e}")))?;
        if !status.is_success() {
            return Err(ChannelError::Provider(format!(
                "twilio returned {status}: {raw}"
            )));
        }
        let provider_message_id = serde_json::from_str::<serde_json::Value>(&raw)
            .ok()
            .and_then(|v| {
                v.get("sid")
                    .and_then(|sid| sid.as_str())
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
        self.config.quiet_hours.as_ref()
    }
}

fn env_required(name: &str) -> ChannelResult<String> {
    let value = argo_config::cred::get(name)
        .map_err(|_| ChannelError::Config(format!("missing env {name}")))?;
    if value.is_empty() {
        Err(ChannelError::Config(format!(
            "env {name} must not be empty"
        )))
    } else {
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{OperatorTarget, RiskLevel};
    use chrono::{Duration, Utc};

    fn notification(risk_level: RiskLevel) -> Notification {
        Notification {
            target: OperatorTarget {
                operator_id: "op-1".into(),
                email: "operator@example.com".into(),
            },
            action_id: "act-1".into(),
            approval_id: "act-1".into(),
            risk_level,
            prompt_body: "Approve high-risk action?".into(),
            approve_url: "https://example.com/channels/sms/decision/approve-token".into(),
            reject_url: "https://example.com/channels/sms/decision/reject-token".into(),
            expiry: Utc::now() + Duration::minutes(5),
        }
    }

    fn channel() -> SmsApprovalChannel {
        let mut phones = BTreeMap::new();
        phones.insert("op-1".into(), "+15555550199".into());
        SmsApprovalChannel::new(SmsApprovalChannelConfig::disabled_for_tests(phones))
    }

    #[test]
    fn body_contains_approve_and_reject_links() {
        let body = channel().message_body(&notification(RiskLevel::High));
        assert!(body.contains("Approve: https://example.com/channels/sms/decision/approve-token"));
        assert!(body.contains("Reject: https://example.com/channels/sms/decision/reject-token"));
    }

    #[tokio::test]
    async fn non_high_risk_sms_is_deferred() {
        let receipt = channel()
            .send(notification(RiskLevel::Medium))
            .await
            .unwrap();
        assert_eq!(receipt.status, DeliveryStatus::Deferred);
        assert_eq!(
            receipt.deferred_reason.as_deref(),
            Some("sms_reserved_for_high_risk")
        );
    }

    #[tokio::test]
    async fn twilio_integration_gated_on_env() {
        let sid = std::env::var("TWILIO_TEST_ACCOUNT_SID").ok();
        let token = std::env::var("TWILIO_TEST_AUTH_TOKEN").ok();
        let from = std::env::var("TWILIO_TEST_FROM").ok();
        let to = std::env::var("TWILIO_TEST_TO").ok();
        let (Some(sid), Some(token), Some(from), Some(to)) = (sid, token, from, to) else {
            return;
        };
        let mut phones = BTreeMap::new();
        phones.insert("op-1".into(), to);
        let channel = SmsApprovalChannel::new(SmsApprovalChannelConfig {
            channel_id: ChannelId::new("sms"),
            endpoint: format!("https://api.twilio.com/2010-04-01/Accounts/{sid}/Messages.json"),
            account_sid: sid,
            auth_token: token,
            from_number: from,
            operator_phone_numbers: phones,
            rate_limit: RateLimitConfig::unlimited(),
            quiet_hours: None,
        });
        let receipt = channel.send(notification(RiskLevel::High)).await.unwrap();
        assert_eq!(receipt.status, DeliveryStatus::Sent);
    }
}
