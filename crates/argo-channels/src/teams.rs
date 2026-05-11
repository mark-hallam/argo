//! Microsoft Teams approval notification channel (Graph v1.0).
//!
//! Azure-AD app-only OAuth2 (client_credentials grant) with a token
//! cache that refreshes on 401 from Graph or within
//! [`TOKEN_REFRESH_WINDOW`] of stated expiry. Outbound only —
//! inbound (Bot Framework Connector) is intentionally out of scope
//! for v1. Payload-building helpers live in
//! [`crate::teams_payload`].

use crate::quiet_hours::QuietHours;
use crate::teams_payload::{adaptive_card, body_html, token_from_url};
use crate::{
    quiet_hours_deferred_receipt, ChannelError, ChannelId, ChannelResult, DeliveryReceipt,
    DeliveryStatus, InboundEvent, Notification, NotifyChannel,
};
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

pub use crate::teams_payload::TeamsAttachment;

/// Default Graph v1.0 base URL.
pub const DEFAULT_GRAPH_BASE: &str = "https://graph.microsoft.com/v1.0";
/// Default Azure-AD login base URL.
pub const DEFAULT_LOGIN_BASE: &str = "https://login.microsoftonline.com";
/// Refresh the cached app-only token within this window of expiry.
pub const TOKEN_REFRESH_WINDOW: Duration = Duration::minutes(5);
/// AAD-user-id lookups (`/users/{email}`) are cached for this long.
pub const MENTION_CACHE_TTL: Duration = Duration::hours(1);

/// Configuration for [`TeamsApprovalChannel`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamsApprovalChannelConfig {
    /// Stable channel id.
    pub channel_id: ChannelId,
    /// Azure-AD tenant id.
    pub tenant_id: String,
    /// Azure-AD app client id.
    pub client_id: String,
    /// App client secret (env-loaded — never persisted in TOML).
    pub client_secret: String,
    /// Default team id used when send carries no explicit team.
    pub default_team_id: Option<String>,
    /// Default channel id used when send carries no explicit channel.
    pub default_channel_id: Option<String>,
    /// Microsoft Graph base URL.
    pub graph_base: String,
    /// Azure-AD login base URL.
    pub login_base: String,
    /// When true, render body as HTML with minimal inline markdown.
    pub markdown_to_html: bool,
    /// Operator quiet-hours window, if configured.
    #[serde(default)]
    pub quiet_hours: Option<QuietHours>,
}

impl TeamsApprovalChannelConfig {
    /// Build from the `[channels.teams]` TOML block, loading the
    /// client secret indirectly from the configured env var.
    ///
    /// # Errors
    /// Returns [`ChannelError::Config`] when the configured env var
    /// is missing or empty.
    pub fn from_toml(cfg: &argo_config::TeamsTomlConfig) -> ChannelResult<Self> {
        let client_secret = argo_config::cred::get(&cfg.client_secret_env).map_err(|_| {
            ChannelError::Config(format!("missing env {}", cfg.client_secret_env))
        })?;
        if client_secret.trim().is_empty() {
            return Err(ChannelError::Config(format!(
                "{} must not be empty",
                cfg.client_secret_env
            )));
        }
        Ok(Self {
            channel_id: ChannelId::new("teams"),
            tenant_id: cfg.tenant_id.clone(),
            client_id: cfg.client_id.clone(),
            client_secret,
            default_team_id: cfg.default_team_id.clone(),
            default_channel_id: cfg.default_channel_id.clone(),
            graph_base: DEFAULT_GRAPH_BASE.into(),
            login_base: DEFAULT_LOGIN_BASE.into(),
            markdown_to_html: cfg.markdown_to_html,
            quiet_hours: None,
        })
    }

    /// Test config pointing at a closed local port.
    #[must_use]
    pub fn disabled_for_tests() -> Self {
        Self {
            channel_id: ChannelId::new("teams"),
            tenant_id: "test-tenant".into(),
            client_id: "test-client".into(),
            client_secret: "test-secret".into(),
            default_team_id: Some("T1".into()),
            default_channel_id: Some("C1".into()),
            graph_base: "http://127.0.0.1:9".into(),
            login_base: "http://127.0.0.1:9".into(),
            markdown_to_html: true,
            quiet_hours: None,
        }
    }
}

#[derive(Debug, Clone)]
struct CachedToken {
    access_token: String,
    expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
struct CachedMention {
    aad_user_id: String,
    cached_at: DateTime<Utc>,
}

/// Microsoft Teams approval channel.
pub struct TeamsApprovalChannel {
    config: TeamsApprovalChannelConfig,
    client: reqwest::Client,
    token: RwLock<Option<CachedToken>>,
    mention_cache: RwLock<HashMap<String, CachedMention>>,
}

impl TeamsApprovalChannel {
    /// Construct a Teams channel.
    #[must_use]
    pub fn new(config: TeamsApprovalChannelConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
            token: RwLock::new(None),
            mention_cache: RwLock::new(HashMap::new()),
        }
    }

