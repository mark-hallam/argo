//! Shared Blue Bubbles HTTP helpers for SKYCOT iMessage surfaces.
//!
//! ```no_run
//! # async fn demo() -> argo_bluebubbles_client::BlueBubblesResult<()> {
//! use argo_bluebubbles_client::{BlueBubblesClient, SendMessageRequest};
//!
//! let client = BlueBubblesClient::new("http://127.0.0.1:1234", "password")?;
//! let _ = client.send_message(SendMessageRequest {
//!     chat_guid: "iMessage;+;+15551234567".into(),
//!     message: "approve action act-1?".into(),
//!     temp_guid: None,
//! }).await?;
//! # Ok(()) }
//! ```
#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

use hmac::{Hmac, Mac};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::Sha256;

const DEFAULT_USER_AGENT: &str = concat!("argo-bluebubbles-client/", env!("CARGO_PKG_VERSION"));

/// Result type for Blue Bubbles operations.
pub type BlueBubblesResult<T> = Result<T, BlueBubblesError>;

/// Errors returned by the Blue Bubbles client.
#[derive(Debug, thiserror::Error)]
pub enum BlueBubblesError {
    /// Static client configuration is invalid.
    #[error("bluebubbles config: {0}")]
    Config(String),
    /// HTTP transport failed.
    #[error("bluebubbles transport: {0}")]
    Transport(String),
    /// Server returned an unsuccessful HTTP status.
    #[error("bluebubbles HTTP {status}: {body}")]
    Http {
        /// HTTP status code.
        status: StatusCode,
        /// Response body.
        body: String,
    },
    /// Response or webhook payload could not be decoded.
    #[error("bluebubbles decode: {0}")]
    Decode(String),
    /// Webhook signature did not match.
    #[error("bluebubbles signature mismatch")]
    Signature,
}

/// Minimal Blue Bubbles REST client.
#[derive(Clone)]
pub struct BlueBubblesClient {
    http: reqwest::Client,
    base_url: String,
    password: String,
}

/// Request body for sending a plain iMessage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMessageRequest {
    /// Blue Bubbles chat GUID, for example `iMessage;+;+15551234567`.
    #[serde(rename = "chatGuid")]
    pub chat_guid: String,
    /// Text body to send.
    pub message: String,
    /// Optional caller-provided idempotency GUID.
    #[serde(rename = "tempGuid", skip_serializing_if = "Option::is_none")]
    pub temp_guid: Option<String>,
}

/// Request body for sending a tapback reaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendReactionRequest {
    /// Chat containing the target message.
    #[serde(rename = "chatGuid")]
    pub chat_guid: String,
    /// Message GUID to react to.
    #[serde(rename = "selectedMessageGuid")]
    pub selected_message_guid: String,
    /// Blue Bubbles reaction string, e.g. `love` or `-like`.
    pub reaction: String,
    /// Optional multipart part index.
    #[serde(rename = "partIndex", skip_serializing_if = "Option::is_none")]
    pub part_index: Option<u32>,
}

/// Query parameters accepted by list endpoints.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ListQuery {
    /// Maximum rows to return.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    /// Number of rows to skip.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<u32>,
    /// Related records to include.
    #[serde(rename = "with", skip_serializing_if = "Option::is_none")]
    pub with_fields: Option<Vec<String>>,
    /// Sort field or direction accepted by Blue Bubbles.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort: Option<String>,
}

/// Parsed inbound Blue Bubbles webhook event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueBubblesWebhookEvent {
    /// Provider event type when present.
    pub event: Option<String>,
    /// Sender handle or address.
    pub sender: Option<String>,
    /// Chat GUID associated with the reply.
    pub chat_guid: Option<String>,
    /// Message GUID when present.
    pub message_guid: Option<String>,
    /// Plain text body.
    pub text: String,
    /// Original webhook JSON.
    pub raw: Value,
}

