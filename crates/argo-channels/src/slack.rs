//! Slack Block Kit approval notification channel.

use crate::quiet_hours::QuietHours;
use crate::rate_limit::{RateLimit, RateLimitConfig};
use crate::{
    quiet_hours_deferred_receipt, ChannelError, ChannelId, ChannelResult, DeliveryReceipt,
    DeliveryStatus, InboundEvent, Notification, NotifyChannel,
};
use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;

/// Per-operator Slack delivery target.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SlackOperatorRoute {
    /// Slack workspace/team id for audit and config clarity.
    pub workspace_id: String,
    /// Slack channel id to post into, when channel delivery is used.
    pub channel_id: Option<String>,
    /// Slack user or DM channel id to target when direct delivery is used.
    pub user_id: Option<String>,
}

impl SlackOperatorRoute {
    fn destination(&self) -> ChannelResult<&str> {
        self.channel_id
            .as_deref()
            .or(self.user_id.as_deref())
            .ok_or_else(|| ChannelError::Config("missing Slack channel_id or user_id".into()))
    }
}

/// Configuration for [`SlackApprovalChannel`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackApprovalChannelConfig {
    /// Channel id.
    pub channel_id: ChannelId,
    /// Slack bot token from `SLACK_APPROVALS_BOT_TOKEN`.
    pub bot_token: String,
    /// Slack `chat.postMessage` endpoint.
    pub endpoint: String,
    /// Per-operator routing preferences.
    pub operator_routes: BTreeMap<String, SlackOperatorRoute>,
    /// Operator override for the per-channel rate-limit budget.
    /// Missing fields fall back to the Slack default
    /// (5/sec, 60/min).
    #[serde(default)]
    pub rate_limit: RateLimitConfig,
    /// Operator quiet hours. `None` means quiet hours are not configured
    /// for this channel — today's behavior preserved.
    #[serde(default)]
    pub quiet_hours: Option<QuietHours>,
}

impl SlackApprovalChannelConfig {
    /// Build config using `SLACK_APPROVALS_BOT_TOKEN` and one default route.
    pub fn from_env(default_operator_id: &str, route: SlackOperatorRoute) -> ChannelResult<Self> {
        let bot_token = argo_config::cred::get("SLACK_APPROVALS_BOT_TOKEN")
            .map_err(|_| ChannelError::Config("missing SLACK_APPROVALS_BOT_TOKEN".into()))?;
        let mut operator_routes = BTreeMap::new();
        operator_routes.insert(default_operator_id.to_string(), route);
        Ok(Self {
            channel_id: ChannelId::new("slack"),
            bot_token,
            endpoint: "https://slack.com/api/chat.postMessage".into(),
            operator_routes,
            rate_limit: RateLimitConfig::default(),
            quiet_hours: None,
        })
    }
}

/// Slack approval channel backed by `chat.postMessage`.
pub struct SlackApprovalChannel {
    config: SlackApprovalChannelConfig,
    client: reqwest::Client,
    rate_limit: RateLimit,
}

impl SlackApprovalChannel {
    /// Create a Slack approval channel from config.
    #[must_use]
    pub fn new(config: SlackApprovalChannelConfig) -> Self {
        let rate_limit = RateLimit::from_config(
            RateLimitConfig::slack_default().with_override(config.rate_limit),
        );
        Self {
            config,
            client: reqwest::Client::new(),
            rate_limit,
        }
    }

    /// Build the Slack `chat.postMessage` JSON payload.
    pub fn message_payload(&self, n: &Notification) -> ChannelResult<Value> {
        let route = self.route_for(&n.target.operator_id)?;
        let approve_token = token_from_url(&n.approve_url)?;
        let reject_token = token_from_url(&n.reject_url)?;
        Ok(json!({
            "channel": route.destination()?,
            "text": format!("Approval needed for {}", n.action_id),
            "blocks": [
                {
                    "type": "section",
                    "text": {
                        "type": "mrkdwn",
                        "text": format!(
                            "*Approval needed*\n{}\n*Risk:* `{}`\n*Action:* `{}`",
                            n.prompt_body, n.risk_level, n.action_id
                        )
                    }
                },
                {
                    "type": "context",
                    "elements": [{
                        "type": "mrkdwn",
                        "text": format!("approval_id `{}` · expires `{}`", n.approval_id, n.expiry)
                    }]
                },
                {
                    "type": "actions",
                    "elements": [
                        button("approve", "Approve", "primary", &approve_token),
                        button("reject", "Reject", "danger", &reject_token)
                    ]
                }
            ]
        }))
    }

    fn route_for(&self, operator_id: &str) -> ChannelResult<&SlackOperatorRoute> {
        self.config
            .operator_routes
            .get(operator_id)
            .or_else(|| self.config.operator_routes.get("default"))
            .ok_or_else(|| ChannelError::Config(format!("no Slack route for {operator_id}")))
    }
}

