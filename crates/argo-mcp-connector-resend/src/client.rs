//! Resend REST client.
//!
//! All requests carry `Authorization: Bearer <re_…>` + JSON bodies.
//! Methods: `POST /emails`, `POST /emails/batch` (≤100 per call),
//! `GET /emails/<id>`, `GET /domains`. 401 → [`ResendError::AuthFailed`];
//! 429 → [`ResendError::RateLimited`] with `Retry-After` (default `1`).
//! Resend's default rate limit is 10 requests/second.

use crate::types::{
    ResendDomain, ResendEmail, ResendEmailStatus, ResendError, ResendMessageId, ResendRegion,
};

/// Env var holding the Resend API key (`re_…`).
pub const RESEND_API_KEY_ENV: &str = "ARGO_RESEND_API_KEY";

/// Env var holding the region selector (`us` / `eu`). Empty → `us`.
pub const RESEND_REGION_ENV: &str = "ARGO_RESEND_REGION";

/// Maximum recipients per `POST /emails/batch` call.
pub const BATCH_MAX: usize = 100;

/// Resend connector — cheap to clone (reqwest's client is `Arc`-backed).
#[derive(Clone)]
pub struct ResendClient {
    client: reqwest::Client,
    api_key: String,
    region: ResendRegion,
    base_url_override: Option<String>,
}

impl ResendClient {
    /// Build with an explicit API key + region.
    #[must_use]
    pub fn new(api_key: String, region: ResendRegion) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            region,
            base_url_override: None,
        }
    }

    /// Build from `ARGO_RESEND_API_KEY` + `ARGO_RESEND_REGION` env.
    pub fn from_env() -> Result<Self, ResendError> {
        let api_key = std::env::var(RESEND_API_KEY_ENV)
            .ok()
            .filter(|s| !s.trim().is_empty())
            .ok_or(ResendError::MissingApiKey {
                env: RESEND_API_KEY_ENV,
            })?;
        let region_raw = std::env::var(RESEND_REGION_ENV).unwrap_or_default();
        let region = ResendRegion::parse(&region_raw).ok_or_else(|| ResendError::InvalidEmail {
            reason: format!("unknown {RESEND_REGION_ENV}: {region_raw:?}"),
        })?;
        Ok(Self::new(api_key, region))
    }

    /// Test hook — point the connector at an arbitrary base URL.
    #[doc(hidden)]
    #[must_use]
    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.base_url_override = Some(base_url.trim_end_matches('/').to_string());
        self
    }

    /// Borrow the configured region (diagnostic / test helper).
    #[must_use]
    pub fn region(&self) -> ResendRegion {
        self.region
    }

    fn base_url(&self) -> &str {
        self.base_url_override
            .as_deref()
            .unwrap_or_else(|| self.region.base_url())
    }

    /// Send a single email via `POST /emails`.
    pub async fn send_email(&self, email: ResendEmail) -> Result<ResendMessageId, ResendError> {
        if email.html.is_none() && email.text.is_none() {
            return Err(ResendError::InvalidEmail {
                reason: "missing html or text".into(),
            });
        }
        let value = self.post_json("/emails", email_to_json(&email)).await?;
        extract_id(&value)
    }

    /// Send up to [`BATCH_MAX`] emails via `POST /emails/batch`.
    pub async fn send_batch(
        &self,
        emails: Vec<ResendEmail>,
    ) -> Result<Vec<ResendMessageId>, ResendError> {
        if emails.len() > BATCH_MAX {
            return Err(ResendError::BatchTooLarge {
                actual: emails.len(),
                max: BATCH_MAX,
            });
        }
        let body = serde_json::Value::Array(emails.iter().map(email_to_json).collect());
        let value = self.post_json("/emails/batch", body).await?;
        let arr = value
            .get("data")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| ResendError::Decode("batch: missing 'data' array".into()))?;
        arr.iter().map(extract_id).collect()
    }

    /// Look up the delivery status of a previously-sent email.
    pub async fn get_email(&self, message_id: &str) -> Result<ResendEmailStatus, ResendError> {
        let value = self.get(&format!("/emails/{message_id}")).await?;
        serde_json::from_value(value).map_err(|e| ResendError::Decode(format!("get_email: {e}")))
    }

    /// List the operator's registered sender domains.
    pub async fn list_domains(&self) -> Result<Vec<ResendDomain>, ResendError> {
        let value = self.get("/domains").await?;
        let arr = value
            .get("data")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| ResendError::Decode("list_domains: missing 'data' array".into()))?;
        arr.iter()
            .map(|item| {
                serde_json::from_value::<ResendDomain>(item.clone())
                    .map_err(|e| ResendError::Decode(format!("list_domains: {e}")))
            })
            .collect()
    }

    async fn post_json(
        &self,
        path: &str,
        body: serde_json::Value,
    ) -> Result<serde_json::Value, ResendError> {
        let url = format!("{}{path}", self.base_url());
        let resp = self
            .client
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| ResendError::Transport(format!("POST {path}: {e}")))?;
        decode_response(resp, path).await
    }

    async fn get(&self, path: &str) -> Result<serde_json::Value, ResendError> {
        let url = format!("{}{path}", self.base_url());
        let resp = self
            .client
            .get(&url)
            .bearer_auth(&self.api_key)
            .send()
            .await
            .map_err(|e| ResendError::Transport(format!("GET {path}: {e}")))?;
        decode_response(resp, path).await
    }
}

