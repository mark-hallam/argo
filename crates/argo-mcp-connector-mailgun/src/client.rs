//! Mailgun REST client.
//!
//! POSTs `application/x-www-form-urlencoded` bodies to
//! `{base_url}/v3/{domain}/messages` with HTTP-Basic auth (user `api`,
//! password `<api_key>`). Repeated recipients are sent as repeated keys
//! (`to=a&to=b`), per Mailgun's wire format.

use crate::types::{MailgunMessage, MailgunRegion, MailgunSendResponse};
use argo_types::{ArgoError, ArgoResult};

/// Mailgun connector. Holds an HTTP client, API key, sending domain, and
/// region. Cheap to clone (the inner [`reqwest::Client`] uses an `Arc`).
#[derive(Clone)]
pub struct MailgunConnector {
    client: reqwest::Client,
    api_key: String,
    domain: String,
    region: MailgunRegion,
    base_url_override: Option<String>,
}

impl MailgunConnector {
    /// Build a connector for `domain` (e.g. `"mg.example.com"`) using
    /// `api_key` (the Mailgun "private API key", typically prefixed
    /// `key-…`). `region` selects the base URL.
    #[must_use]
    pub fn new(api_key: String, domain: String, region: MailgunRegion) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            domain,
            region,
            base_url_override: None,
        }
    }

    /// Test/local hook — point the connector at an arbitrary base URL
    /// (e.g. `http://127.0.0.1:PORT` for an HTTP fixture). Production
    /// callers should never use this.
    #[doc(hidden)]
    #[must_use]
    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.base_url_override = Some(base_url.trim_end_matches('/').to_string());
        self
    }

    /// Effective base URL — override if set, else `region.base_url()`.
    fn base_url(&self) -> &str {
        self.base_url_override
            .as_deref()
            .unwrap_or_else(|| self.region.base_url())
    }

    /// Borrow the configured region (diagnostic / test helper).
    #[must_use]
    pub fn region(&self) -> MailgunRegion {
        self.region
    }

    /// Send a message via Mailgun.
    ///
    /// On success returns Mailgun's `id` (with surrounding angle brackets
    /// stripped) and confirmation `message`. On HTTP 4xx/5xx returns
    /// [`ArgoError::Provider`] with the status and response body for audit
    /// trails.
    pub async fn send_email(&self, msg: MailgunMessage) -> ArgoResult<MailgunSendResponse> {
        if msg.to.is_empty() {
            return Err(ArgoError::Schema(
                "mailgun: at least one 'to' recipient required".into(),
            ));
        }
        if msg.text.is_none() && msg.html.is_none() {
            return Err(ArgoError::Schema(
                "mailgun: at least one of 'text' or 'html' required".into(),
            ));
        }
        let url = format!("{}/v3/{}/messages", self.base_url(), self.domain);
        let form = encode_form(&msg);
        let resp = self
            .client
            .post(&url)
            .basic_auth("api", Some(&self.api_key))
            .header("content-type", "application/x-www-form-urlencoded")
            .body(form)
            .send()
            .await
            .map_err(|e| ArgoError::Provider(format!("mailgun POST {url}: {e}")))?;
        let status = resp.status();
        let body = resp
            .text()
            .await
            .map_err(|e| ArgoError::Provider(format!("mailgun POST {url}: read body: {e}")))?;
        if !status.is_success() {
            return Err(ArgoError::Provider(format!(
                "mailgun POST {url}: HTTP {status}: {body}"
            )));
        }
        let parsed: serde_json::Value = serde_json::from_str(&body).map_err(|e| {
            ArgoError::Provider(format!("mailgun POST {url}: decode response: {e}: {body}"))
        })?;
        let id = parsed
            .get("id")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                ArgoError::Provider(format!("mailgun POST {url}: missing 'id' field: {body}"))
            })?;
        let message = parsed
            .get("message")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .to_string();
        Ok(MailgunSendResponse {
            id: strip_angle_brackets(id),
            message,
        })
    }
}

/// Encode a [`MailgunMessage`] as Mailgun's form body, with repeated keys
/// for `to`/`cc`/`bcc`/`o:tag`. Skips empty optional fields.
fn encode_form(msg: &MailgunMessage) -> String {
    let mut pairs: Vec<(&str, String)> = Vec::with_capacity(8 + msg.to.len() + msg.tags.len());
    pairs.push(("from", msg.from.clone()));
    for to in &msg.to {
        pairs.push(("to", to.clone()));
    }
    for cc in &msg.cc {
        pairs.push(("cc", cc.clone()));
    }
    for bcc in &msg.bcc {
        pairs.push(("bcc", bcc.clone()));
    }
    pairs.push(("subject", msg.subject.clone()));
    if let Some(text) = &msg.text {
        pairs.push(("text", text.clone()));
    }
    if let Some(html) = &msg.html {
        pairs.push(("html", html.clone()));
    }
    if let Some(reply_to) = &msg.reply_to {
        pairs.push(("h:Reply-To", reply_to.clone()));
    }
    for tag in &msg.tags {
        pairs.push(("o:tag", tag.clone()));
    }
    pairs
        .into_iter()
        .map(|(k, v)| format!("{}={}", percent_encode(k), percent_encode(&v)))
        .collect::<Vec<_>>()
        .join("&")
}

/// RFC 3986 application/x-www-form-urlencoded percent-encoding. Spaces map
/// to `+`. We hand-roll this so the crate avoids a new dependency for one
/// helper.
fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Strip leading `<` and trailing `>` Mailgun wraps Message-IDs in.
fn strip_angle_brackets(id: &str) -> String {
    let trimmed = id.trim();
    let no_lead = trimmed.strip_prefix('<').unwrap_or(trimmed);
    no_lead.strip_suffix('>').unwrap_or(no_lead).to_string()
}

#[cfg(test)]
pub(crate) fn percent_encode_for_tests(s: &str) -> String {
    percent_encode(s)
}
