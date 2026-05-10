//! Blue Bubbles-backed iMessage approval notification channel.
//!
//! ```no_run
//! # async fn demo() -> argo_channels::ChannelResult<()> {
//! use argo_channels::imessage::{IMessageChannel, IMessageChannelConfig};
//! use argo_channels::NotifyChannel;
//!
//! let channel = IMessageChannel::new(IMessageChannelConfig::from_env_default()?)?;
//! let _ = channel.poll().await?;
//! # Ok(()) }
//! ```

use crate::quiet_hours::QuietHours;
use crate::rate_limit::{RateLimit, RateLimitConfig};
use crate::token::verify_approval_token;
use crate::{
    quiet_hours_deferred_receipt, ChannelError, ChannelId, ChannelResult, DeliveryReceipt,
    DeliveryStatus, InboundEvent, Notification, NotifyChannel,
};
use argo_bluebubbles_client::{
    parse_webhook_event, validate_webhook_signature, BlueBubblesClient, SendMessageRequest,
};
use async_trait::async_trait;
use chrono::Utc;
use std::collections::BTreeMap;

/// Configuration for [`IMessageChannel`].
#[derive(Debug, Clone)]
pub struct IMessageChannelConfig {
    /// Channel id.
    pub channel_id: ChannelId,
    /// Blue Bubbles server URL.
    pub server_url: String,
    /// Blue Bubbles server password.
    pub password: String,
    /// Approval token signing secret.
    pub approval_secret: String,
    /// Optional webhook HMAC secret. Defaults to `password` when omitted.
    pub webhook_secret: Option<String>,
    /// Per-operator Blue Bubbles chat GUID routes.
    pub operator_routes: BTreeMap<String, String>,
    /// Operator override for the per-channel rate-limit budget.
    /// Missing fields fall back to the iMessage default
    /// (5/sec, 60/min).
    pub rate_limit: RateLimitConfig,
    /// Operator quiet hours. `None` means quiet hours are not configured
    /// for this channel — today's behavior preserved.
    pub quiet_hours: Option<QuietHours>,
}

impl IMessageChannelConfig {
    /// Build config from `BLUE_BUBBLES_URL`, `BLUE_BUBBLES_PASSWORD`,
    /// and `ARGO_CHANNEL_HMAC_SECRET`.
    pub fn from_env_default() -> ChannelResult<Self> {
        Ok(Self {
            channel_id: ChannelId::new("imessage"),
            server_url: required_env("BLUE_BUBBLES_URL")?,
            password: required_env("BLUE_BUBBLES_PASSWORD")?,
            approval_secret: required_env("ARGO_CHANNEL_HMAC_SECRET")?,
            webhook_secret: argo_config::cred::get("BLUE_BUBBLES_WEBHOOK_SECRET").ok(),
            operator_routes: BTreeMap::new(),
            rate_limit: RateLimitConfig::default(),
            quiet_hours: None,
        })
    }

    /// Build local test config.
    #[must_use]
    pub fn local(server_url: impl Into<String>) -> Self {
        Self {
            channel_id: ChannelId::new("imessage"),
            server_url: server_url.into(),
            password: "password".into(),
            approval_secret: "secret".into(),
            webhook_secret: Some("webhook-secret".into()),
            operator_routes: BTreeMap::new(),
            rate_limit: RateLimitConfig::unlimited(),
            quiet_hours: None,
        }
    }
}

/// iMessage approval channel backed by Blue Bubbles.
pub struct IMessageChannel {
    config: IMessageChannelConfig,
    client: BlueBubblesClient,
    rate_limit: RateLimit,
}

impl IMessageChannel {
    /// Create an iMessage channel from config.
    pub fn new(config: IMessageChannelConfig) -> ChannelResult<Self> {
        let client = BlueBubblesClient::new(&config.server_url, &config.password)
            .map_err(|e| ChannelError::Config(e.to_string()))?;
        let rate_limit = RateLimit::from_config(
            RateLimitConfig::imessage_default().with_override(config.rate_limit),
        );
        Ok(Self {
            config,
            client,
            rate_limit,
        })
    }

    /// Parse an inbound Blue Bubbles webhook into an approval event.
    pub fn parse_webhook(
        &self,
        body: &[u8],
        signature: Option<&str>,
    ) -> ChannelResult<InboundEvent> {
        if let Some(signature) = signature {
            let secret = self
                .config
                .webhook_secret
                .as_deref()
                .unwrap_or(&self.config.password);
            validate_webhook_signature(body, signature, secret)
                .map_err(|e| ChannelError::Auth(e.to_string()))?;
        }
        let event = parse_webhook_event(body).map_err(|e| ChannelError::Parse(e.to_string()))?;
        let token = extract_approval_token(&event.text)
            .ok_or_else(|| ChannelError::Parse("iMessage reply missing approval token".into()))?;
        let claims = verify_approval_token(&token, &self.config.approval_secret)?;
        Ok(InboundEvent {
            channel_id: self.config.channel_id.clone(),
            approval_id: claims.approval_id,
            decision: claims.decision,
            actor: event.sender.unwrap_or_else(|| "imessage-operator".into()),
            note: Some(event.text),
            received_at: Utc::now(),
            raw: event.raw,
        })
    }