impl BlueBubblesClient {
    /// Build a client from a server URL and password.
    pub fn new(
        base_url: impl Into<String>,
        password: impl Into<String>,
    ) -> BlueBubblesResult<Self> {
        let base_url = clean_base_url(base_url.into())?;
        let password = password.into();
        if password.trim().is_empty() {
            return Err(BlueBubblesError::Config(
                "BLUE_BUBBLES_PASSWORD must not be empty".into(),
            ));
        }
        let http = reqwest::Client::builder()
            .user_agent(DEFAULT_USER_AGENT)
            .build()
            .map_err(|e| BlueBubblesError::Transport(format!("http client: {e}")))?;
        Ok(Self {
            http,
            base_url,
            password,
        })
    }

    /// Build a client from `BLUE_BUBBLES_URL` and `BLUE_BUBBLES_PASSWORD`.
    pub fn from_env() -> BlueBubblesResult<Self> {
        let url = required_env("BLUE_BUBBLES_URL")?;
        let password = required_env("BLUE_BUBBLES_PASSWORD")?;
        Self::new(url, password)
    }

    /// Send a plain text iMessage.
    pub async fn send_message(&self, mut req: SendMessageRequest) -> BlueBubblesResult<Value> {
        if req
            .temp_guid
            .as_deref()
            .unwrap_or_default()
            .trim()
            .is_empty()
        {
            req.temp_guid = Some(uuid::Uuid::now_v7().to_string());
        }
        self.post_json("/api/v1/message/text", json!(req)).await
    }

    /// List chats known to the Blue Bubbles server.
    pub async fn list_chats(&self, query: ListQuery) -> BlueBubblesResult<Value> {
        self.post_json("/api/v1/chat/query", json!(query)).await
    }

    /// List messages, optionally filtered by Blue Bubbles query fields.
    pub async fn list_messages(&self, query: Value) -> BlueBubblesResult<Value> {
        self.post_json("/api/v1/message/query", query).await
    }

    /// Fetch one message by GUID.
    pub async fn get_message(&self, message_guid: &str) -> BlueBubblesResult<Value> {
        let guid = clean_guid(message_guid)?;
        self.get_json(&format!("/api/v1/message/{guid}")).await
    }

    /// Send a tapback reaction to a message.
    pub async fn send_reaction(&self, req: SendReactionRequest) -> BlueBubblesResult<Value> {
        self.post_json("/api/v1/message/react", json!(req)).await
    }

    async fn post_json(&self, path: &str, body: Value) -> BlueBubblesResult<Value> {
        let resp = self
            .http
            .post(self.url(path))
            .query(&[("password", self.password.as_str())])
            .json(&body)
            .send()
            .await
            .map_err(|e| BlueBubblesError::Transport(format!("POST {path}: {e}")))?;
        decode_response(path, resp).await
    }

    async fn get_json(&self, path: &str) -> BlueBubblesResult<Value> {
        let resp = self
            .http
            .get(self.url(path))
            .query(&[("password", self.password.as_str())])
            .send()
            .await
            .map_err(|e| BlueBubblesError::Transport(format!("GET {path}: {e}")))?;
        decode_response(path, resp).await
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }
}

/// Validate an HMAC-SHA256 webhook signature.
pub fn validate_webhook_signature(
    body: &[u8],
    signature: &str,
    secret: &str,
) -> BlueBubblesResult<()> {
    let expected = sign_webhook_body(body, secret)?;
    if constant_time_eq(expected.as_bytes(), signature.trim().as_bytes()) {
        Ok(())
    } else {
        Err(BlueBubblesError::Signature)
    }
}

/// Sign a webhook body with HMAC-SHA256.
pub fn sign_webhook_body(body: &[u8], secret: &str) -> BlueBubblesResult<String> {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
        .map_err(|e| BlueBubblesError::Config(format!("webhook secret: {e}")))?;
    mac.update(body);
    Ok(hex::encode(mac.finalize().into_bytes()))
}

