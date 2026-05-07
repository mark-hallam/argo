//! Thin Zendesk Support REST API client.

use crate::auth::ZendeskConfig;
use argo_mcp::{CallToolResult, McpContent};

/// Zendesk Support REST client with API-token basic auth.
#[derive(Clone)]
pub struct ZendeskClient {
    http: reqwest::Client,
    base_url: String,
    auth_user: String,
    api_token: String,
}

impl ZendeskClient {
    /// Build from `ZENDESK_*` env vars and credential fallback storage.
    pub fn from_env() -> Result<Self, String> {
        let cfg = ZendeskConfig::from_env()?;
        Self::new(cfg.email, cfg.api_token, cfg.base_url)
    }

    /// Build with explicit email, API token, and REST API base URL.
    pub fn new(email: String, api_token: String, base_url: String) -> Result<Self, String> {
        if email.trim().is_empty() {
            return Err("missing Zendesk email".into());
        }
        if api_token.trim().is_empty() {
            return Err("missing Zendesk API token".into());
        }
        let http = reqwest::Client::builder()
            .user_agent(concat!(
                "argo-mcp-connector-zendesk/",
                env!("CARGO_PKG_VERSION")
            ))
            .build()
            .map_err(|e| format!("zendesk http client: {e}"))?;
        Ok(Self {
            http,
            base_url: base_url.trim_end_matches('/').to_string(),
            auth_user: format!("{}/token", email.trim()),
            api_token,
        })
    }

    /// Search Zendesk tickets with a raw Zendesk search query.
    pub async fn list_tickets(
        &self,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let query = required_str(args, "query")?;
        self.get("/search.json", vec![("query".into(), query)])
            .await
    }

    /// Fetch one Zendesk ticket by id.
    pub async fn get_ticket(&self, args: &serde_json::Value) -> Result<serde_json::Value, String> {
        let id = clean_id(&required_str(args, "id")?)?;
        self.get(&format!("/tickets/{id}.json"), Vec::new()).await
    }

    /// Create a Zendesk ticket.
    pub async fn create_ticket(
        &self,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let ticket = required_object(args, "ticket")?;
        self.post_json("/tickets.json", serde_json::json!({ "ticket": ticket }))
            .await
    }

    /// Update a Zendesk ticket.
    pub async fn update_ticket(
        &self,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let id = clean_id(&required_str(args, "id")?)?;
        let ticket = required_object(args, "ticket")?;
        self.put_json(
            &format!("/tickets/{id}.json"),
            serde_json::json!({ "ticket": ticket }),
        )
        .await
    }

    /// Add a comment to a Zendesk ticket.
    pub async fn add_comment(&self, args: &serde_json::Value) -> Result<serde_json::Value, String> {
        let ticket_id = clean_id(&required_str(args, "ticket_id")?)?;
        let body = required_str(args, "body")?;
        let public = args
            .get("public")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        self.put_json(
            &format!("/tickets/{ticket_id}.json"),
            serde_json::json!({
                "ticket": {
                    "comment": {
                        "body": body,
                        "public": public
                    }
                }
            }),
        )
        .await
    }

    /// List Zendesk users.
    pub async fn list_users(&self, args: &serde_json::Value) -> Result<serde_json::Value, String> {
        let mut query = vec![("per_page".into(), bounded_limit(args, 25, 100).to_string())];
        if let Some(role) = optional_clean_role(args)? {
            query.push(("role".into(), role));
        }
        self.get("/users.json", query).await
    }

    /// List Zendesk macros.
    pub async fn list_macros(&self, args: &serde_json::Value) -> Result<serde_json::Value, String> {
        let mut query = vec![("per_page".into(), bounded_limit(args, 25, 100).to_string())];
        if let Some(active) = args.get("active").and_then(|v| v.as_bool()) {
            query.push(("active".into(), active.to_string()));
        }
        self.get("/macros.json", query).await
    }