    fn chat_guid_for(&self, notification: &Notification) -> ChannelResult<String> {
        self.config
            .operator_routes
            .get(&notification.target.operator_id)
            .cloned()
            .or_else(|| nonempty(&notification.target.email))
            .ok_or_else(|| {
                ChannelError::Config(format!(
                    "no iMessage chat route for {}",
                    notification.target.operator_id
                ))
            })
    }
}

#[async_trait]
impl NotifyChannel for IMessageChannel {
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
        let chat_guid = self.chat_guid_for(&notification)?;
        let message = render_approval_message(&notification);
        let sent = self
            .client
            .send_message(SendMessageRequest {
                chat_guid,
                message,
                temp_guid: Some(notification.approval_id.clone()),
            })
            .await
            .map_err(|e| ChannelError::Provider(e.to_string()))?;
        Ok(DeliveryReceipt {
            channel_id: self.config.channel_id.clone(),
            notification_id: notification.approval_id,
            provider_message_id: provider_message_id(&sent),
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

fn render_approval_message(n: &Notification) -> String {
    format!(
        "{} risk approval requested for action {}\n\n{}\n\nApprove: {}\nReject: {}\n\napproval_id: {}\nexpires: {}",
        n.risk_level, n.action_id, n.prompt_body, n.approve_url, n.reject_url, n.approval_id, n.expiry
    )
}

fn provider_message_id(value: &serde_json::Value) -> Option<String> {
    value
        .pointer("/data/guid")
        .or_else(|| value.pointer("/data/message/guid"))
        .or_else(|| value.get("guid"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
}

fn extract_approval_token(text: &str) -> Option<String> {
    for marker in ["approval_token=", "token="] {
        if let Some((_, rest)) = text.split_once(marker) {
            return Some(clean_token(rest));
        }
    }
    let lower = text.trim().to_ascii_lowercase();
    if lower.starts_with("yes ") || lower.starts_with("approve ") {
        return text.split_whitespace().nth(1).map(str::to_owned);
    }
    if lower.starts_with("no ") || lower.starts_with("reject ") {
        return text.split_whitespace().nth(1).map(str::to_owned);
    }
    None
}

fn clean_token(raw: &str) -> String {
    raw.split_whitespace()
        .next()
        .unwrap_or(raw)
        .trim_matches(|c: char| c == '"' || c == '\'' || c == ',' || c == ')' || c == ']')
        .to_string()
}

fn nonempty(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_owned())
    }
}

fn required_env(name: &str) -> ChannelResult<String> {
    argo_config::cred::get(name)
        .ok()
        .and_then(|v| nonempty(&v))
        .ok_or_else(|| ChannelError::Config(format!("missing env {name}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token::{sign_approval_token, ApprovalDecision};
    use chrono::Duration;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn send_posts_imessage_to_fake_bluebubbles() {
        let (url, handle) = fake_bluebubbles().await;
        let channel = IMessageChannel::new(IMessageChannelConfig::local(url)).unwrap();
        let receipt = channel.send(fake_notification("act-1")).await.unwrap();
        assert_eq!(receipt.provider_message_id.as_deref(), Some("msg-1"));
        let request = handle.await.unwrap();
        assert!(request.contains("/api/v1/message/text?password=password"));
        assert!(request.contains("Approve"));
    }

    #[test]
    fn webhook_reply_becomes_inbound_event() {
        let channel =
            IMessageChannel::new(IMessageChannelConfig::local("http://127.0.0.1:1")).unwrap();
        let exp = Utc::now().timestamp() + 60;
        let token = sign_approval_token("act-2", ApprovalDecision::Approve, exp, "secret").unwrap();
        let body = serde_json::json!({
            "message": {"text": format!("YES approval_token={token}"), "handle": "+1555"}
        })
        .to_string();
        let sig =
            argo_bluebubbles_client::sign_webhook_body(body.as_bytes(), "webhook-secret").unwrap();
        let event = channel
            .parse_webhook(body.as_bytes(), Some(&sig))
            .expect("event");
        assert_eq!(event.approval_id, "act-2");
        assert_eq!(event.actor, "+1555");
    }

    #[test]
    fn token_extraction_accepts_key_value_form() {
        assert_eq!(
            extract_approval_token("ok\napproval_token=act.approve.123.sig, thanks").as_deref(),
            Some("act.approve.123.sig")
        );
    }

    async fn fake_bluebubbles() -> (String, tokio::task::JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = vec![0_u8; 4096];
            let n = stream.read(&mut buf).await.unwrap();
            let request = String::from_utf8_lossy(&buf[..n]).to_string();
            let body = r#"{"data":{"guid":"msg-1"}}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).await.unwrap();
            request
        });
        (format!("http://{addr}"), handle)
    }

    fn fake_notification(id: &str) -> Notification {
        Notification {
            target: crate::OperatorTarget {
                operator_id: "op-1".into(),
                email: "iMessage;+;+15551234567".into(),
            },
            action_id: id.into(),
            approval_id: id.into(),
            risk_level: crate::RiskLevel::High,
            prompt_body: "Send the invoice reminder?".into(),
            approve_url: "https://example.test/approve?token=a".into(),
            reject_url: "https://example.test/reject?token=r".into(),
            expiry: Utc::now() + Duration::minutes(15),
        }
    }
}