#[async_trait]
impl NotifyChannel for SlackApprovalChannel {
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
        if self.config.bot_token.is_empty() {
            return Err(ChannelError::Config(
                "SLACK_APPROVALS_BOT_TOKEN must not be empty".into(),
            ));
        }
        let payload = self.message_payload(&notification)?;
        let resp = self
            .client
            .post(&self.config.endpoint)
            .bearer_auth(&self.config.bot_token)
            .header("Content-Type", "application/json; charset=utf-8")
            .json(&payload)
            .send()
            .await
            .map_err(|e| ChannelError::Provider(format!("slack send: {e}")))?;
        let status = resp.status();
        let body = resp
            .text()
            .await
            .map_err(|e| ChannelError::Provider(format!("slack response: {e}")))?;
        if !status.is_success() {
            return Err(ChannelError::Provider(format!(
                "slack returned {status}: {body}"
            )));
        }
        let parsed: Value = serde_json::from_str(&body)
            .map_err(|e| ChannelError::Provider(format!("slack response JSON: {e}")))?;
        if parsed.get("ok").and_then(Value::as_bool) != Some(true) {
            return Err(ChannelError::Provider(format!("slack error: {body}")));
        }
        let provider_message_id = parsed.get("ts").and_then(Value::as_str).map(str::to_string);
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

fn button(action_id: &str, text: &str, style: &str, value: &str) -> Value {
    json!({
        "type": "button",
        "text": {"type": "plain_text", "text": text},
        "style": style,
        "action_id": action_id,
        "value": value
    })
}

fn token_from_url(url: &str) -> ChannelResult<String> {
    let query = url
        .split_once('?')
        .map(|(_, q)| q)
        .ok_or_else(|| ChannelError::Config("approval URL missing token query".into()))?;
    query
        .split('&')
        .find_map(|kv| kv.strip_prefix("token="))
        .map(str::to_string)
        .ok_or_else(|| ChannelError::Config("approval URL missing token value".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RiskLevel;
    use chrono::{Duration, Utc};

    fn channel() -> SlackApprovalChannel {
        let mut operator_routes = BTreeMap::new();
        operator_routes.insert(
            "op-1".into(),
            SlackOperatorRoute {
                workspace_id: "T123".into(),
                channel_id: Some("C123".into()),
                user_id: None,
            },
        );
        SlackApprovalChannel::new(SlackApprovalChannelConfig {
            channel_id: ChannelId::new("slack"),
            bot_token: "test".into(),
            endpoint: "http://127.0.0.1:9".into(),
            operator_routes,
            rate_limit: RateLimitConfig::unlimited(),
            quiet_hours: None,
        })
    }

    fn notification() -> Notification {
        let expiry = Utc::now() + Duration::minutes(5);
        Notification {
            target: crate::OperatorTarget {
                operator_id: "op-1".into(),
                email: "operator@example.com".into(),
            },
            action_id: "act-1".into(),
            approval_id: "act-1".into(),
            risk_level: RiskLevel::High,
            prompt_body: "Ship it?".into(),
            approve_url: format!(
                "https://gw/decision?token=act-1.approve.{}.aaa",
                expiry.timestamp()
            ),
            reject_url: format!(
                "https://gw/decision?token=act-1.reject.{}.bbb",
                expiry.timestamp()
            ),
            expiry,
        }
    }

    #[test]
    fn payload_carries_signed_tokens_in_buttons() {
        let notification = notification();
        let approve = format!("act-1.approve.{}.aaa", notification.expiry.timestamp());
        let reject = format!("act-1.reject.{}.bbb", notification.expiry.timestamp());
        let payload = channel().message_payload(&notification).expect("payload");
        let buttons = payload["blocks"][2]["elements"]
            .as_array()
            .expect("buttons");
        assert_eq!(buttons[0]["value"], approve);
        assert_eq!(buttons[1]["value"], reject);
    }

    #[tokio::test]
    async fn posts_to_slack_when_env_present() {
        let Ok(bot_token) = std::env::var("SLACK_APPROVALS_BOT_TOKEN") else {
            return;
        };
        let Ok(channel_id) = std::env::var("SLACK_APPROVALS_TEST_CHANNEL") else {
            return;
        };
        let mut ch = channel();
        ch.config.bot_token = bot_token;
        ch.config.endpoint = "https://slack.com/api/chat.postMessage".into();
        ch.config
            .operator_routes
            .get_mut("op-1")
            .expect("route")
            .channel_id = Some(channel_id);
        let payload = ch.message_payload(&notification()).expect("payload");
        assert_eq!(payload["blocks"][2]["elements"][0]["action_id"], "approve");
        let receipt = ch.send(notification()).await.expect("send");
        assert_eq!(receipt.status, DeliveryStatus::Sent);
    }
}