    async fn get(
        &self,
        path: &str,
        query: Vec<(String, String)>,
    ) -> Result<serde_json::Value, String> {
        let resp = self
            .http
            .get(format!("{}{}", self.base_url, path))
            .basic_auth(&self.auth_user, Some(&self.api_token))
            .query(&query)
            .send()
            .await
            .map_err(|e| format!("zendesk GET {path}: {e}"))?;
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
            .basic_auth(&self.auth_user, Some(&self.api_token))
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("zendesk POST {path}: {e}"))?;
        decode_response("POST", path, resp).await
    }

    async fn put_json(
        &self,
        path: &str,
        body: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let resp = self
            .http
            .put(format!("{}{}", self.base_url, path))
            .basic_auth(&self.auth_user, Some(&self.api_token))
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("zendesk PUT {path}: {e}"))?;
        decode_response("PUT", path, resp).await
    }
}

/// Convert a Zendesk result into MCP text content.
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
        .map_err(|e| format!("zendesk {method} {path}: read response: {e}"))?;
    let parsed = if text.trim().is_empty() {
        serde_json::json!({})
    } else {
        serde_json::from_str(&text)
            .map_err(|e| format!("zendesk {method} {path}: decode response: {e}: {text}"))?
    };
    if status.is_success() {
        Ok(parsed)
    } else {
        Err(format!("zendesk {method} {path}: HTTP {status}: {parsed}"))
    }
}

fn required_str(args: &serde_json::Value, field: &str) -> Result<String, String> {
    args.get(field)
        .and_then(|v| v.as_str())
        .filter(|v| !v.trim().is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("missing string field '{field}'"))
}

fn required_object(args: &serde_json::Value, field: &str) -> Result<serde_json::Value, String> {
    args.get(field)
        .filter(|v| v.is_object())
        .cloned()
        .ok_or_else(|| format!("missing object field '{field}'"))
}

fn bounded_limit(args: &serde_json::Value, default: u64, max: u64) -> u64 {
    args.get("limit")
        .and_then(|v| v.as_u64())
        .unwrap_or(default)
        .clamp(1, max)
}

fn optional_clean_role(args: &serde_json::Value) -> Result<Option<String>, String> {
    args.get("role")
        .and_then(|v| v.as_str())
        .map(clean_segment)
        .transpose()
}

fn clean_segment(raw: &str) -> Result<String, String> {
    if !raw.is_empty()
        && raw
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
    {
        Ok(raw.to_string())
    } else {
        Err(format!("unsafe Zendesk segment: {raw}"))
    }
}

fn clean_id(raw: &str) -> Result<String, String> {
    if !raw.is_empty() && raw.bytes().all(|b| b.is_ascii_digit()) {
        Ok(raw.to_string())
    } else {
        Err(format!("unsafe Zendesk id: {raw}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    #[tokio::test]
    async fn list_tickets_passes_search_query_through() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = [0; 4096];
            let n = stream.read(&mut buf).unwrap();
            let req = String::from_utf8_lossy(&buf[..n]);
            assert!(req.contains("GET /search.json?query=status%3Aopen+priority%3Ahigh"));
            assert!(req.to_ascii_lowercase().contains("authorization: basic "));
            let body = r#"{"results":[{"id":42,"subject":"broken"}]}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(response.as_bytes()).unwrap();
        });
        let client = ZendeskClient::new(
            "agent@example.com".into(),
            "tok".into(),
            format!("http://{addr}"),
        )
        .unwrap();
        let value = client
            .list_tickets(&serde_json::json!({"query": "status:open priority:high"}))
            .await
            .unwrap();
        assert_eq!(value["results"][0]["id"], 42);
    }

    #[tokio::test]
    async fn add_comment_uses_ticket_update_endpoint() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = [0; 4096];
            let n = stream.read(&mut buf).unwrap();
            let req = String::from_utf8_lossy(&buf[..n]);
            assert!(req.contains("PUT /tickets/123.json"));
            assert!(req.contains(r#""comment":{"body":"done","public":true}"#));
            let body = r#"{"ticket":{"id":123}}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(response.as_bytes()).unwrap();
        });
        let client = ZendeskClient::new(
            "agent@example.com".into(),
            "tok".into(),
            format!("http://{addr}"),
        )
        .unwrap();
        let value = client
            .add_comment(&serde_json::json!({
                "ticket_id": "123",
                "body": "done",
                "public": true
            }))
            .await
            .unwrap();
        assert_eq!(value["ticket"]["id"], 123);
    }
}
