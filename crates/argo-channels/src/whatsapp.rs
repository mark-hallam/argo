//! Twilio-backed WhatsApp approval channel.

use crate::rate_limit::{RateLimit, RateLimitConfig};
use crate::{
    ChannelError, ChannelId, ChannelResult, DeliveryReceipt, DeliveryStatus, InboundEvent,
    Notification, NotifyChannel, RiskLevel,
};
use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Configuration for [`WhatsAppApprovalChannel`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhatsAppApprovalChannelConfig {
    /// Channel id.
    pub channel_id: ChannelId,
    /// Twilio Account SID.
    pub account_sid: String,
    /// Twilio auth token.
    pub auth_token: String,
    /// Twilio WhatsApp sender, e.g. `whatsapp:+14155238886`.
    pub from_number: String,
    /// Twilio Messages API endpoint.
    pub endpoint: String,
    /// Per-operator WhatsApp number map.
    pub operator_whatsapp_numbers: BTreeMap<String, String>,
    /// Operator override for the per-channel rate-limit budget.
    /// Missing fields fall back to the WhatsApp default
    /// (5/sec, 30/min).
    #[serde(default)]
    pub rate_limit: RateLimitConfig,
}

impl WhatsAppApprovalChannelConfig {
    /// Build config from environment variables and an operator map.
    pub fn from_env(operator_whatsapp_numbers: BTreeMap<String, String>) -> ChannelResult<Self> {
        let account_sid = env_required("TWILIO_ACCOUNT_SID")?;
        let auth_token = env_required("TWILIO_AUTH_TOKEN")?;
        let from_number = env_required("TWILIO_WHATSAPP_FROM_NUMBER")?;
        Ok(Self {
            endpoint: format!(
                "https://api.twilio.com/2010-04-01/Accounts/{account_sid}/Messages.json"
            ),
            channel_id: ChannelId::new("whatsapp"),
            account_sid,
            auth_token,
            from_number: whatsapp_address(&from_number),
            operator_whatsapp_numbers,
            rate_limit: RateLimitConfig::default(),
        })
    }

    /// Test config that points at a closed local port.
    #[must_use]
    pub fn disabled_for_tests(operator_whatsapp_numbers: BTreeMap<String, String>) -> Self {
        Self {
            channel_id: ChannelId::new("whatsapp"),
            account_sid: "AC_test".into(),
            auth_token: "test-token".into(),
            from_number: "whatsapp:+14155238886".into(),
            endpoint: "http://127.0.0.1:9/Messages.json".into(),
            operator_whatsapp_numbers,
            rate_limit: RateLimitConfig::unlimited(),
        }
    }
}

/// WhatsApp approval channel backed by Twilio Messages.
pub struct WhatsAppApprovalChannel {
    config: WhatsAppApprovalChannelConfig,
    client: reqwest::Client,
    rate_limit: RateLimit,
}

impl WhatsAppApprovalChannel {
    /// Create a WhatsApp approval channel.
    #[must_use]
    pub fn new(config: WhatsAppApprovalChannelConfig) -> Self {
        let rate_limit = RateLimit::from_config(
            RateLimitConfig::whatsapp_default().with_override(config.rate_limit),
        );
        Self {
            config,
            client: reqwest::Client::new(),
            rate_limit,
        }
    }

    /// Render the WhatsApp body with reply keywords and signed tokens.
    pub fn message_body(&self, n: &Notification) -> ChannelResult<String> {
        let approve = token_from_url(&n.approve_url)?;
        let reject = token_from_url(&n.reject_url)?;
        Ok(format!(
            "*Approval needed*\n{}\n\nRisk: `{}`\nAction: `{}`\n\nReply `yes {approve}` to approve.\nReply `no {reject}` to deny.\nExpires: {}",
            n.prompt_body,
            n.risk_level,
            n.action_id,
            n.expiry.to_rfc3339()
        ))
    }

    fn phone_for(&self, n: &Notification) -> ChannelResult<String> {
        self.config
            .operator_whatsapp_numbers
            .get(&n.target.operator_id)
            .cloned()
            .map(|v| whatsapp_address(&v))
            .ok_or_else(|| {
                ChannelError::Config(format!(
                    "missing WhatsApp number for operator {}",
                    n.target.operator_id
                ))
            })
    }
}

#[async_trait]
impl NotifyChannel for WhatsAppApprovalChannel {
    fn id(&self) -> ChannelId {
        self.config.channel_id.clone()
    }

    async fn send(&self, notification: Notification) -> ChannelResult<DeliveryReceipt> {
        if let Err(retry_after) = self.rate_limit.try_acquire() {
            return Err(ChannelError::RateLimited { retry_after });
        }
        if notification.risk_level != RiskLevel::High {
            return Ok(DeliveryReceipt {
                channel_id: self.id(),
                notification_id: notification.approval_id,
                provider_message_id: None,
                status: DeliveryStatus::Deferred,
                deferred_reason: Some("whatsapp_reserved_for_high_risk".into()),
                delivered_at: Utc::now(),
            });
        }
        let from = whatsapp_address(&self.config.from_number);
        let to = self.phone_for(&notification)?;
        let body = self.message_body(&notification)?;
        let params = [
            ("From", from.as_str()),
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
            .map_err(|e| ChannelError::Provider(format!("twilio whatsapp send: {e}")))?;
        let status = resp.status();
        let raw = resp
            .text()
            .await
            .map_err(|e| ChannelError::Provider(format!("twilio whatsapp response: {e}")))?;
        if !status.is_success() {
            return Err(ChannelError::Provider(format!(
                "twilio whatsapp returned {status}: {raw}"
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
}

fn token_from_url(url: &str) -> ChannelResult<String> {
    url.split_once('?')
        .and_then(|(_, q)| q.split('&').find_map(|kv| kv.strip_prefix("token=")))
        .map(str::to_string)
        .ok_or_else(|| ChannelError::Config("WhatsApp approval URL missing token".into()))
}

fn whatsapp_address(value: &str) -> String {
    let value = value.trim();
    if value.starts_with("whatsapp:") {
        value.to_string()
    } else {
        format!("whatsapp:{value}")
    }
}

fn env_required(name: &str) -> ChannelResult<String> {
    argo_config::cred::get(name)
        .ok()
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| ChannelError::Config(format!("missing env {name}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{OperatorTarget, RiskLevel};
    use chrono::{Duration, Utc};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    fn notification(risk_level: RiskLevel) -> Notification {
        Notification {
            target: OperatorTarget {
                operator_id: "op-1".into(),
                email: "operator@example.com".into(),
            },
            action_id: "act-1".into(),
            approval_id: "appr-1".into(),
            risk_level,
            prompt_body: "Approve high-risk refund?".into(),
            approve_url: "https://gw.example.com/decision?token=appr-1.approve.999.sig".into(),
            reject_url: "https://gw.example.com/decision?token=appr-1.reject.999.sig".into(),
            expiry: Utc::now() + Duration::minutes(5),
        }
    }

    fn channel(endpoint: &str) -> WhatsAppApprovalChannel {
        let mut phones = BTreeMap::new();
        phones.insert("op-1".into(), "+15555550199".into());
        let mut cfg = WhatsAppApprovalChannelConfig::disabled_for_tests(phones);
        cfg.endpoint = endpoint.into();
        WhatsAppApprovalChannel::new(cfg)
    }

    #[test]
    fn body_contains_reply_keywords_and_tokens() {
        let body = channel("http://127.0.0.1:9/Messages.json")
            .message_body(&notification(RiskLevel::High))
            .unwrap();
        assert!(body.contains("yes appr-1.approve.999.sig"));
        assert!(body.contains("no appr-1.reject.999.sig"));
    }

    #[test]
    fn whatsapp_address_prefixes_plain_numbers() {
        assert_eq!(whatsapp_address("+15555550199"), "whatsapp:+15555550199");
        assert_eq!(
            whatsapp_address("whatsapp:+15555550199"),
            "whatsapp:+15555550199"
        );
    }

    #[tokio::test]
    async fn non_high_risk_whatsapp_is_deferred() {
        let receipt = channel("http://127.0.0.1:9/Messages.json")
            .send(notification(RiskLevel::Medium))
            .await
            .unwrap();
        assert_eq!(receipt.status, DeliveryStatus::Deferred);
        assert_eq!(
            receipt.deferred_reason.as_deref(),
            Some("whatsapp_reserved_for_high_risk")
        );
    }

    #[tokio::test]
    async fn mocked_twilio_message_returns_receipt() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = [0_u8; 4096];
            let n = stream.read(&mut buf).await.unwrap();
            let req = String::from_utf8_lossy(&buf[..n]);
            assert!(req.contains("POST /Messages.json HTTP/1.1"));
            assert!(req.contains("From=whatsapp%3A%2B14155238886"));
            assert!(req.contains("To=whatsapp%3A%2B15555550199"));
            let body = r#"{"sid":"SM123","status":"queued"}"#;
            let resp = format!(
                "HTTP/1.1 201 Created\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(resp.as_bytes()).await.unwrap();
        });
        let receipt = channel(&format!("http://{addr}/Messages.json"))
            .send(notification(RiskLevel::High))
            .await
            .unwrap();
        assert_eq!(receipt.status, DeliveryStatus::Sent);
        assert_eq!(receipt.provider_message_id.as_deref(), Some("SM123"));
        server.await.unwrap();
    }
}
