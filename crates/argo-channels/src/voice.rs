//! Twilio-backed voice approval channel.

use crate::{
    ChannelError, ChannelId, ChannelResult, DeliveryReceipt, DeliveryStatus, InboundEvent,
    Notification, NotifyChannel, RiskLevel,
};
use async_trait::async_trait;
use chrono::Utc;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::collections::BTreeMap;

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
        }
    }
}

/// Operator decision captured from Twilio DTMF digits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum VoiceDecision {
    /// Operator pressed 1 to approve.
    Approve,
    /// Operator pressed 2 to deny.
    Deny,
    /// Operator pressed 9 to request more details.
    RequestDetails,
}

impl VoiceDecision {
    /// Parse the single DTMF digit used by the voice approval flow.
    #[must_use]
    pub fn from_digits(digits: &str) -> Option<Self> {
        match digits.trim() {
            "1" => Some(Self::Approve),
            "2" => Some(Self::Deny),
            "9" => Some(Self::RequestDetails),
            _ => None,
        }
    }

    /// Return the stable decision string used in audit payloads.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Approve => "approve",
            Self::Deny => "deny",
            Self::RequestDetails => "request-details",
        }
    }
}

/// Verified HMAC token claims from a Twilio DTMF callback URL.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoiceCallbackClaims {
    /// Approval id the callback may settle.
    pub approval_id: String,
    /// Worker action id included for audit/debug correlation.
    pub action_id: String,
    /// Operator id targeted by the outbound call.
    pub operator_id: String,
    /// Unix expiry timestamp.
    pub exp: i64,
    /// Per-call nonce to make callback URLs unique.
    pub nonce: String,
}

/// Sign a DTMF callback token for the voice channel.
pub fn sign_voice_callback_token(
    approval_id: &str,
    action_id: &str,
    operator_id: &str,
    exp: i64,
    nonce: &str,
    secret: &str,
) -> ChannelResult<String> {
    if secret.is_empty() {
        return Err(ChannelError::Config(
            "voice callback secret must not be empty".into(),
        ));
    }
    for (name, value) in [
        ("approval_id", approval_id),
        ("action_id", action_id),
        ("operator_id", operator_id),
        ("nonce", nonce),
    ] {
        if value.is_empty() || value.contains('|') {
            return Err(ChannelError::Config(format!(
                "voice callback {name} must be non-empty and not contain |"
            )));
        }
    }
    let payload = format!("{approval_id}|{action_id}|{operator_id}|{exp}|{nonce}");
    Ok(format!(
        "{}.{}",
        hex::encode(payload.as_bytes()),
        hmac_sha256_hex(payload.as_bytes(), secret.as_bytes())?
    ))
}

/// Verify a signed DTMF callback token and reject expired callbacks.
pub fn verify_voice_callback_token(
    token: &str,
    secret: &str,
) -> ChannelResult<VoiceCallbackClaims> {
    if secret.is_empty() {
        return Err(ChannelError::Auth(
            "voice callback secret must not be empty".into(),
        ));
    }
    let (payload_hex, sig) = token.rsplit_once('.').ok_or_else(|| {
        ChannelError::Auth("voice callback token must have payload.signature".into())
    })?;
    let payload_bytes = hex::decode(payload_hex)
        .map_err(|e| ChannelError::Auth(format!("voice callback token payload hex: {e}")))?;
    let payload = std::str::from_utf8(&payload_bytes)
        .map_err(|e| ChannelError::Auth(format!("voice callback token payload utf8: {e}")))?;
    let expected = hmac_sha256_hex(payload.as_bytes(), secret.as_bytes())?;
    if !constant_time_eq(expected.as_bytes(), sig.as_bytes()) {
        return Err(ChannelError::Auth(
            "voice callback token signature mismatch".into(),
        ));
    }
    let parts: Vec<&str> = payload.split('|').collect();
    if parts.len() != 5 {
        return Err(ChannelError::Auth(
            "voice callback token payload must have 5 fields".into(),
        ));
    }
    let exp = parts[3]
        .parse::<i64>()
        .map_err(|e| ChannelError::Auth(format!("voice callback token exp: {e}")))?;
    if Utc::now().timestamp() > exp {
        return Err(ChannelError::Auth("voice callback token expired".into()));
    }
    Ok(VoiceCallbackClaims {
        approval_id: parts[0].to_string(),
        action_id: parts[1].to_string(),
        operator_id: parts[2].to_string(),
        exp,
        nonce: parts[4].to_string(),
    })
}

/// Render TwiML that speaks the action description and gathers one DTMF key.
#[must_use]
pub fn render_voice_twiml(action_description: &str, callback_url: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
<Response><Gather input=\"dtmf\" numDigits=\"1\" timeout=\"10\" action=\"{}\" method=\"POST\">\
<Say voice=\"alice\">{} Press 1 to approve. Press 2 to deny. Press 9 for more details.</Say>\
</Gather><Say>No input received. Goodbye.</Say></Response>",
        xml_escape(callback_url),
        xml_escape(action_description)
    )
}

/// Voice approval channel backed by Twilio.
pub struct VoiceApprovalChannel {
    config: VoiceApprovalChannelConfig,
    client: reqwest::Client,
}

impl VoiceApprovalChannel {
    /// Create a voice approval channel.
    #[must_use]
    pub fn new(config: VoiceApprovalChannelConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
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

fn hmac_sha256_hex(data: &[u8], secret: &[u8]) -> ChannelResult<String> {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret)
        .map_err(|e| ChannelError::Auth(format!("voice callback hmac init: {e}")))?;
    mac.update(data);
    Ok(hex::encode(mac.finalize().into_bytes()))
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0_u8, |diff, (x, y)| diff | (x ^ y)) == 0
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
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

    #[test]
    fn dtmf_digits_map_to_decisions() {
        assert_eq!(
            VoiceDecision::from_digits("1"),
            Some(VoiceDecision::Approve)
        );
        assert_eq!(VoiceDecision::from_digits("2"), Some(VoiceDecision::Deny));
        assert_eq!(
            VoiceDecision::from_digits("9"),
            Some(VoiceDecision::RequestDetails)
        );
        assert_eq!(VoiceDecision::from_digits("3"), None);
    }

    #[test]
    fn twiml_say_template_renders_action_description() {
        let twiml = render_voice_twiml("Approve <refund>?", "https://gw/twilio?vt=abc&x=1");
        assert!(twiml.contains("<Gather"));
        assert!(twiml.contains("<Say voice=\"alice\">Approve &lt;refund&gt;?"));
        assert!(twiml.contains("action=\"https://gw/twilio?vt=abc&amp;x=1\""));
    }

    #[test]
    fn voice_callback_token_verifies_claims() {
        let exp = Utc::now().timestamp() + 60;
        let token = sign_voice_callback_token("appr", "act", "op", exp, "nonce", "secret").unwrap();
        let claims = verify_voice_callback_token(&token, "secret").unwrap();
        assert_eq!(claims.approval_id, "appr");
        assert_eq!(claims.action_id, "act");
        assert_eq!(claims.operator_id, "op");
    }

    #[test]
    fn expired_voice_callback_token_is_rejected() {
        let exp = Utc::now().timestamp() - 1;
        let token = sign_voice_callback_token("appr", "act", "op", exp, "nonce", "secret").unwrap();
        assert!(verify_voice_callback_token(&token, "secret").is_err());
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
