//! WhatsApp Cloud REST client.
//!
//! All requests POST `{base_url}/v18.0/{phone_number_id}/messages` with
//! `Authorization: Bearer <access_token>`. The access token is read from
//! the env var named by `token_env` on every request — never cached. The
//! production `base_url` is `https://graph.facebook.com`; tests inject a
//! wiremock URL via [`WhatsAppCloudClient::with_base_url`].

use crate::types::{
    ErrorEnvelope, MediaBody, OutboundEnvelope, OutboundPayload, ReadReceipt, SendResponse,
    TemplateComponent, TemplateLanguage, TemplateParameter, TemplatePayload, TextBody,
    WhatsAppCloudError, WhatsAppMediaKind, WhatsAppMediaSource, WhatsAppMessageId,
    WhatsAppTemplateVariable,
};
use reqwest::header::HeaderMap;
use reqwest::StatusCode;
use serde::Serialize;
use std::time::Duration;

/// Production base URL for the Meta Graph API.
pub const META_GRAPH_BASE_URL: &str = "https://graph.facebook.com";
/// Default env var name for the long-lived system-user access token.
pub const DEFAULT_ACCESS_TOKEN_ENV: &str = "ARGO_WHATSAPP_CLOUD_ACCESS_TOKEN";
const GRAPH_API_VERSION: &str = "v18.0";
const DEFAULT_TIMEOUT_SECS: u64 = 30;
const DEFAULT_MAX_RETRY_DELAY_SECS: u64 = 60;

/// Typed WhatsApp Business Cloud client.
///
/// Cheap to construct (one [`reqwest::Client`] internally). The bearer
/// token is read from the env on each request and never cached on the
/// struct — see crate-level docs for the rationale.
pub struct WhatsAppCloudClient {
    client: reqwest::Client,
    phone_number_id: String,
    base_url: String,
    token_env: String,
    token_override: Option<String>,
    max_retry_delay: Duration,
}

