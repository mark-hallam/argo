//! Thin Calendly REST API client.

use crate::auth;
use argo_mcp::{CallToolResult, McpContent};

/// Calendly REST client authenticated with a personal access token.
#[derive(Clone)]
pub struct CalendlyClient {
    http: reqwest::Client,
    base_url: String,
    token: String,
}

impl CalendlyClient {
    /// Build from `CALENDLY_API_KEY` and optional `CALENDLY_BASE_URL`.
    pub fn from_env() -> Result<Self, String> {
        Self::new(auth::api_key_from_env()?, auth::base_url_from_env())
    }

    /// Build with an explicit personal access token and API base URL.
    pub fn new(token: String, base_url: String) -> Result<Self, String> {
        if token.trim().is_empty() {
            return Err(format!("missing {}", auth::CALENDLY_API_KEY_ENV));
        }
        let http = reqwest::Client::builder()
            .user_agent(concat!(
                "argo-mcp-connector-calendly/",
                env!("CARGO_PKG_VERSION")
            ))
            .build()
            .map_err(|e| format!("calendly http client: {e}"))?;
        Ok(Self {
            http,
            base_url: base_url.trim_end_matches('/').to_string(),
            token,
        })
    }

    /// GET `/event_types`, scoped to the provided Calendly user URI.
    pub async fn list_event_types(
        &self,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let mut query = paging_query(args);
        query.push(("user".into(), required_str(args, "user_uri")?));
        push_query_bool(args, &mut query, "active");
        self.get("/event_types", query).await
    }

    /// GET `/scheduled_events`, scoped to the provided Calendly user URI.
    pub async fn list_scheduled_events(
        &self,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let mut query = paging_query(args);
        query.push(("user".into(), required_str(args, "user_uri")?));
        push_query_str(args, &mut query, "min_start_time");
        push_query_str(args, &mut query, "max_start_time");
        push_query_str(args, &mut query, "status");
        self.get("/scheduled_events", query).await
    }

    /// GET `/scheduled_events/{event_id}`.
    pub async fn get_event(&self, args: &serde_json::Value) -> Result<serde_json::Value, String> {
        let event_id = scheduled_event_id(args)?;
        self.get(&format!("/scheduled_events/{event_id}"), Vec::new())
            .await
    }

    /// POST `/scheduled_events/{event_id}/cancellation`.
    pub async fn cancel_event(
        &self,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let event_id = scheduled_event_id(args)?;
        let mut body = serde_json::Map::new();
        if let Some(reason) = optional_str(args, "reason") {
            body.insert("reason".into(), reason.into());
        }
        self.post_json(
            &format!("/scheduled_events/{event_id}/cancellation"),
            serde_json::Value::Object(body),
        )
        .await
    }

    /// GET `/scheduled_events/{event_id}/invitees/{invitee_id}`.
    pub async fn get_invitee(&self, args: &serde_json::Value) -> Result<serde_json::Value, String> {
        let event_id = scheduled_event_id(args)?;
        let invitee_id = invitee_id(args)?;
        self.get(
            &format!("/scheduled_events/{event_id}/invitees/{invitee_id}"),
            Vec::new(),
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
            .bearer_auth(&self.token)
            .query(&query)
            .send()
            .await
            .map_err(|e| format!("calendly GET {path}: {e}"))?;
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
            .bearer_auth(&self.token)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("calendly POST {path}: {e}"))?;
        decode_response("POST", path, resp).await
    }
}

/// Convert a Calendly result into MCP text content.
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
        .map_err(|e| format!("calendly {method} {path}: read response: {e}"))?;
    let parsed = if text.trim().is_empty() {
        serde_json::json!({})
    } else {
        serde_json::from_str(&text)
            .map_err(|e| format!("calendly {method} {path}: decode response: {e}: {text}"))?
    };
    if status.is_success() {
        Ok(parsed)
    } else {
        Err(format!("calendly {method} {path}: HTTP {status}: {parsed}"))
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
    push_query_str(args, &mut query, "page_token");
    query
}

fn push_query_str(args: &serde_json::Value, query: &mut Vec<(String, String)>, field: &str) {
    if let Some(value) = optional_str(args, field) {
        query.push((field.into(), value));
    }
}

fn push_query_bool(args: &serde_json::Value, query: &mut Vec<(String, String)>, field: &str) {
    if let Some(value) = args.get(field).and_then(serde_json::Value::as_bool) {
        query.push((field.into(), value.to_string()));
    }
}

fn scheduled_event_id(args: &serde_json::Value) -> Result<String, String> {
    uri_id(args, "event_uri", "event_id", "/scheduled_events/")
}

fn invitee_id(args: &serde_json::Value) -> Result<String, String> {
    uri_id(args, "invitee_uri", "invitee_id", "/invitees/")
}

fn uri_id(
    args: &serde_json::Value,
    uri_field: &str,
    id_field: &str,
    marker: &str,
) -> Result<String, String> {
    let raw = optional_str(args, uri_field)
        .or_else(|| optional_str(args, id_field))
        .ok_or_else(|| format!("missing string field '{uri_field}' or '{id_field}'"))?;
    let id = raw.rsplit(marker).next().unwrap_or(raw.as_str());
    Ok(encode_path_segment(id))
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
    async fn list_methods_scope_to_user_uri() {
        let user_uri = "https://api.calendly.com/users/u1";
        let (base, server) = serve_once(r#"{"collection":[]}"#).await;
        let client = CalendlyClient::new("cal_token".into(), base).expect("client");
        client
            .list_event_types(&serde_json::json!({"user_uri": user_uri, "count": 5}))
            .await
            .expect("event types");
        let req = server.await.expect("server");
        assert!(req
            .to_ascii_lowercase()
            .contains("authorization: bearer cal_token"));
        assert!(req.contains("user=https%3A%2F%2Fapi.calendly.com%2Fusers%2Fu1"));

        let (base, server) = serve_once(r#"{"collection":[]}"#).await;
        let client = CalendlyClient::new("cal_token".into(), base).expect("client");
        client
            .list_scheduled_events(&serde_json::json!({"user_uri": user_uri}))
            .await
            .expect("events");
        let req = server.await.expect("server");
        assert!(req.starts_with("GET /scheduled_events?count=25&user="));
    }

    #[tokio::test]
    async fn cancel_event_posts_cancellation_reason() {
        let (base, server) = serve_once(r#"{"resource":{"canceled":true}}"#).await;
        let client = CalendlyClient::new("cal_token".into(), base).expect("client");
        client
            .cancel_event(&serde_json::json!({"event_id": "evt1", "reason": "No longer needed"}))
            .await
            .expect("cancel");
        let req = server.await.expect("server");
        assert!(req.starts_with("POST /scheduled_events/evt1/cancellation "));
        assert_eq!(request_body(&req)["reason"], "No longer needed");
    }

    #[tokio::test]
    async fn get_invitee_extracts_ids_from_uris() {
        let (base, server) = serve_once(r#"{"resource":{"email":"a@example.com"}}"#).await;
        let client = CalendlyClient::new("cal_token".into(), base).expect("client");
        client
            .get_invitee(&serde_json::json!({
                "event_uri": "https://api.calendly.com/scheduled_events/evt1",
                "invitee_uri": "https://api.calendly.com/scheduled_events/evt1/invitees/inv1"
            }))
            .await
            .expect("invitee");
        let req = server.await.expect("server");
        assert!(req.starts_with("GET /scheduled_events/evt1/invitees/inv1 "));
    }
}
