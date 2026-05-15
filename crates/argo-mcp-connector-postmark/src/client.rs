//! Thin Postmark REST API client.

use crate::auth;
use argo_mcp::{CallToolResult, McpContent};

/// Postmark REST client authenticated with a server token.
#[derive(Clone)]
pub struct PostmarkClient {
    http: reqwest::Client,
    base_url: String,
    server_token: String,
}

impl PostmarkClient {
    /// Build from `POSTMARK_SERVER_TOKEN`.
    pub fn from_env() -> Result<Self, String> {
        Self::new(auth::server_token_from_env()?, auth::base_url_from_env())
    }

    /// Build with an explicit server token and API base URL.
    pub fn new(server_token: String, base_url: String) -> Result<Self, String> {
        if server_token.trim().is_empty() {
            return Err(format!("missing {}", auth::POSTMARK_SERVER_TOKEN_ENV));
        }
        let http = reqwest::Client::builder()
            .user_agent(concat!(
                "argo-mcp-connector-postmark/",
                env!("CARGO_PKG_VERSION")
            ))
            .build()
            .map_err(|e| format!("postmark http client: {e}"))?;
        Ok(Self {
            http,
            base_url: base_url.trim_end_matches('/').to_string(),
            server_token,
        })
    }

    /// GET `/server`, returned by the `postmark.list_servers` tool.
    pub async fn list_servers(
        &self,
        _args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        self.get("/server", Vec::new()).await
    }

    /// POST `/email`.
    pub async fn send_email(&self, args: &serde_json::Value) -> Result<serde_json::Value, String> {
        let mut body = serde_json::Map::new();
        body.insert("From".into(), required_str(args, "from")?.into());
        body.insert("To".into(), required_str(args, "to")?.into());
        body.insert("Subject".into(), required_str(args, "subject")?.into());
        insert_optional(args, &mut body, "text_body", "TextBody");
        insert_optional(args, &mut body, "html_body", "HtmlBody");
        if !body.contains_key("TextBody") && !body.contains_key("HtmlBody") {
            return Err("missing text_body or html_body".into());
        }
        insert_optional(args, &mut body, "cc", "Cc");
        insert_optional(args, &mut body, "bcc", "Bcc");
        insert_optional(args, &mut body, "reply_to", "ReplyTo");
        insert_optional(args, &mut body, "tag", "Tag");
        insert_optional(args, &mut body, "track_links", "TrackLinks");
        insert_optional(args, &mut body, "message_stream", "MessageStream");
        copy_value(args, &mut body, "track_opens", "TrackOpens");
        copy_value(args, &mut body, "headers", "Headers");
        copy_value(args, &mut body, "metadata", "Metadata");
        copy_value(args, &mut body, "attachments", "Attachments");
        self.post_json("/email", serde_json::Value::Object(body))
            .await
    }

    /// GET `/messages/outbound`.
    pub async fn list_messages(
        &self,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let mut query = vec![
            (
                "count".into(),
                bounded_u64(args, "count", 50, 1, 500).to_string(),
            ),
            (
                "offset".into(),
                bounded_u64(args, "offset", 0, 0, 10_000).to_string(),
            ),
        ];
        push_query_str(args, &mut query, "recipient");
        push_query_str(args, &mut query, "fromemail");
        push_query_str(args, &mut query, "tag");
        push_query_str(args, &mut query, "status");
        push_query_str(args, &mut query, "todate");
        push_query_str(args, &mut query, "fromdate");
        push_query_str(args, &mut query, "subject");
        push_query_str_as(args, &mut query, "message_stream", "messagestream");
        self.get("/messages/outbound", query).await
    }

    /// GET `/messages/outbound/{message_id}/details`.
    pub async fn get_message(&self, args: &serde_json::Value) -> Result<serde_json::Value, String> {
        let message_id = clean_segment(&required_str(args, "message_id")?)?;
        self.get(
            &format!("/messages/outbound/{message_id}/details"),
            Vec::new(),
        )
        .await
    }

    /// GET `/templates`.
    pub async fn list_templates(
        &self,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let mut query = vec![
            (
                "count".into(),
                bounded_u64(args, "count", 100, 1, 500).to_string(),
            ),
            (
                "offset".into(),
                bounded_u64(args, "offset", 0, 0, 10_000).to_string(),
            ),
        ];
        push_query_str_as(args, &mut query, "layout_template", "LayoutTemplate");
        push_query_str_as(args, &mut query, "template_type", "TemplateType");
        self.get("/templates", query).await
    }

    async fn get(
        &self,
        path: &str,
        query: Vec<(String, String)>,
    ) -> Result<serde_json::Value, String> {
        let resp = self
            .http
            .get(format!("{}{}", self.base_url, path))
            .header("Accept", "application/json")
            .header("X-Postmark-Server-Token", &self.server_token)
            .query(&query)
            .send()
            .await
            .map_err(|e| format!("postmark GET {path}: {e}"))?;
        decode_response("GET", path, resp).await
    }

    async fn post_json(
        &self,
        path: &str,
        body: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let resp = self
            .http
            .post(format!("{}{}", self.base_url, path))
            .header("Accept", "application/json")
            .header("X-Postmark-Server-Token", &self.server_token)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("postmark POST {path}: {e}"))?;
        decode_response("POST", path, resp).await
    }
}

/// Convert a Postmark tool result into MCP text content.
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