impl WhatsAppCloudClient {
    /// Build a client for the given phone-number id.
    ///
    /// The token is read from `ARGO_WHATSAPP_CLOUD_ACCESS_TOKEN`; tests can
    /// override via [`Self::with_token_override`].
    #[must_use]
    pub fn new(phone_number_id: String) -> Self {
        let client = reqwest::Client::builder()
            .user_agent(concat!(
                "argo-mcp-connector-whatsapp-cloud/",
                env!("CARGO_PKG_VERSION")
            ))
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            client,
            phone_number_id,
            base_url: META_GRAPH_BASE_URL.to_string(),
            token_env: DEFAULT_ACCESS_TOKEN_ENV.to_string(),
            token_override: None,
            max_retry_delay: Duration::from_secs(DEFAULT_MAX_RETRY_DELAY_SECS),
        }
    }

    /// Override the API base URL. Tests point this at a wiremock server;
    /// production code should not call this.
    #[must_use]
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into().trim_end_matches('/').to_string();
        self
    }

    /// Override the env var read on each request. Defaults to
    /// `ARGO_WHATSAPP_CLOUD_ACCESS_TOKEN`.
    #[must_use]
    pub fn with_token_env(mut self, env_name: impl Into<String>) -> Self {
        self.token_env = env_name.into();
        self
    }

    /// Supply an explicit token. Bypasses env-lookup; intended for tests
    /// that need a deterministic auth header.
    #[must_use]
    pub fn with_token_override(mut self, token: impl Into<String>) -> Self {
        self.token_override = Some(token.into());
        self
    }

    /// Cap on `Retry-After` + jitter. Beyond this the 429 surfaces as
    /// [`WhatsAppCloudError::RateLimited`] to the caller. Default 60s.
    #[must_use]
    pub fn with_max_retry_delay(mut self, delay: Duration) -> Self {
        self.max_retry_delay = delay;
        self
    }

    /// Send a free-form text message. Subject to the 24-hour messaging
    /// window — outside that window, callers should use [`Self::send_template`].
    pub async fn send_text(
        &self,
        to_e164: &str,
        body: &str,
    ) -> Result<WhatsAppMessageId, WhatsAppCloudError> {
        validate_e164(to_e164)?;
        let envelope = OutboundEnvelope {
            messaging_product: "whatsapp",
            recipient_type: "individual",
            to: to_e164,
            payload: OutboundPayload::Text { text: TextBody { body } },
        };
        self.post_with_retry(&envelope).await
    }

    /// Send a pre-approved template message. `variables` populate the
    /// template body's positional `{{n}}` slots in order. Templates
    /// bypass the 24-hour window restriction.
    pub async fn send_template(
        &self,
        to_e164: &str,
        template_name: &str,
        language_code: &str,
        variables: &[WhatsAppTemplateVariable],
    ) -> Result<WhatsAppMessageId, WhatsAppCloudError> {
        validate_e164(to_e164)?;
        let components = if variables.is_empty() {
            Vec::new()
        } else {
            vec![TemplateComponent {
                kind: "body",
                parameters: variables
                    .iter()
                    .map(|v| TemplateParameter { kind: "text", text: v.value.clone() })
                    .collect(),
            }]
        };
        let envelope = OutboundEnvelope {
            messaging_product: "whatsapp",
            recipient_type: "individual",
            to: to_e164,
            payload: OutboundPayload::Template {
                template: TemplatePayload {
                    name: template_name,
                    language: TemplateLanguage { code: language_code },
                    components,
                },
            },
        };
        self.post_with_retry(&envelope).await
    }

    /// Send a media message. `source` is either an HTTPS URL fetched by
    /// Meta's edge or a pre-uploaded media id. `caption` is honored for
    /// `Image`, `Document`, and `Video`; ignored otherwise.
    pub async fn send_media(
        &self,
        to_e164: &str,
        kind: WhatsAppMediaKind,
        source: &WhatsAppMediaSource,
        caption: Option<&str>,
    ) -> Result<WhatsAppMessageId, WhatsAppCloudError> {
        validate_e164(to_e164)?;
        let (link, id) = match source {
            WhatsAppMediaSource::Url(u) => (Some(u.as_str()), None),
            WhatsAppMediaSource::Id(i) => (None, Some(i.as_str())),
        };
        let body = MediaBody { link, id, caption };
        let payload = match kind {
            WhatsAppMediaKind::Image => OutboundPayload::Image { image: body },
            WhatsAppMediaKind::Document => OutboundPayload::Document { document: body },
            WhatsAppMediaKind::Audio => OutboundPayload::Audio { audio: body },
            WhatsAppMediaKind::Video => OutboundPayload::Video { video: body },
            WhatsAppMediaKind::Sticker => OutboundPayload::Sticker { sticker: body },
        };
        let envelope = OutboundEnvelope {
            messaging_product: "whatsapp",
            recipient_type: "individual",
            to: to_e164,
            payload,
        };
        self.post_with_retry(&envelope).await
    }

    /// Mark an inbound message as read (outgoing-read receipt). Idempotent
    /// — Meta accepts repeated marks against the same id.
    pub async fn mark_read(&self, message_id: &str) -> Result<(), WhatsAppCloudError> {
        let body = ReadReceipt {
            messaging_product: "whatsapp",
            status: "read",
            message_id,
        };
        let (status, headers, text) = self.raw_post(&body).await?;
        if status.is_success() {
            return Ok(());
        }
        Err(map_http_error(status, &headers, &text))
    }

    async fn post_with_retry<B: Serialize>(
        &self,
        body: &B,
    ) -> Result<WhatsAppMessageId, WhatsAppCloudError> {
        let mut attempts: u32 = 0;
        loop {
            let (status, headers, text) = self.raw_post(body).await?;
            if status.is_success() {
                let parsed: SendResponse = serde_json::from_str(&text).map_err(|e| {
                    WhatsAppCloudError::Transport(format!("decode send response: {e}: {text}"))
                })?;
                let first = parsed.messages.into_iter().next().ok_or_else(|| {
                    WhatsAppCloudError::Other(format!("send response missing message id: {text}"))
                })?;
                return Ok(WhatsAppMessageId(first.id));
            }
            let err = map_http_error(status, &headers, &text);
            if attempts == 0 {
                if let WhatsAppCloudError::RateLimited { retry_after_secs } = &err {
                    if let Some(delay) = compute_retry_delay(*retry_after_secs, self.max_retry_delay)
                    {
                        tracing::warn!(
                            target: "argo_mcp_connector_whatsapp_cloud",
                            retry_after_secs = retry_after_secs,
                            "whatsapp-cloud: 429 — sleeping {:?} before single retry",
                            delay
                        );
                        tokio::time::sleep(delay).await;
                        attempts += 1;
                        continue;
                    }
                }
            }
            return Err(err);
        }
    }

    async fn raw_post<B: Serialize>(
        &self,
        body: &B,
    ) -> Result<(StatusCode, HeaderMap, String), WhatsAppCloudError> {
        let token = self.resolve_token()?;
        let url = format!(
            "{}/{}/{}/messages",
            self.base_url, GRAPH_API_VERSION, self.phone_number_id
        );
        let resp = self
            .client
            .post(&url)
            .bearer_auth(&token)
            .json(body)
            .send()
            .await
            .map_err(|e| WhatsAppCloudError::Transport(format!("post messages: {e}")))?;
        let status = resp.status();
        let headers = resp.headers().clone();
        let text = resp
            .text()
            .await
            .map_err(|e| WhatsAppCloudError::Transport(format!("read body: {e}")))?;
        Ok((status, headers, text))
    }

    fn resolve_token(&self) -> Result<String, WhatsAppCloudError> {
        if let Some(t) = &self.token_override {
            return Ok(t.clone());
        }
        std::env::var(&self.token_env).map_err(|_| {
            WhatsAppCloudError::AuthFailed(format!("env var {} not set", self.token_env))
        })
    }
}

