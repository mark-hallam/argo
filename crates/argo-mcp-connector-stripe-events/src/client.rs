//! Thin read-only Stripe API HTTP client.

use crate::auth;
use argo_mcp::{CallToolResult, McpContent};

/// Stripe events API client authenticated with a bearer secret key.
#[derive(Clone)]
pub struct StripeEventsClient {
    http: reqwest::Client,
    base_url: String,
    api_key: String,
}

impl StripeEventsClient {
    /// Build from `STRIPE_API_KEY`.
    pub fn from_env() -> Result<Self, String> {
        Self::new(auth::api_key_from_env()?, auth::base_url_from_env())
    }

    /// Build with an explicit key and base URL.
    pub fn new(api_key: String, base_url: String) -> Result<Self, String> {
        if api_key.trim().is_empty() {
            return Err(format!("missing {}", auth::STRIPE_API_KEY_ENV));
        }
        let http = reqwest::Client::builder()
            .user_agent(concat!(
                "argo-mcp-connector-stripe-events/",
                env!("CARGO_PKG_VERSION")
            ))
            .build()
            .map_err(|e| format!("stripe events http client: {e}"))?;
        Ok(Self {
            http,
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
        })
    }

    /// GET `/v1/events`.
    pub async fn list_events(&self, args: &serde_json::Value) -> Result<serde_json::Value, String> {
        let mut query = list_query(args);
        push_str(args, &mut query, "type");
        push_created(args, &mut query);
        self.get("/v1/events", query).await
    }

    /// GET `/v1/events/{id}`.
    pub async fn get_event(&self, args: &serde_json::Value) -> Result<serde_json::Value, String> {
        let id = clean_id(&required_str(args, "id")?)?;
        self.get(&format!("/v1/events/{id}"), Vec::new()).await
    }

    /// GET `/v1/charges`.
    pub async fn list_charges(
        &self,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let mut query = list_query(args);
        push_str(args, &mut query, "customer");
        push_str(args, &mut query, "payment_intent");
        push_created(args, &mut query);
        self.get("/v1/charges", query).await
    }

    /// GET `/v1/customers`.
    pub async fn list_customers(
        &self,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let mut query = list_query(args);
        push_str(args, &mut query, "email");
        self.get("/v1/customers", query).await
    }

    /// GET `/v1/customers/{id}`.
    pub async fn get_customer(
        &self,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let id = clean_id(&required_str(args, "id")?)?;
        self.get(&format!("/v1/customers/{id}"), Vec::new()).await
    }

    /// GET `/v1/subscriptions`.
    pub async fn list_subscriptions(
        &self,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let mut query = list_query(args);
        push_str(args, &mut query, "customer");
        push_str(args, &mut query, "status");
        push_str(args, &mut query, "price");
        self.get("/v1/subscriptions", query).await
    }

    async fn get(
        &self,
        path: &str,
        query: Vec<(String, String)>,
    ) -> Result<serde_json::Value, String> {
        let url = url_with_query(&self.base_url, path, &query)?;
        let resp = self
            .http
            .get(url)
            .bearer_auth(&self.api_key)
            .send()
            .await
            .map_err(|e| format!("stripe events GET {path}: {e}"))?;
        decode_response("GET", path, resp).await
    }
}

/// Convert a Stripe events result into MCP text content.
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
        .map_err(|e| format!("stripe events {method} {path}: read response: {e}"))?;
    let parsed = if text.trim().is_empty() {
        serde_json::json!({})
    } else {
        serde_json::from_str(&text)
            .map_err(|e| format!("stripe events {method} {path}: decode response: {e}: {text}"))?
    };
    if status.is_success() {
        Ok(parsed)
    } else {
        Err(format!(
            "stripe events {method} {path}: HTTP {status}: {parsed}"
        ))
    }
}

fn url_with_query(
    base_url: &str,
    path: &str,
    query: &[(String, String)],
) -> Result<String, String> {
    let mut url = format!("{base_url}{path}");
    if !query.is_empty() {
        let encoded = serde_urlencoded::to_string(query)
            .map_err(|e| format!("stripe events encode query {path}: {e}"))?;
        url.push('?');
        url.push_str(&encoded);
    }
    Ok(url)
}

