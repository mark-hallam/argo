//! Thin Figma REST API client.

use crate::auth;
use argo_mcp::{CallToolResult, McpContent};
use reqwest::header::{HeaderMap, HeaderValue};

const FIGMA_TOKEN_HEADER: &str = "X-Figma-Token";

/// Figma REST client authenticated with a personal access token.
#[derive(Clone)]
pub struct FigmaClient {
    http: reqwest::Client,
    base_url: String,
}

impl FigmaClient {
    /// Build from `FIGMA_TOKEN` and optional `FIGMA_BASE_URL`.
    pub fn from_env() -> Result<Self, String> {
        Self::new(auth::token_from_env()?, auth::base_url_from_env())
    }

    /// Build with an explicit personal access token and API base URL.
    pub fn new(token: String, base_url: String) -> Result<Self, String> {
        if token.trim().is_empty() {
            return Err(format!("missing {}", auth::FIGMA_TOKEN_ENV));
        }
        let http = reqwest::Client::builder()
            .user_agent(concat!(
                "argo-mcp-connector-figma/",
                env!("CARGO_PKG_VERSION")
            ))
            .default_headers(default_headers(&token)?)
            .build()
            .map_err(|e| format!("figma http client: {e}"))?;
        Ok(Self {
            http,
            base_url: base_url.trim_end_matches('/').to_string(),
        })
    }

    /// GET `/projects/{project_id}/files`.
    pub async fn list_files(&self, args: &serde_json::Value) -> Result<serde_json::Value, String> {
        let project_id = encode_path_segment(&required_str(args, "project_id")?);
        self.get(&format!("/projects/{project_id}/files"), Vec::new())
            .await
    }

    /// GET `/files/{file_key}`.
    pub async fn get_file(&self, args: &serde_json::Value) -> Result<serde_json::Value, String> {
        let file_key = encode_path_segment(&required_str(args, "file_key")?);
        let mut query = Vec::new();
        push_str(args, &mut query, "version");
        push_str(args, &mut query, "ids");
        push_str(args, &mut query, "depth");
        push_str(args, &mut query, "geometry");
        self.get(&format!("/files/{file_key}"), query).await
    }

    /// GET `/files/{file_key}/components`.
    pub async fn get_file_components(
        &self,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let file_key = encode_path_segment(&required_str(args, "file_key")?);
        self.get(&format!("/files/{file_key}/components"), Vec::new())
            .await
    }

    /// GET `/files/{file_key}/comments`.
    pub async fn list_comments(
        &self,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let file_key = encode_path_segment(&required_str(args, "file_key")?);
        self.get(&format!("/files/{file_key}/comments"), Vec::new())
            .await
    }

    /// POST `/files/{file_key}/comments`.
    pub async fn add_comment(&self, args: &serde_json::Value) -> Result<serde_json::Value, String> {
        let file_key = encode_path_segment(&required_str(args, "file_key")?);
        let mut body = serde_json::Map::new();
        body.insert("message".into(), required_str(args, "message")?.into());
        if let Some(client_meta) = args.get("client_meta").filter(|value| value.is_object()) {
            body.insert("client_meta".into(), client_meta.clone());
        }
        self.post_json(
            &format!("/files/{file_key}/comments"),
            serde_json::Value::Object(body),
        )
        .await
    }

    async fn get(
        &self,
        path: &str,
        query: Vec<(String, String)>,
    ) -> Result<serde_json::Value, String> {
        let resp = self
            .http
            .get(format!("{}{}", self.base_url, path))
            .query(&query)
            .send()
            .await
            .map_err(|e| format!("figma GET {path}: {e}"))?;
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
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("figma POST {path}: {e}"))?;
        decode_response("POST", path, resp).await
    }
}

/// Convert a Figma result into MCP text content.
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
        .map_err(|e| format!("figma {method} {path}: read response: {e}"))?;
    let parsed = if text.trim().is_empty() {
        serde_json::json!({})
    } else {
        serde_json::from_str(&text)
            .map_err(|e| format!("figma {method} {path}: decode response: {e}: {text}"))?
    };
    if status.is_success() {
        Ok(parsed)
    } else {
        Err(format!("figma {method} {path}: HTTP {status}: {parsed}"))
    }
}

fn default_headers(token: &str) -> Result<HeaderMap, String> {
    let mut headers = HeaderMap::new();
    let value = HeaderValue::from_str(token)
        .map_err(|e| format!("invalid {} header: {e}", auth::FIGMA_TOKEN_ENV))?;
    headers.insert(FIGMA_TOKEN_HEADER, value);
    Ok(headers)
}

fn required_str(args: &serde_json::Value, field: &str) -> Result<String, String> {
    args.get(field)
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("missing string field '{field}'"))
}

fn push_str(args: &serde_json::Value, query: &mut Vec<(String, String)>, field: &str) {
    if let Some(value) = args
        .get(field)
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.trim().is_empty())
    {
        query.push((field.to_string(), value.to_string()));
    }
}

fn encode_path_segment(value: &str) -> String {
    let mut out = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(char::from(byte));
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    async fn serve_once(response: &'static str) -> (String, tokio::task::JoinHandle<String>) {
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
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{}",
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
    async fn list_files_uses_figma_token_header() {
        let (base, server) = serve_once(r#"{"files":[]}"#).await;
        let client = FigmaClient::new("fig_test".into(), base).expect("client");
        let out = client
            .list_files(&serde_json::json!({"project_id": "123"}))
            .await
            .expect("list files");
        assert!(out["files"].is_array());
        let req = server.await.expect("server");
        assert!(req.to_ascii_lowercase().contains("x-figma-token: fig_test"));
        assert!(req.starts_with("GET /projects/123/files "));
    }

    #[tokio::test]
    async fn get_file_preserves_file_key_path() {
        let (base, server) = serve_once(r#"{"name":"Design"}"#).await;
        let client = FigmaClient::new("fig_test".into(), base).expect("client");
        client
            .get_file(&serde_json::json!({"file_key": "abc123", "depth": "2"}))
            .await
            .expect("get file");
        let req = server.await.expect("server");
        assert!(req.starts_with("GET /files/abc123?depth=2 "));
    }

    #[tokio::test]
    async fn components_and_comments_preserve_file_key_path() {
        let (base, server) = serve_once(r#"{"meta":{"components":[]}}"#).await;
        let client = FigmaClient::new("fig_test".into(), base).expect("client");
        client
            .get_file_components(&serde_json::json!({"file_key": "abc123"}))
            .await
            .expect("components");
        let req = server.await.expect("server");
        assert!(req.starts_with("GET /files/abc123/components "));

        let (base, server) = serve_once(r#"{"comments":[]}"#).await;
        let client = FigmaClient::new("fig_test".into(), base).expect("client");
        client
            .list_comments(&serde_json::json!({"file_key": "abc123"}))
            .await
            .expect("comments");
        let req = server.await.expect("server");
        assert!(req.starts_with("GET /files/abc123/comments "));
    }

    #[tokio::test]
    async fn add_comment_posts_message_body() {
        let (base, server) = serve_once(r#"{"id":"comment-1"}"#).await;
        let client = FigmaClient::new("fig_test".into(), base).expect("client");
        client
            .add_comment(&serde_json::json!({
                "file_key": "abc123",
                "message": "Please review",
                "client_meta": {"node_id": "1:2"}
            }))
            .await
            .expect("add comment");
        let req = server.await.expect("server");
        assert!(req.starts_with("POST /files/abc123/comments "));
        let body = request_body(&req);
        assert_eq!(body["message"], "Please review");
        assert_eq!(body["client_meta"]["node_id"], "1:2");
    }
}