fn validate_e164(value: &str) -> Result<(), WhatsAppCloudError> {
    let rest = value.strip_prefix('+').ok_or_else(|| {
        WhatsAppCloudError::InvalidRecipient(format!("must start with '+': {value:?}"))
    })?;
    if rest.is_empty() || rest.len() > 15 || !rest.bytes().all(|b| b.is_ascii_digit()) {
        return Err(WhatsAppCloudError::InvalidRecipient(format!(
            "must contain 1-15 digits after '+': {value:?}"
        )));
    }
    Ok(())
}

fn parse_retry_after_header(headers: &HeaderMap) -> u64 {
    headers
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.trim().parse::<u64>().ok())
        .unwrap_or(0)
}

fn jitter_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| u64::from(d.subsec_nanos()))
        .unwrap_or(0);
    nanos % 1000
}

fn compute_retry_delay(retry_after_secs: u64, ceiling: Duration) -> Option<Duration> {
    let base = Duration::from_secs(retry_after_secs);
    let total = if retry_after_secs == 0 {
        Duration::ZERO
    } else {
        base + Duration::from_millis(jitter_ms())
    };
    if total > ceiling {
        return None;
    }
    Some(total)
}

fn map_http_error(status: StatusCode, headers: &HeaderMap, body: &str) -> WhatsAppCloudError {
    if status == StatusCode::TOO_MANY_REQUESTS {
        return WhatsAppCloudError::RateLimited {
            retry_after_secs: parse_retry_after_header(headers),
        };
    }
    let parsed: Option<ErrorEnvelope> = serde_json::from_str(body).ok();
    let (code, message) = parsed
        .map(|e| (e.error.code, e.error.message))
        .unwrap_or((0, body.to_string()));
    if status == StatusCode::UNAUTHORIZED
        || status == StatusCode::FORBIDDEN
        || code == 131005
        || code == 131031
    {
        return WhatsAppCloudError::AuthFailed(message);
    }
    match code {
        130429 | 131048 => WhatsAppCloudError::RateLimited {
            retry_after_secs: parse_retry_after_header(headers),
        },
        131047 | 131026 | 470 => WhatsAppCloudError::OutsideMessagingWindow(message),
        131050 => WhatsAppCloudError::RecipientNotOptedIn(message),
        132001 | 132005 | 132007 | 132012 | 132015 => {
            WhatsAppCloudError::TemplateNotApproved(message)
        }
        131052 => WhatsAppCloudError::MediaTooLarge(message),
        _ => WhatsAppCloudError::Other(format!("HTTP {status}: {message}")),
    }
}

#[cfg(test)]
#[path = "client_tests.rs"]
mod client_tests;