    /// Seed the token cache (test-only helper for the proactive-
    /// refresh path).
    #[doc(hidden)]
    pub fn seed_token(&self, access_token: impl Into<String>, expires_at: DateTime<Utc>) {
        *self.token.write() = Some(CachedToken {
            access_token: access_token.into(),
            expires_at,
        });
    }

    /// Build the Graph channel-message JSON (body + attachments).
    ///
    /// # Errors
    /// Returns [`ChannelError::Config`] when either approval URL is
    /// missing its signed token query.
    pub fn message_payload(
        &self,
        n: &Notification,
        attachments: &[TeamsAttachment],
    ) -> ChannelResult<Value> {
        let approve = token_from_url(&n.approve_url)?;
        let reject = token_from_url(&n.reject_url)?;
        let text = format!(
            "**Approval needed**\n\n{}\n\nRisk: `{}` · Action: `{}`\n\nApprove: `{}` · Reject: `{}`\n\nExpires: {}",
            n.prompt_body,
            n.risk_level,
            n.action_id,
            approve,
            reject,
            n.expiry.to_rfc3339(),
        );
        let atts: Vec<Value> = attachments
            .iter()
            .enumerate()
            .map(|(i, a)| adaptive_card(i, a))
            .collect();
        Ok(json!({
            "body": {
                "contentType": if self.config.markdown_to_html { "html" } else { "text" },
                "content": body_html(&text, self.config.markdown_to_html),
            },
            "attachments": atts,
        }))
    }

    /// Send to explicit `(team, channel)`, falling back to defaults.
    ///
    /// # Errors
    /// Returns [`ChannelError::Config`] when neither an explicit nor
    /// a default value is available for team or channel; otherwise
    /// surfaces provider errors from Graph.
    pub async fn send_to_target(
        &self,
        notification: &Notification,
        team_id: Option<&str>,
        channel_id: Option<&str>,
        attachments: &[TeamsAttachment],
    ) -> ChannelResult<DeliveryReceipt> {
        let team = team_id
            .or(self.config.default_team_id.as_deref())
            .ok_or_else(|| ChannelError::Config("no team id (explicit or default)".into()))?;
        let chan = channel_id
            .or(self.config.default_channel_id.as_deref())
            .ok_or_else(|| ChannelError::Config("no channel id (explicit or default)".into()))?;
        let url = format!(
            "{}/teams/{team}/channels/{chan}/messages",
            self.config.graph_base.trim_end_matches('/')
        );
        let payload = self.message_payload(notification, attachments)?;
        self.post_with_retry(&url, &payload, &notification.approval_id)
            .await
    }

    /// Reply to a thread via `/messages/{thread_id}/replies`.
    ///
    /// # Errors
    /// Surfaces provider errors from Graph.
    pub async fn reply_to_thread(
        &self,
        notification: &Notification,
        team_id: &str,
        channel_id: &str,
        thread_id: &str,
        attachments: &[TeamsAttachment],
    ) -> ChannelResult<DeliveryReceipt> {
        let url = format!(
            "{}/teams/{team_id}/channels/{channel_id}/messages/{thread_id}/replies",
            self.config.graph_base.trim_end_matches('/')
        );
        let payload = self.message_payload(notification, attachments)?;
        self.post_with_retry(&url, &payload, &notification.approval_id)
            .await
    }