/// Encode a [`ResendEmail`] as Resend's flat JSON body, omitting empty
/// optional fields so we send `{ from, to, subject, html }` rather than
/// `{ from, to, subject, html, text: null, reply_to: null, … }`.
fn email_to_json(email: &ResendEmail) -> serde_json::Value {
    let mut obj = serde_json::Map::new();
    obj.insert("from".into(), email.from.clone().into());
    obj.insert(
        "to".into(),
        serde_json::Value::Array(email.to.iter().cloned().map(Into::into).collect()),
    );
    obj.insert("subject".into(), email.subject.clone().into());
    if let Some(html) = &email.html {
        obj.insert("html".into(), html.clone().into());
    }
    if let Some(text) = &email.text {
        obj.insert("text".into(), text.clone().into());
    }
    if let Some(reply_to) = &email.reply_to {
        obj.insert("reply_to".into(), reply_to.clone().into());
    }
    if !email.cc.is_empty() {
        obj.insert(
            "cc".into(),
            serde_json::Value::Array(email.cc.iter().cloned().map(Into::into).collect()),
        );
    }
    if !email.bcc.is_empty() {
        obj.insert(
            "bcc".into(),
            serde_json::Value::Array(email.bcc.iter().cloned().map(Into::into).collect()),
        );
    }
    if !email.tags.is_empty() {
        let tags = email
            .tags
            .iter()
            .map(|t| serde_json::json!({"name": t.name, "value": t.value}))
            .collect();
        obj.insert("tags".into(), serde_json::Value::Array(tags));
    }
    serde_json::Value::Object(obj)
}

fn extract_id(value: &serde_json::Value) -> Result<ResendMessageId, ResendError> {
    value
        .get("id")
        .and_then(serde_json::Value::as_str)
        .filter(|s| !s.is_empty())
        .map(|s| ResendMessageId(s.to_string()))
        .ok_or_else(|| ResendError::Decode(format!("missing 'id' in response: {value}")))
}

async fn decode_response(
    resp: reqwest::Response,
    path: &str,
) -> Result<serde_json::Value, ResendError> {
    let status = resp.status();
    let retry_after = resp
        .headers()
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.trim().parse::<u64>().ok());
    let body_text = resp
        .text()
        .await
        .map_err(|e| ResendError::Transport(format!("read {path}: {e}")))?;
    if status.is_success() {
        if body_text.trim().is_empty() {
            return Ok(serde_json::json!({}));
        }
        return serde_json::from_str(&body_text)
            .map_err(|e| ResendError::Decode(format!("{path}: {e}: {body_text}")));
    }
    Err(map_error(status.as_u16(), &body_text, retry_after))
}

/// Map a non-2xx response onto a typed [`ResendError`]. Resend's envelope
/// is `{ "name": "<code>", "message": "<text>" }`; we react to documented
/// codes (`validation_error`, `rate_limit_exceeded`, `recipient_blocked`)
/// plus message-substring matches for domain-verification and
/// missing-body. Unrecognized failures drop to [`ResendError::Other`].
fn map_error(status: u16, body: &str, retry_after: Option<u64>) -> ResendError {
    if status == 401 {
        return ResendError::AuthFailed;
    }
    if status == 429 {
        return ResendError::RateLimited {
            retry_after_secs: retry_after.unwrap_or(1),
        };
    }
    let envelope: Option<serde_json::Value> = serde_json::from_str(body).ok();
    let name = envelope
        .as_ref()
        .and_then(|v| v.get("name").or_else(|| v.get("error")))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("")
        .to_string();
    let message = envelope
        .as_ref()
        .and_then(|v| v.get("message"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("")
        .to_string();
    let msg_lower = message.to_ascii_lowercase();
    if msg_lower.contains("domain is not verified") || msg_lower.contains("domain not verified") {
        return ResendError::DomainNotVerified {
            domain: extract_quoted(&message).unwrap_or_default(),
        };
    }
    if msg_lower.contains("missing html or text") || msg_lower.contains("missing `html` or `text`")
    {
        return ResendError::InvalidEmail {
            reason: "missing html or text".into(),
        };
    }
    if name == "recipient_blocked" || msg_lower.contains("blocked") {
        return ResendError::RecipientBlocked {
            recipient: extract_quoted(&message).unwrap_or_default(),
        };
    }
    if name == "validation_error" {
        return ResendError::InvalidEmail {
            reason: if message.is_empty() {
                body.to_string()
            } else {
                message
            },
        };
    }
    ResendError::Other {
        status,
        body: body.to_string(),
    }
}

/// Pull the first quoted substring out of a Resend error message —
/// recovers the offending domain or recipient (Resend formats messages
/// like `"The 'example.com' domain is not verified."`).
fn extract_quoted(message: &str) -> Option<String> {
    for q in ['\'', '"'] {
        let Some(start) = message.find(q) else { continue };
        let rest = &message[start + 1..];
        let Some(end) = rest.find(q) else { continue };
        let span = &rest[..end];
        if !span.is_empty() {
            return Some(span.to_string());
        }
    }
    None
}

#[cfg(test)]
pub(crate) fn map_error_for_tests(
    status: u16,
    body: &str,
    retry_after: Option<u64>,
) -> ResendError {
    map_error(status, body, retry_after)
}
