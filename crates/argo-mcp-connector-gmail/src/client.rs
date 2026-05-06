//! Thin Gmail API client for connector tools.
use crate::{auth, mime};
use argo_mcp::{CallToolResult, McpContent};

const DEFAULT_BASE_URL: &str = "https://gmail.googleapis.com";

/// Gmail API client with a bearer access token.
#[derive(Clone)]
pub struct GmailClient {
    http: reqwest::Client,
    base_url: String,
    token: String,
    refresh_on_401: bool,
}

impl GmailClient {
    /// Build from OAuth env vars and the workspace token cache.
    pub async fn from_env() -> Result<Self, String> {
        let token = auth::access_token_from_env().await?;
        let base_url = std::env::var("GMAIL_BASE_URL").unwrap_or_else(|_| DEFAULT_BASE_URL.into());
        Self::new(token, base_url, true)
    }

    /// Build from a raw access token for gated live tests.
    pub fn with_access_token(token: String) -> Result<Self, String> {
        Self::new(token, DEFAULT_BASE_URL.into(), false)
    }

    fn new(token: String, base_url: String, refresh_on_401: bool) -> Result<Self, String> {
        if token.trim().is_empty() {
            return Err("missing Gmail access token".into());
        }
        let http = reqwest::Client::builder()
            .user_agent(concat!(
                "argo-mcp-connector-gmail/",
                env!("CARGO_PKG_VERSION")
            ))
            .build()
            .map_err(|e| format!("gmail http client: {e}"))?;
        Ok(Self {
            http,
            base_url: base_url.trim_end_matches('/').to_string(),
            token,
            refresh_on_401,
        })
    }

    /// Search messages with Gmail query syntax.
    pub async fn search(&mut self, args: &serde_json::Value) -> Result<serde_json::Value, String> {
        let mut query = Vec::new();
        push_arg(args, &mut query, "q", "query");
        push_arg(args, &mut query, "pageToken", "page_token");
        for label in args
            .get("label_ids")
            .and_then(|v| v.as_array())
            .into_iter()
            .flatten()
            .filter_map(|v| v.as_str())
        {
            query.push(("labelIds".into(), label.to_string()));
        }
        query.push((
            "maxResults".into(),
            bounded_limit(args, 10, 100).to_string(),
        ));
        self.get("/gmail/v1/users/me/messages", query).await
    }

    /// Read one message by id.
    pub async fn read(&mut self, args: &serde_json::Value) -> Result<serde_json::Value, String> {
        let id = clean_id(&required_str(args, "id")?)?;
        let format = args
            .get("format")
            .and_then(|v| v.as_str())
            .unwrap_or("full");
        let query = vec![("format".into(), clean_format(format)?)];
        self.get(&format!("/gmail/v1/users/me/messages/{id}"), query)
            .await
    }

    /// Create a draft from a raw or structured RFC 2822 message.
    pub async fn draft(&mut self, args: &serde_json::Value) -> Result<serde_json::Value, String> {
        let message = message_body(args)?;
        self.post(
            "/gmail/v1/users/me/drafts",
            serde_json::json!({"message": message}),
        )
        .await
    }

    /// Send a raw or structured RFC 2822 message.
    pub async fn send(&mut self, args: &serde_json::Value) -> Result<serde_json::Value, String> {
        let message = message_body(args)?;
        self.post("/gmail/v1/users/me/messages/send", message).await
    }

    /// Add and/or remove labels from a message.
    pub async fn label(&mut self, args: &serde_json::Value) -> Result<serde_json::Value, String> {
        let id = clean_id(&required_str(args, "id")?)?;
        let body = serde_json::json!({
            "addLabelIds": string_array(args, "add_label_ids"),
            "removeLabelIds": string_array(args, "remove_label_ids")
        });
        self.post(&format!("/gmail/v1/users/me/messages/{id}/modify"), body)
            .await
    }

    async fn get(
        &mut self,
        path: &str,
        query: Vec<(String, String)>,
    ) -> Result<serde_json::Value, String> {
        self.request_json("GET", path, query, None).await
    }

    async fn post(
        &mut self,
        path: &str,
        body: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        self.request_json("POST", path, Vec::new(), Some(body))
            .await
    }

