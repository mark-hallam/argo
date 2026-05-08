//! Thin Mailchimp Marketing API client.

use crate::auth::{self, MailchimpConfig};
use argo_mcp::{CallToolResult, McpContent};

const BASIC_AUTH_USER: &str = "argo";

/// Mailchimp client authenticated with API key Basic auth.
#[derive(Clone)]
pub struct MailchimpClient {
    http: reqwest::Client,
    base_url: String,
    api_key: String,
}

impl MailchimpClient {
    /// Build from `MAILCHIMP_API_KEY` and `MAILCHIMP_DC`.
    pub fn from_env() -> Result<Self, String> {
        Self::from_config(auth::config_from_env()?)
    }

    /// Build from an explicit Mailchimp config.
    pub fn from_config(config: MailchimpConfig) -> Result<Self, String> {
        Self::new(config.api_key, config.base_url)
    }

    /// Build with an explicit API key and API base URL.
    pub fn new(api_key: String, base_url: String) -> Result<Self, String> {
        if api_key.trim().is_empty() {
            return Err(format!("missing {}", auth::MAILCHIMP_API_KEY_ENV));
        }
        let http = reqwest::Client::builder()
            .user_agent(concat!(
                "argo-mcp-connector-mailchimp/",
                env!("CARGO_PKG_VERSION")
            ))
            .build()
            .map_err(|e| format!("mailchimp http client: {e}"))?;
        Ok(Self {
            http,
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
        })
    }

    /// GET `/lists`.
    pub async fn list_audiences(
        &self,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        self.get("/lists", paging_query(args)).await
    }

    /// GET `/campaigns`.
    pub async fn list_campaigns(
        &self,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let mut query = paging_query(args);
        push_query_str(args, &mut query, "type");
        push_query_str(args, &mut query, "status");
        self.get("/campaigns", query).await
    }

    /// GET `/campaigns/{campaign_id}`.
    pub async fn get_campaign(
        &self,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let id = encode_path_segment(&required_str(args, "campaign_id")?);
        self.get(&format!("/campaigns/{id}"), Vec::new()).await
    }

    /// POST `/campaigns/{campaign_id}/actions/send`.
    pub async fn send_campaign(
        &self,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let id = encode_path_segment(&required_str(args, "campaign_id")?);
        self.post_json(
            &format!("/campaigns/{id}/actions/send"),
            serde_json::json!({}),
        )
        .await
    }

    /// GET `/lists/{list_id}/members`.
    pub async fn list_subscribers(
        &self,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let list_id = encode_path_segment(&required_str(args, "list_id")?);
        let mut query = paging_query(args);
        push_query_str(args, &mut query, "status");
        self.get(&format!("/lists/{list_id}/members"), query).await
    }

    /// POST `/lists/{list_id}/members`.
    pub async fn add_subscriber(
        &self,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let list_id = encode_path_segment(&required_str(args, "list_id")?);
        let mut body = serde_json::Map::new();
        body.insert("email_address".into(), required_str(args, "email")?.into());
        body.insert(
            "status".into(),
            optional_str(args, "status")
                .unwrap_or_else(|| "subscribed".into())
                .into(),
        );
        if let Some(fields) = args.get("merge_fields").filter(|value| value.is_object()) {
            body.insert("merge_fields".into(), fields.clone());
        }
        self.post_json(
            &format!("/lists/{list_id}/members"),
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
            .basic_auth(BASIC_AUTH_USER, Some(&self.api_key))
            .query(&query)
            .send()
            .await
            .map_err(|e| format!("mailchimp GET {path}: {e}"))?;
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
            .basic_auth(BASIC_AUTH_USER, Some(&self.api_key))
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("mailchimp POST {path}: {e}"))?;
        decode_response("POST", path, resp).await
    }
}

/// Convert a Mailchimp result into MCP text content.
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
        .map_err(|e| format!("mailchimp {method} {path}: read response: {e}"))?;
    let parsed = if text.trim().is_empty() {
        serde_json::json!({})
    } else {
        serde_json::from_str(&text)
            .map_err(|e| format!("mailchimp {method} {path}: decode response: {e}: {text}"))?
    };
    if status.is_success() {
        Ok(parsed)
    } else {
        Err(format!(
            "mailchimp {method} {path}: HTTP {status}: {parsed}"
        ))
    }
}

fn paging_query(args: &serde_json::Value) -> Vec<(String, String)> {
    let count = args
        .get("count")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(25)
        .clamp(1, 100)
        .to_string();
    let mut query = vec![("count".into(), count)];
    if let Some(offset) = args.get("offset").and_then(serde_json::Value::as_u64) {
        query.push(("offset".into(), offset.to_string()));
    }
    query
}

fn push_query_str(args: &serde_json::Value, query: &mut Vec<(String, String)>, field: &str) {
    if let Some(value) = optional_str(args, field) {
        query.push((field.into(), value));
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
    async fn list_audiences_uses_basic_auth() {
        let (base, server) = serve_once(r#"{"lists":[]}"#).await;
        let client = MailchimpClient::new("mc_key".into(), base).expect("client");
        client
            .list_audiences(&serde_json::json!({"count": 5}))
            .await
            .expect("audiences");
        let req = server.await.expect("server");
        assert!(req.to_ascii_lowercase().contains("authorization: basic "));
        assert!(req.starts_with("GET /lists?count=5 "));
    }

    #[tokio::test]
    async fn send_campaign_posts_action_path() {
        let (base, server) = serve_once(r#"{}"#).await;
        let client = MailchimpClient::new("mc_key".into(), base).expect("client");
        client
            .send_campaign(&serde_json::json!({"campaign_id": "abc"}))
            .await
            .expect("send");
        let req = server.await.expect("server");
        assert!(req.starts_with("POST /campaigns/abc/actions/send "));
    }

    #[tokio::test]
    async fn add_subscriber_posts_member_body() {
        let (base, server) = serve_once(r#"{"id":"member"}"#).await;
        let client = MailchimpClient::new("mc_key".into(), base).expect("client");
        client
            .add_subscriber(&serde_json::json!({
                "list_id": "list1",
                "email": "a@example.com",
                "merge_fields": {"FNAME": "A"}
            }))
            .await
            .expect("add subscriber");
        let req = server.await.expect("server");
        assert!(req.starts_with("POST /lists/list1/members "));
        let body = request_body(&req);
        assert_eq!(body["email_address"], "a@example.com");
        assert_eq!(body["status"], "subscribed");
        assert_eq!(body["merge_fields"]["FNAME"], "A");
    }
}
