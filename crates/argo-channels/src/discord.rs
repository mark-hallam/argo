//! Discord approval notification channel.

use crate::{
    ChannelError, ChannelId, ChannelResult, DeliveryReceipt, DeliveryStatus, InboundEvent,
    Notification, NotifyChannel,
};
use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Configuration for [`DiscordChannel`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordChannelConfig {
    /// Stable Argo channel id.
    pub channel_id: ChannelId,
    /// Discord bot token from `DISCORD_BOT_TOKEN`.
    pub bot_token: String,
    /// Discord channel snowflake that receives approval prompts.
    pub discord_channel_id: String,
    /// Discord REST API base URL.
    pub endpoint_base: String,
}

impl DiscordChannelConfig {
    /// Build config using `DISCORD_BOT_TOKEN` and one Discord channel id.
    pub fn from_env(discord_channel_id: impl Into<String>) -> ChannelResult<Self> {
        let bot_token = env_required("DISCORD_BOT_TOKEN")?;
        Ok(Self {
            channel_id: ChannelId::new("discord"),
            bot_token,
            discord_channel_id: discord_channel_id.into(),
            endpoint_base: "https://discord.com/api/v10".into(),
        })
    }

    /// Test config that points at a closed local port.
    #[must_use]
    pub fn disabled_for_tests(discord_channel_id: impl Into<String>) -> Self {
        Self {
            channel_id: ChannelId::new("discord"),
            bot_token: "test-token".into(),
            discord_channel_id: discord_channel_id.into(),
            endpoint_base: "http://127.0.0.1:9".into(),
        }
    }
}

/// Discord approval channel backed by `POST /channels/{id}/messages`.
pub struct DiscordChannel {
    config: DiscordChannelConfig,
    client: reqwest::Client,
}

impl DiscordChannel {
    /// Create a Discord approval channel from config.
    #[must_use]
    pub fn new(config: DiscordChannelConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    /// Build the Discord message payload with two approval buttons.
    pub fn message_payload(&self, n: &Notification) -> ChannelResult<Value> {
        let approve_token = token_from_url(&n.approve_url)?;
        let reject_token = token_from_url(&n.reject_url)?;
        Ok(json!({
            "content": format!("Approval needed for {}", n.action_id),
            "embeds": [{
                "title": "Approval needed",
                "description": n.prompt_body,
                "fields": [
                    {"name": "Risk", "value": format!("`{}`", n.risk_level), "inline": true},
                    {"name": "Action", "value": format!("`{}`", n.action_id), "inline": true},
                    {
                        "name": "Approval",
                        "value": format!("`{}` expires `{}`", n.approval_id, n.expiry),
                        "inline": false
                    }
                ]
            }],
            "components": [{
                "type": 1,
                "components": [
                    button("Approve", 3, &approve_token),
                    button("Deny", 4, &reject_token)
                ]
            }]
        }))
    }

    fn message_endpoint(&self) -> String {
        format!(
            "{}/channels/{}/messages",
            self.config.endpoint_base.trim_end_matches('/'),
            self.config.discord_channel_id
        )
    }
}

#[async_trait]
impl NotifyChannel for DiscordChannel {
    fn id(&self) -> ChannelId {
        self.config.channel_id.clone()
    }

    async fn send(&self, notification: Notification) -> ChannelResult<DeliveryReceipt> {
        if self.config.bot_token.trim().is_empty() {
            return Err(ChannelError::Config(
                "DISCORD_BOT_TOKEN must not be empty".into(),
            ));
        }
        let payload = self.message_payload(&notification)?;
        let resp = self
            .client
            .post(self.message_endpoint())
            .header("Authorization", format!("Bot {}", self.config.bot_token))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| ChannelError::Provider(format!("discord send: {e}")))?;
        let status = resp.status();
        let body = resp
            .text()
            .await
            .map_err(|e| ChannelError::Provider(format!("discord response: {e}")))?;
        if !status.is_success() {
            return Err(ChannelError::Provider(format!(
                "discord returned {status}: {body}"
            )));
        }
        let provider_message_id = serde_json::from_str::<Value>(&body)
            .ok()
            .and_then(|v| v.get("id").and_then(Value::as_str).map(str::to_string));
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

fn button(label: &str, style: u8, token: &str) -> Value {
    json!({
        "type": 2,
        "style": style,
        "label": label,
        "custom_id": format!("argo_approval:{token}")
    })
}

fn token_from_url(url: &str) -> ChannelResult<String> {
    url.split_once('?')
        .and_then(|(_, q)| q.split('&').find_map(|kv| kv.strip_prefix("token=")))
        .map(str::to_string)
        .ok_or_else(|| ChannelError::Config("Discord approval URL missing token".into()))
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

    fn notification() -> Notification {
        Notification {
            target: OperatorTarget {
                operator_id: "op-1".into(),
                email: "operator@example.com".into(),
            },
            action_id: "act-1".into(),
            approval_id: "appr-1".into(),
            risk_level: RiskLevel::High,
            prompt_body: "Approve the Discord post?".into(),
            approve_url: "https://gw.example.com/decision?token=appr-1.approve.999.sig".into(),
            reject_url: "https://gw.example.com/decision?token=appr-1.reject.999.sig".into(),
            expiry: Utc::now() + Duration::minutes(5),
        }
    }

    fn channel(endpoint_base: &str) -> DiscordChannel {
        let mut cfg = DiscordChannelConfig::disabled_for_tests("123456");
        cfg.endpoint_base = endpoint_base.into();
        DiscordChannel::new(cfg)
    }

    #[test]
    fn payload_carries_signed_tokens_in_buttons() {
        let payload = channel("http://127.0.0.1:9")
            .message_payload(&notification())
            .unwrap();
        let buttons = payload["components"][0]["components"].as_array().unwrap();
        assert_eq!(
            buttons[0]["custom_id"],
            "argo_approval:appr-1.approve.999.sig"
        );
        assert_eq!(
            buttons[1]["custom_id"],
            "argo_approval:appr-1.reject.999.sig"
        );
    }

    #[test]
    fn token_from_url_requires_token_query() {
        let err = token_from_url("https://gw.example.com/decision").unwrap_err();
        assert!(err.to_string().contains("missing token"));
    }

    #[tokio::test]
    async fn mocked_discord_message_returns_receipt() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = [0_u8; 4096];
            let n = stream.read(&mut buf).await.unwrap();
            let req = String::from_utf8_lossy(&buf[..n]);
            assert!(req.contains("POST /channels/123456/messages HTTP/1.1"));
            assert!(req
                .to_ascii_lowercase()
                .contains("authorization: bot test-token"));
            assert!(req.contains("argo_approval:appr-1.approve.999.sig"));
            let body = r#"{"id":"msg-1"}"#;
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(resp.as_bytes()).await.unwrap();
        });
        let receipt = channel(&format!("http://{addr}"))
            .send(notification())
            .await
            .unwrap();
        assert_eq!(receipt.status, DeliveryStatus::Sent);
        assert_eq!(receipt.provider_message_id.as_deref(), Some("msg-1"));
        server.await.unwrap();
    }
}