    async fn request_json(
        &mut self,
        method: &str,
        path: &str,
        query: Vec<(String, String)>,
        body: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, String> {
        let first = self.send_once(method, path, &query, body.as_ref()).await;
        if first.as_ref().is_err_and(|e| e.unauthorized) && self.refresh_on_401 {
            self.token = auth::refresh_and_cache().await?;
            return self
                .send_once(method, path, &query, body.as_ref())
                .await
                .map_err(|e| e.message);
        }
        first.map_err(|e| e.message)
    }

    async fn send_once(
        &self,
        method: &str,
        path: &str,
        query: &[(String, String)],
        body: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, GmailError> {
        let url = format!("{}{}", self.base_url, path);
        let mut req = match method {
            "GET" => self.http.get(url),
            "POST" => self.http.post(url),
            _ => {
                return Err(GmailError::new(
                    format!("unsupported method {method}"),
                    false,
                ))
            }
        }
        .bearer_auth(&self.token)
        .query(query);
        if let Some(body) = body {
            req = req.json(body);
        }
        let resp = req
            .send()
            .await
            .map_err(|e| GmailError::new(format!("gmail {method} {path}: {e}"), false))?;
        decode_response(method, path, resp).await
    }
}

/// Convert a Gmail tool result into MCP text content.
pub fn to_call_result(result: Result<serde_json::Value, String>) -> CallToolResult {
    match result {
        Ok(value) => CallToolResult {
            content: vec![McpContent::Text {
                text: value.to_string(),
            }],
            is_error: false,
        },
        Err(text) => CallToolResult {
            content: vec![McpContent::Text { text }],
            is_error: true,
        },
    }
}

#[derive(Debug)]
struct GmailError {
    message: String,
    unauthorized: bool,
}

impl GmailError {
    fn new(message: String, unauthorized: bool) -> Self {
        Self {
            message,
            unauthorized,
        }
    }
}

async fn decode_response(
    method: &str,
    path: &str,
    resp: reqwest::Response,
) -> Result<serde_json::Value, GmailError> {
    let status = resp.status();
    let text = resp.text().await.map_err(|e| {
        GmailError::new(format!("gmail {method} {path}: read response: {e}"), false)
    })?;
    let parsed = if text.trim().is_empty() {
        serde_json::json!({})
    } else {
        serde_json::from_str(&text).map_err(|e| {
            GmailError::new(
                format!("gmail {method} {path}: decode response: {e}: {text}"),
                false,
            )
        })?
    };
    if status.is_success() {
        Ok(parsed)
    } else {
        Err(GmailError::new(
            format!("gmail {method} {path}: HTTP {status}: {parsed}"),
            status == reqwest::StatusCode::UNAUTHORIZED,
        ))
    }
}

fn message_body(args: &serde_json::Value) -> Result<serde_json::Value, String> {
    let mut message = serde_json::json!({"raw": mime::raw_message(args)?});
    if let Some(thread_id) = args.get("thread_id").and_then(|v| v.as_str()) {
        message["threadId"] = clean_id(thread_id)?.into();
    }
    Ok(message)
}

fn push_arg(args: &serde_json::Value, query: &mut Vec<(String, String)>, wire: &str, field: &str) {
    if let Some(value) = args.get(field).and_then(|v| v.as_str()) {
        query.push((wire.into(), value.to_string()));
    }
}

fn required_str(args: &serde_json::Value, field: &str) -> Result<String, String> {
    args.get(field)
        .and_then(|v| v.as_str())
        .filter(|v| !v.trim().is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("missing string field '{field}'"))
}

fn bounded_limit(args: &serde_json::Value, default: u64, max: u64) -> u64 {
    args.get("limit")
        .and_then(|v| v.as_u64())
        .unwrap_or(default)
        .clamp(1, max)
}

fn string_array(args: &serde_json::Value, field: &str) -> Vec<String> {
    args.get(field)
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
        .filter_map(|v| v.as_str().map(str::to_string))
        .collect()
}

fn clean_id(raw: &str) -> Result<String, String> {
    if !raw.is_empty()
        && raw
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
    {
        Ok(raw.to_string())
    } else {
        Err(format!("unsafe Gmail id: {raw}"))
    }
}

fn clean_format(raw: &str) -> Result<String, String> {
    match raw {
        "minimal" | "full" | "metadata" | "raw" => Ok(raw.to_string()),
        other => Err(format!("unsupported Gmail read format: {other}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_rejects_path_characters() {
        assert!(clean_id("abc_123-def").is_ok());
        assert!(clean_id("../abc").is_err());
    }
}