async fn decode_response(
    method: &str,
    path: &str,
    resp: reqwest::Response,
) -> Result<serde_json::Value, String> {
    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| format!("postmark {method} {path}: read response: {e}"))?;
    let parsed = if text.trim().is_empty() {
        serde_json::json!({})
    } else {
        serde_json::from_str(&text)
            .map_err(|e| format!("postmark {method} {path}: decode response: {e}: {text}"))?
    };
    if status.is_success() {
        Ok(parsed)
    } else {
        Err(format!("postmark {method} {path}: HTTP {status}: {parsed}"))
    }
}

fn required_str(args: &serde_json::Value, field: &str) -> Result<String, String> {
    optional_str(args, field).ok_or_else(|| format!("missing string field '{field}'"))
}

fn optional_str(args: &serde_json::Value, field: &str) -> Option<String> {
    args.get(field)
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
}

fn insert_optional(
    args: &serde_json::Value,
    body: &mut serde_json::Map<String, serde_json::Value>,
    field: &str,
    wire: &str,
) {
    if let Some(value) = optional_str(args, field) {
        body.insert(wire.into(), value.into());
    }
}

fn copy_value(
    args: &serde_json::Value,
    body: &mut serde_json::Map<String, serde_json::Value>,
    field: &str,
    wire: &str,
) {
    if let Some(value) = args.get(field) {
        body.insert(wire.into(), value.clone());
    }
}

fn bounded_u64(args: &serde_json::Value, field: &str, default: u64, min: u64, max: u64) -> u64 {
    args.get(field)
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(default)
        .clamp(min, max)
}

fn push_query_str(args: &serde_json::Value, out: &mut Vec<(String, String)>, field: &str) {
    if let Some(value) = optional_str(args, field) {
        out.push((field.into(), value));
    }
}

fn push_query_str_as(
    args: &serde_json::Value,
    out: &mut Vec<(String, String)>,
    field: &str,
    wire: &str,
) {
    if let Some(value) = optional_str(args, field) {
        out.push((wire.into(), value));
    }
}

fn clean_segment(raw: &str) -> Result<String, String> {
    if !raw.is_empty()
        && raw
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
    {
        Ok(raw.to_string())
    } else {
        Err(format!("unsafe Postmark path segment: {raw}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    async fn serve_once(
        status: &'static str,
        response: &'static str,
    ) -> (String, tokio::task::JoinHandle<String>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let addr = listener.local_addr().expect("addr");
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept");
            let mut buf = vec![0_u8; 8192];
            let n = socket.read(&mut buf).await.expect("read");
            let req = String::from_utf8_lossy(&buf[..n]).to_string();
            let wire = format!(
                "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{}",
                response.len(),
                response
            );
            socket.write_all(wire.as_bytes()).await.expect("write");
            req
        });
        (format!("http://{addr}"), server)
    }

    fn request_body(req: &str) -> serde_json::Value {
        let body = req.split("\r\n\r\n").nth(1).expect("http body");
        serde_json::from_str(body).expect("json body")
    }

    #[tokio::test]
    async fn list_servers_uses_server_token_header() {
        let (base, server) = serve_once("200 OK", r#"{"ID":1,"Name":"prod"}"#).await;
        let client = PostmarkClient::new("pm-token".into(), base).expect("client");
        let out = client
            .list_servers(&serde_json::json!({}))
            .await
            .expect("server");
        assert_eq!(out["Name"], "prod");
        let req = server.await.expect("server");
        assert!(req.starts_with("GET /server "));
        assert!(req
            .to_ascii_lowercase()
            .contains("x-postmark-server-token: pm-token"));
    }

    #[tokio::test]
    async fn send_email_posts_pascal_case_body() {
        let (base, server) = serve_once("200 OK", r#"{"ErrorCode":0,"Message":"OK"}"#).await;
        let client = PostmarkClient::new("pm-token".into(), base).expect("client");
        client
            .send_email(&serde_json::json!({
                "from": "sender@example.com",
                "to": "receiver@example.com",
                "subject": "Hello",
                "text_body": "Plain",
                "message_stream": "outbound",
                "metadata": {"tenant": "acme"}
            }))
            .await
            .expect("send");
        let req = server.await.expect("server");
        assert!(req.starts_with("POST /email "));
        assert!(req
            .to_ascii_lowercase()
            .contains("x-postmark-server-token: pm-token"));
        let body = request_body(&req);
        assert_eq!(body["From"], "sender@example.com");
        assert_eq!(body["TextBody"], "Plain");
        assert_eq!(body["MessageStream"], "outbound");
        assert_eq!(body["Metadata"]["tenant"], "acme");
    }

    #[tokio::test]
    async fn list_messages_maps_filters_to_query() {
        let (base, server) = serve_once("200 OK", r#"{"TotalCount":0,"Messages":[]}"#).await;
        let client = PostmarkClient::new("pm-token".into(), base).expect("client");
        client
            .list_messages(&serde_json::json!({
                "count": 5,
                "offset": 2,
                "message_stream": "broadcast"
            }))
            .await
            .expect("messages");
        let req = server.await.expect("server");
        assert!(req.starts_with("GET /messages/outbound?count=5&offset=2&messagestream=broadcast "));
    }

    #[test]
    fn rejects_unsafe_message_id() {
        let err = clean_segment("../secret").unwrap_err();
        assert!(err.contains("unsafe Postmark path segment"));
    }
}
