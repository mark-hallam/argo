//! Twilio-backed voice approval channel.

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

pub use crate::voice_format::{
    render_voice_twiml, sign_voice_callback_token, verify_voice_callback_token,
    VoiceCallbackClaims, VoiceDecision,
};

/// Configuration for [`VoiceApprovalChannel`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceApprovalChannelConfig {
    /// Channel id.
    pub channel_id: ChannelId,
    /// Twilio Account SID.
    pub account_sid: String,
    /// Twilio auth token.
    pub auth_token: String,
    /// Twilio caller phone number.
    pub from_number: String,
    /// Twilio Calls API endpoint.
    pub endpoint: String,
    /// Public DTMF callback URL, without the `vt` query parameter.
    pub decision_callback_base_url: String,
    /// HMAC secret used for DTMF callback tokens.
    pub hmac_secret: String,
    /// Per-operator phone-number map.
    pub operator_phone_numbers: BTreeMap<String, String>,
    /// Operator override for the per-channel rate-limit budget.
    /// Missing fields fall back to the voice default
    /// (1/min, 5/hour).
    #[serde(default)]
    pub rate_limit: RateLimitConfig,
    /// Operator quiet hours. `None` means quiet hours are not configured
    /// for this channel — today's behavior preserved.
    #[serde(default)]
    pub quiet_hours: Option<QuietHours>,
}

impl VoiceApprovalChannelConfig {
    /// Build config from environment variables and an operator map.
    pub fn from_env(operator_phone_numbers: BTreeMap<String, String>) -> ChannelResult<Self> {
        let account_sid = env_required("TWILIO_ACCOUNT_SID")?;
        let auth_token = env_required("TWILIO_AUTH_TOKEN")?;
        let from_number = env_first("TWILIO_VOICE_FROM_NUMBER", "TWILIO_FROM_NUMBER")?;
        let decision_callback_base_url = env_required("ARGO_TWILIO_VOICE_DECISION_URL")?;
        let hmac_secret = env_required("ARGO_CHANNEL_HMAC_SECRET")?;
        Ok(Self {
            endpoint: format!(
                "https://api.twilio.com/2010-04-01/Accounts/{account_sid}/Calls.json"
            ),
            channel_id: ChannelId::new("voice"),
            account_sid,
            auth_token,
            from_number,
            decision_callback_base_url,
            hmac_secret,
            operator_phone_numbers,
            rate_limit: RateLimitConfig::default(),
            quiet_hours: None,
        })
    }

    /// Test config that points at a closed local port.
    #[must_use]
    pub fn disabled_for_tests(operator_phone_numbers: BTreeMap<String, String>) -> Self {
        Self {
            channel_id: ChannelId::new("voice"),
            account_sid: "AC_test".into(),
            auth_token: "test-token".into(),
            from_number: "+15555550100".into(),
            endpoint: "http://127.0.0.1:9/Calls.json".into(),
            decision_callback_base_url: "https://gw.example.com/twilio/voice/decision".into(),
            hmac_secret: "test-secret".into(),
            operator_phone_numbers,
            rate_limit: RateLimitConfig::unlimited(),
            quiet_hours: None,
        }
    }
}

/// Voice approval channel backed by Twilio.
pub struct VoiceApprovalChannel {
    config: VoiceApprovalChannelConfig,
    client: reqwest::Client,
    rate_limit: RateLimit,
}

impl VoiceApprovalChannel {
    /// Create a voice approval channel.
    #[must_use]
    pub fn new(config: VoiceApprovalChannelConfig) -> Self {
        let rate_limit = RateLimit::from_config(
            RateLimitConfig::voice_default().with_override(config.rate_limit),
        );
        Self {
            config,
            client: reqwest::Client::new(),
            rate_limit,
        }
    }

    /// Build the signed DTMF callback URL for this notification.
    pub fn callback_url(&self, n: &Notification) -> ChannelResult<String> {
        let nonce = format!("{}-{}", n.action_id, Utc::now().timestamp_millis());
        let token = sign_voice_callback_token(
            &n.approval_id,
            &n.action_id,
            &n.target.operator_id,
            n.expiry.timestamp(),
            &nonce,
            &self.config.hmac_secret,
        )?;
        let sep = if self.config.decision_callback_base_url.contains('?') {
            '&'
        } else {
            '?'
        };
        Ok(format!(
            "{}{sep}vt={token}",
            self.config.decision_callback_base_url
        ))
    }

