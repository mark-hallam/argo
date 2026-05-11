//! Matrix Client-Server API approval notification channel.
//!
//! v1 is **cleartext-only** — Matrix's end-to-end encryption (Olm /
//! Megolm) is intentionally OUT of scope. E2E requires per-device
//! keys, a persistent session store, and device-verification UX; that
//! lift is a separate follow-up brief built around `matrix-rust-sdk`,
//! not the lightweight reqwest+serde shape here.
//!
//! Mirrors `crate::discord` in shape (long-token-auth, HTTP-out only).
//! Matrix-specific ceremony:
//! - Send is `PUT /_matrix/client/r0/rooms/<room>/send/m.room.message/<txn_id>`.
//!   Matrix's spec guarantees same-`(room, txn_id)` → same event, so
//!   a retry with the same `txn_id` is idempotent server-side.
//! - `txn_counter` is a monotonic `AtomicU64` seeded from
//!   nanos-since-epoch (different starting point per process restart,
//!   bounded probability of pre/post-restart collision).
//! - `m.thread` (MSC3440) relation when a thread root event id is set.
//! - `m.mentions` (MSC3952) carries explicit `@user:domain` ids.
//! - `body` is plain text; `formatted_body` is HTML via
//!   [`crate::teams_payload::body_html`].

use crate::teams_payload::body_html;
use crate::{
    ChannelError, ChannelId, ChannelResult, DeliveryReceipt, DeliveryStatus, InboundEvent,
    Notification, NotifyChannel,
};
use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};

/// Client-Server API base path prefix (r0.6.1).
pub const CLIENT_API_PREFIX: &str = "/_matrix/client/r0";

/// Configuration for [`MatrixChannel`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatrixChannelConfig {
    /// Stable Argo channel id.
    pub channel_id: ChannelId,
    /// Homeserver base URL (e.g. `https://matrix.org`).
    pub homeserver_url: String,
    /// Long-lived access token, env-loaded.
    pub access_token: String,
    /// Bot user id, `@local:domain` form.
    pub user_id: String,
    /// Default room id (`!opaque:domain`); explicit override wins.
    pub default_room_id: Option<String>,
}

impl MatrixChannelConfig {
    /// Build from `[channels.matrix]` plus env-loaded access token.
    ///
    /// # Errors
    /// - [`ChannelError::Config`] when the configured env var is
    ///   missing/empty, or `user_id`/`default_room_id` are malformed.
    pub fn from_toml(cfg: &argo_config::MatrixTomlConfig) -> ChannelResult<Self> {
        let access_token = argo_config::cred::get(&cfg.access_token_env).map_err(|_| {
            ChannelError::Config(format!("missing env {}", cfg.access_token_env))
        })?;
        if access_token.trim().is_empty() {
            return Err(ChannelError::Config(format!(
                "{} must not be empty",
                cfg.access_token_env
            )));
        }
        validate_user_id(&cfg.user_id)?;
        if let Some(room) = cfg.default_room_id.as_deref() {
            validate_room_id(room)?;
        }
        Ok(Self {
            channel_id: ChannelId::new("matrix"),
            homeserver_url: cfg.homeserver_url.clone(),
            access_token,
            user_id: cfg.user_id.clone(),
            default_room_id: cfg.default_room_id.clone(),
        })
    }

    /// Test config pointing at a closed local port.
    #[must_use]
    pub fn disabled_for_tests() -> Self {
        Self {
            channel_id: ChannelId::new("matrix"),
            homeserver_url: "http://127.0.0.1:9".into(),
            access_token: "test-token".into(),
            user_id: "@argo-bot:matrix.example.com".into(),
            default_room_id: Some("!default:matrix.example.com".into()),
        }
    }
}

/// Matrix approval channel.
pub struct MatrixChannel {
    config: MatrixChannelConfig,
    client: reqwest::Client,
    txn_counter: AtomicU64,
}