fn required_str(args: &serde_json::Value, field: &str) -> Result<String, String> {
    args.get(field)
        .and_then(|v| v.as_str())
        .filter(|v| !v.trim().is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("missing string field '{field}'"))
}

fn optional_str(args: &serde_json::Value, field: &str) -> Option<String> {
    args.get(field)
        .and_then(|v| v.as_str())
        .filter(|v| !v.trim().is_empty())
        .map(str::to_string)
}

fn list_query(args: &serde_json::Value) -> Vec<(String, String)> {
    let mut query = vec![(
        "limit".into(),
        args.get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(10)
            .clamp(1, 100)
            .to_string(),
    )];
    push_str(args, &mut query, "starting_after");
    push_str(args, &mut query, "ending_before");
    query
}

fn push_str(args: &serde_json::Value, out: &mut Vec<(String, String)>, field: &str) {
    if let Some(value) = optional_str(args, field) {
        out.push((field.into(), value));
    }
}

fn push_created(args: &serde_json::Value, out: &mut Vec<(String, String)>) {
    push_u64_as(args, out, "created", "created");
    push_u64_as(args, out, "created_gte", "created[gte]");
    push_u64_as(args, out, "created_lte", "created[lte]");
}

fn push_u64_as(
    args: &serde_json::Value,
    out: &mut Vec<(String, String)>,
    field: &str,
    stripe_key: &str,
) {
    if let Some(value) = args.get(field).and_then(|v| v.as_u64()) {
        out.push((stripe_key.into(), value.to_string()));
    }
}

fn clean_id(raw: &str) -> Result<String, String> {
    if !raw.is_empty()
        && raw
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
    {
        Ok(raw.to_string())
    } else {
        Err(format!("unsafe Stripe id: {raw}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    const EVENTS_FIXTURE: &str = r#"{
        "object":"list",
        "url":"/v1/events",
        "has_more":false,
        "data":[{"id":"evt_123","object":"event","type":"customer.created"}]
    }"#;

    fn client(base_url: String) -> StripeEventsClient {
        StripeEventsClient::new("sk_test".into(), base_url).unwrap()
    }

    fn serve_once(assert_req: impl FnOnce(&str) + Send + 'static, body: &'static str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = [0; 4096];
            let n = stream.read(&mut buf).unwrap();
            let req = String::from_utf8_lossy(&buf[..n]).to_string();
            assert_req(&req);
            let status = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n";
            let response = format!("{status}Content-Length: {}\r\n\r\n{body}", body.len());
            stream.write_all(response.as_bytes()).unwrap();
        });
        format!("http://{addr}")
    }

    #[tokio::test]
    async fn wire_list_events_decodes_recorded_fixture() {
        let base = serve_once(
            |req| {
                assert!(req.starts_with("GET /v1/events?"));
                assert!(req.contains("authorization: Bearer sk_test"));
                assert!(req.contains("limit=25"));
                assert!(req.contains("starting_after=evt_001"));
                assert!(req.contains("type=customer.created"));
            },
            EVENTS_FIXTURE,
        );
        let out = client(base)
            .list_events(&serde_json::json!({
                "limit": 25,
                "starting_after": "evt_001",
                "type": "customer.created"
            }))
            .await
            .unwrap();
        assert_eq!(out["data"][0]["id"], "evt_123");
    }

    #[tokio::test]
    async fn list_subscriptions_passes_cursor_params() {
        let base = serve_once(
            |req| {
                assert!(req.contains("/v1/subscriptions?"));
                assert!(req.contains("ending_before=sub_999"));
                assert!(req.contains("customer=cus_123"));
            },
            r#"{"object":"list","data":[]}"#,
        );
        let out = client(base)
            .list_subscriptions(&serde_json::json!({
                "ending_before": "sub_999",
                "customer": "cus_123"
            }))
            .await
            .unwrap();
        assert_eq!(out["object"], "list");
    }

    #[test]
    fn rejects_unsafe_path_segments() {
        assert!(clean_id("evt_123").is_ok());
        assert!(clean_id("../evt_123").is_err());
        assert!(clean_id("evt/123").is_err());
    }
}