    /// Render the TwiML body sent to Twilio for the outbound call.
    pub fn twiml_for(&self, n: &Notification) -> ChannelResult<String> {
        Ok(render_voice_twiml(&n.prompt_body, &self.callback_url(n)?))
    }

    fn phone_for(&self, n: &Notification) -> ChannelResult<String> {
        self.config
            .operator_phone_numbers
            .get(&n.target.operator_id)
            .cloned()
            .ok_or_else(|| {
                ChannelError::Config(format!(
                    "missing voice phone number for operator {}",
                    n.target.operator_id
                ))
            })
    }
}

#[async_trait]
impl NotifyChannel for VoiceApprovalChannel {
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
                deferred_reason: Some("voice_reserved_for_high_risk".into()),
                delivered_at: Utc::now(),
            });
        }
        let to = self.phone_for(&notification)?;
        let twiml = self.twiml_for(&notification)?;
        let params = [
            ("From", self.config.from_number.as_str()),
            ("To", to.as_str()),
            ("Twiml", twiml.as_str()),
        ];
        let resp = self
            .client
            .post(&self.config.endpoint)
            .basic_auth(&self.config.account_sid, Some(&self.config.auth_token))
            .form(&params)
            .send()
            .await
            .map_err(|e| ChannelError::Provider(format!("twilio voice send: {e}")))?;
        let status = resp.status();
        let raw = resp
            .text()
            .await
            .map_err(|e| ChannelError::Provider(format!("twilio voice response: {e}")))?;
        if !status.is_success() {
            return Err(ChannelError::Provider(format!(
                "twilio voice returned {status}: {raw}"
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

fn env_first(primary: &str, fallback: &str) -> ChannelResult<String> {
    argo_config::cred::get(primary)
        .ok()
        .filter(|v| !v.is_empty())
        .or_else(|| {
            argo_config::cred::get(fallback)
                .ok()
                .filter(|v| !v.is_empty())
        })
        .ok_or_else(|| ChannelError::Config(format!("missing env {primary} or {fallback}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::OperatorTarget;
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
            prompt_body: "Approve customer refund < $500?".into(),
            approve_url: "https://example.com/approve".into(),
            reject_url: "https://example.com/reject".into(),
            expiry: Utc::now() + Duration::minutes(5),
        }
    }

    fn channel(endpoint: &str) -> VoiceApprovalChannel {
        let mut phones = BTreeMap::new();
        phones.insert("op-1".into(), "+15555550199".into());
        let mut cfg = VoiceApprovalChannelConfig::disabled_for_tests(phones);
        cfg.endpoint = endpoint.into();
        VoiceApprovalChannel::new(cfg)
    }

    #[tokio::test]
    async fn non_high_risk_voice_is_deferred() {
        let receipt = channel("http://127.0.0.1:9/Calls.json")
            .send(notification(RiskLevel::Medium))
            .await
            .unwrap();
        assert_eq!(receipt.status, DeliveryStatus::Deferred);
        assert_eq!(
            receipt.deferred_reason.as_deref(),
            Some("voice_reserved_for_high_risk")
        );
    }

    #[tokio::test]
    async fn mocked_twilio_call_returns_receipt() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = [0_u8; 4096];
            let n = stream.read(&mut buf).await.unwrap();
            let req = String::from_utf8_lossy(&buf[..n]);
            assert!(req.contains("POST /Calls.json HTTP/1.1"));
            assert!(req.contains("Twiml="));
            let body = r#"{"sid":"CA123","status":"queued"}"#;
            let resp = format!(
                "HTTP/1.1 201 Created\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(resp.as_bytes()).await.unwrap();
        });
        let receipt = channel(&format!("http://{addr}/Calls.json"))
            .send(notification(RiskLevel::High))
            .await
            .unwrap();
        assert_eq!(receipt.status, DeliveryStatus::Sent);
        assert_eq!(receipt.provider_message_id.as_deref(), Some("CA123"));
        server.await.unwrap();
    }
}