impl MatrixChannel {
    /// Construct a Matrix channel.
    #[must_use]
    pub fn new(config: MatrixChannelConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
            txn_counter: AtomicU64::new(initial_txn_seed()),
        }
    }

    /// Build the `m.room.message` content body.
    #[must_use]
    pub fn message_payload(
        &self,
        n: &Notification,
        thread_event_id: Option<&str>,
        mentions: &[String],
    ) -> Value {
        let footer = format!(
            "Risk: {}\nApprove: {}\nReject: {}\nExpires: {}",
            n.risk_level,
            n.approve_url,
            n.reject_url,
            n.expiry.to_rfc3339(),
        );
        let plain = format!("{}\n\n{footer}", n.prompt_body);
        let formatted = body_html(&plain, true);
        let mut body = json!({
            "msgtype": "m.text",
            "body": plain,
            "format": "org.matrix.custom.html",
            "formatted_body": formatted,
        });
        if let Some(event_id) = thread_event_id {
            body["m.relates_to"] = json!({
                "rel_type": "m.thread",
                "event_id": event_id,
            });
        }
        if !mentions.is_empty() {
            let resolved: Vec<String> = mentions.iter().map(|m| normalize_mention(m)).collect();
            body["m.mentions"] = json!({ "user_ids": resolved });
        }
        body
    }

    /// Next transaction id — atomic, monotonic across sends.
    pub fn next_txn_id(&self) -> u64 {
        self.txn_counter.fetch_add(1, Ordering::Relaxed)
    }

    /// Send to an explicit room (falls back to default when `None`).
    ///
    /// # Errors
    /// - [`ChannelError::Config`] when no room id is available or the
    ///   provided room id is malformed.
    /// - [`ChannelError::Provider`] when the homeserver returns
    ///   non-2xx or the transport fails.
    pub async fn send_to_room(
        &self,
        notification: &Notification,
        room_id: Option<&str>,
        thread_event_id: Option<&str>,
        mentions: &[String],
    ) -> ChannelResult<DeliveryReceipt> {
        let room = room_id
            .or(self.config.default_room_id.as_deref())
            .ok_or_else(|| ChannelError::Config("no room id (explicit or default)".into()))?;
        validate_room_id(room)?;
        let txn_id = self.next_txn_id();
        let payload = self.message_payload(notification, thread_event_id, mentions);
        self.put_with_txn(room, txn_id, &payload, &notification.approval_id)
            .await
    }

    /// Retry a send with a pre-generated `txn_id` (test helper that
    /// exercises Matrix's idempotency guarantee).
    ///
    /// # Errors
    /// Same as [`Self::send_to_room`].
    #[doc(hidden)]
    pub async fn send_with_txn_id(
        &self,
        notification: &Notification,
        room_id: &str,
        txn_id: u64,
    ) -> ChannelResult<DeliveryReceipt> {
        validate_room_id(room_id)?;
        let payload = self.message_payload(notification, None, &[]);
        self.put_with_txn(room_id, txn_id, &payload, &notification.approval_id)
            .await
    }

    fn send_endpoint(&self, room_id: &str, txn_id: u64) -> String {
        format!(
            "{}{CLIENT_API_PREFIX}/rooms/{room_id}/send/m.room.message/{txn_id}",
            self.config.homeserver_url.trim_end_matches('/')
        )
    }

    async fn put_with_txn(
        &self,
        room_id: &str,
        txn_id: u64,
        payload: &Value,
        approval_id: &str,
    ) -> ChannelResult<DeliveryReceipt> {
        let url = self.send_endpoint(room_id, txn_id);
        let resp = self
            .client
            .put(url)
            .bearer_auth(&self.config.access_token)
            .header("Content-Type", "application/json")
            .json(payload)
            .send()
            .await
            .map_err(|e| ChannelError::Provider(format!("matrix send: {e}")))?;
        let status = resp.status();
        let body = resp
            .text()
            .await
            .map_err(|e| ChannelError::Provider(format!("matrix response: {e}")))?;
        if !status.is_success() {
            return Err(ChannelError::Provider(format!(
                "matrix returned {status}: {body}"
            )));
        }
        let provider_message_id = serde_json::from_str::<Value>(&body)
            .ok()
            .and_then(|v| v.get("event_id").and_then(Value::as_str).map(str::to_string));
        Ok(DeliveryReceipt {
            channel_id: self.id(),
            notification_id: approval_id.to_string(),
            provider_message_id,
            status: DeliveryStatus::Sent,
            deferred_reason: None,
            delivered_at: Utc::now(),
        })
    }
}

#[async_trait]
impl NotifyChannel for MatrixChannel {
    fn id(&self) -> ChannelId {
        self.config.channel_id.clone()
    }

    async fn send(&self, notification: Notification) -> ChannelResult<DeliveryReceipt> {
        if self.config.access_token.trim().is_empty() {
            return Err(ChannelError::Config(
                "matrix access token must not be empty".into(),
            ));
        }
        self.send_to_room(&notification, None, None, &[]).await
    }

    async fn poll(&self) -> ChannelResult<Vec<InboundEvent>> {
        Ok(Vec::new())
    }
}

fn validate_user_id(s: &str) -> ChannelResult<()> {
    if !s.starts_with('@') || !s.contains(':') {
        return Err(ChannelError::Config(format!(
            "matrix user_id must be @local:domain (got `{s}`)"
        )));
    }
    Ok(())
}

fn validate_room_id(s: &str) -> ChannelResult<()> {
    if !s.starts_with('!') {
        return Err(ChannelError::Config(format!(
            "matrix room_id must start with `!` (got `{s}`)"
        )));
    }
    Ok(())
}

fn normalize_mention(raw: &str) -> String {
    if raw.starts_with('@') {
        raw.to_string()
    } else if let Some((local, domain)) = raw.split_once('@') {
        format!("@{local}:{domain}")
    } else {
        format!("@{raw}")
    }
}

fn initial_txn_seed() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|d| u64::try_from(d.as_nanos()).ok())
        .unwrap_or(0)
}

#[cfg(test)]
#[path = "matrix_tests.rs"]
mod matrix_tests;