/// Parse a Blue Bubbles webhook body into a normalized event.
pub fn parse_webhook_event(body: &[u8]) -> BlueBubblesResult<BlueBubblesWebhookEvent> {
    let raw: Value = serde_json::from_slice(body)
        .map_err(|e| BlueBubblesError::Decode(format!("webhook JSON: {e}")))?;
    let message = raw.get("message").unwrap_or(&raw);
    let text = string_at(message, &["text", "message", "body"])
        .or_else(|| string_at(&raw, &["text", "message", "body"]))
        .ok_or_else(|| BlueBubblesError::Decode("webhook missing message text".into()))?;
    Ok(BlueBubblesWebhookEvent {
        event: string_at(&raw, &["event", "type"]),
        sender: string_at(message, &["sender", "handle", "address", "from"])
            .or_else(|| string_at(&raw, &["sender", "handle", "from"])),
        chat_guid: string_at(message, &["chatGuid", "chat_guid"])
            .or_else(|| string_at(&raw, &["chatGuid", "chat_guid"])),
        message_guid: string_at(message, &["guid", "messageGuid", "message_guid"]),
        text,
        raw,
    })
}

async fn decode_response(path: &str, response: reqwest::Response) -> BlueBubblesResult<Value> {
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| BlueBubblesError::Transport(format!("{path}: read response: {e}")))?;
    if !status.is_success() {
        return Err(BlueBubblesError::Http { status, body });
    }
    if body.trim().is_empty() {
        return Ok(json!({}));
    }
    serde_json::from_str(&body)
        .map_err(|e| BlueBubblesError::Decode(format!("{path}: {e}: {body}")))
}

fn required_env(name: &str) -> BlueBubblesResult<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| BlueBubblesError::Config(format!("missing {name}")))
}

fn clean_base_url(raw: String) -> BlueBubblesResult<String> {
    let cleaned = raw.trim().trim_end_matches('/').to_owned();
    if cleaned.starts_with("http://") || cleaned.starts_with("https://") {
        Ok(cleaned)
    } else {
        Err(BlueBubblesError::Config(
            "BLUE_BUBBLES_URL must start with http:// or https://".into(),
        ))
    }
}

fn clean_guid(raw: &str) -> BlueBubblesResult<String> {
    if !raw.trim().is_empty() && !raw.contains('/') && !raw.contains("..") {
        Ok(raw.to_owned())
    } else {
        Err(BlueBubblesError::Config(format!(
            "unsafe Blue Bubbles GUID: {raw}"
        )))
    }
}

fn string_at(value: &Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(Value::as_str))
        .map(str::to_owned)
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0_u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    fn serve_once(body: &'static str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = [0; 4096];
            let n = stream.read(&mut buf).unwrap();
            let req = String::from_utf8_lossy(&buf[..n]);
            assert!(req.contains("password=secret"));
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).unwrap();
        });
        format!("http://{addr}")
    }

    #[tokio::test]
    async fn send_message_posts_with_password() {
        let client = BlueBubblesClient::new(serve_once(r#"{"ok":true}"#), "secret").unwrap();
        let out = client
            .send_message(SendMessageRequest {
                chat_guid: "iMessage;+;+15551234567".into(),
                message: "hi".into(),
                temp_guid: Some("tmp-1".into()),
            })
            .await
            .unwrap();
        assert_eq!(out["ok"], true);
    }

    #[tokio::test]
    async fn get_message_rejects_unsafe_guid() {
        let client = BlueBubblesClient::new("http://127.0.0.1:1", "secret").unwrap();
        assert!(client.get_message("../bad").await.is_err());
    }

    #[test]
    fn webhook_signature_round_trips() {
        let body = br#"{"message":{"text":"YES approval_token=abc"}}"#;
        let sig = sign_webhook_body(body, "secret").unwrap();
        validate_webhook_signature(body, &sig, "secret").unwrap();
        assert!(validate_webhook_signature(body, &sig, "wrong").is_err());
    }

    #[test]
    fn parses_nested_webhook_message() {
        let event = parse_webhook_event(
            br#"{"type":"message","message":{"text":"approve","handle":"+1555","chatGuid":"chat-1","guid":"msg-1"}}"#,
        )
        .unwrap();
        assert_eq!(event.text, "approve");
        assert_eq!(event.sender.as_deref(), Some("+1555"));
        assert_eq!(event.chat_guid.as_deref(), Some("chat-1"));
    }
}