    /// Resolve `email` to its AAD user id via Graph `/users/{email}`.
    /// Cached for [`MENTION_CACHE_TTL`].
    ///
    /// # Errors
    /// Surfaces provider/parse errors from Graph.
    pub async fn resolve_mention(&self, email: &str) -> ChannelResult<String> {
        if let Some(entry) = self.mention_cache.read().get(email).cloned() {
            if Utc::now() - entry.cached_at < MENTION_CACHE_TTL {
                return Ok(entry.aad_user_id);
            }
        }
        let token = self.acquire_token().await?;
        let url = format!(
            "{}/users/{email}",
            self.config.graph_base.trim_end_matches('/')
        );
        let resp = self
            .client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| ChannelError::Provider(format!("teams users: {e}")))?;
        let status = resp.status();
        let body = resp
            .text()
            .await
            .map_err(|e| ChannelError::Provider(format!("teams users body: {e}")))?;
        if !status.is_success() {
            return Err(ChannelError::Provider(format!(
                "teams users {status}: {body}"
            )));
        }
        let parsed: Value = serde_json::from_str(&body)
            .map_err(|e| ChannelError::Parse(format!("teams users JSON: {e}")))?;
        let aad_user_id = parsed
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| ChannelError::Parse("teams users missing id".into()))?
            .to_string();
        self.mention_cache.write().insert(
            email.to_string(),
            CachedMention {
                aad_user_id: aad_user_id.clone(),
                cached_at: Utc::now(),
            },
        );
        Ok(aad_user_id)
    }

    async fn acquire_token(&self) -> ChannelResult<String> {
        if let Some(tok) = self.token.read().clone() {
            if tok.expires_at - Utc::now() > TOKEN_REFRESH_WINDOW {
                return Ok(tok.access_token);
            }
        }
        self.refresh_token().await
    }

    async fn refresh_token(&self) -> ChannelResult<String> {
        let url = format!(
            "{}/{}/oauth2/v2.0/token",
            self.config.login_base.trim_end_matches('/'),
            self.config.tenant_id,
        );
        let form = [
            ("grant_type", "client_credentials"),
            ("client_id", self.config.client_id.as_str()),
            ("client_secret", self.config.client_secret.as_str()),
            ("scope", "https://graph.microsoft.com/.default"),
        ];
        let resp = self
            .client
            .post(&url)
            .form(&form)
            .send()
            .await
            .map_err(|e| ChannelError::Provider(format!("teams token: {e}")))?;
        let status = resp.status();
        let body = resp
            .text()
            .await
            .map_err(|e| ChannelError::Provider(format!("teams token body: {e}")))?;
        if !status.is_success() {
            return Err(ChannelError::Auth(format!("teams token {status}: {body}")));
        }
        let parsed: Value = serde_json::from_str(&body)
            .map_err(|e| ChannelError::Parse(format!("teams token JSON: {e}")))?;
        let access_token = parsed
            .get("access_token")
            .and_then(Value::as_str)
            .ok_or_else(|| ChannelError::Parse("teams token missing access_token".into()))?
            .to_string();
        let expires_in = parsed
            .get("expires_in")
            .and_then(Value::as_i64)
            .unwrap_or(3600);
        *self.token.write() = Some(CachedToken {
            access_token: access_token.clone(),
            expires_at: Utc::now() + Duration::seconds(expires_in),
        });
        Ok(access_token)
    }

    async fn post_with_retry(
        &self,
        url: &str,
        payload: &Value,
        approval_id: &str,
    ) -> ChannelResult<DeliveryReceipt> {
        for attempt in 0..2 {
            let token = self.acquire_token().await?;
            let resp = self
                .client
                .post(url)
                .bearer_auth(&token)
                .header("Content-Type", "application/json")
                .json(payload)
                .send()
                .await
                .map_err(|e| ChannelError::Provider(format!("teams send: {e}")))?;
            let status = resp.status();
            if status.as_u16() == 401 && attempt == 0 {
                *self.token.write() = None;
                continue;
            }
            let body = resp
                .text()
                .await
                .map_err(|e| ChannelError::Provider(format!("teams response: {e}")))?;
            if !status.is_success() {
                return Err(ChannelError::Provider(format!(
                    "teams returned {status}: {body}"
                )));
            }
            let provider_message_id = serde_json::from_str::<Value>(&body)
                .ok()
                .and_then(|v| v.get("id").and_then(Value::as_str).map(str::to_string));
            return Ok(DeliveryReceipt {
                channel_id: self.id(),
                notification_id: approval_id.to_string(),
                provider_message_id,
                status: DeliveryStatus::Sent,
                deferred_reason: None,
                delivered_at: Utc::now(),
            });
        }
        Err(ChannelError::Provider(
            "teams returned 401 after one retry".into(),
        ))
    }
}

#[async_trait]
impl NotifyChannel for TeamsApprovalChannel {
    fn id(&self) -> ChannelId {
        self.config.channel_id.clone()
    }

    async fn send(&self, notification: Notification) -> ChannelResult<DeliveryReceipt> {
        if let Some(quiet) = self.config.quiet_hours.as_ref() {
            if quiet.contains(Utc::now()) {
                tracing::info!(
                    channel = %self.config.channel_id,
                    approval_id = %notification.approval_id,
                    "deferred-send: quiet_hours",
                );
                return Ok(quiet_hours_deferred_receipt(
                    self.id(),
                    notification.approval_id,
                ));
            }
        }
        if self.config.client_secret.trim().is_empty() {
            return Err(ChannelError::Config(
                "Teams client secret must not be empty".into(),
            ));
        }
        self.send_to_target(&notification, None, None, &[]).await
    }

    async fn poll(&self) -> ChannelResult<Vec<InboundEvent>> {
        Ok(Vec::new())
    }

    fn quiet_hours(&self) -> Option<&QuietHours> {
        self.config.quiet_hours.as_ref()
    }
}

#[cfg(test)]
#[path = "teams_tests.rs"]
mod teams_tests;
