//! Thin Stripe API HTTP client.
use crate::auth;
use argo_mcp::{CallToolResult, McpContent};

/// Stripe API client authenticated with a bearer secret key.
#[derive(Clone)]
pub struct StripeClient {
    http: reqwest::Client,
    base_url: String,
    api_key: String,
}

impl StripeClient {
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
                "argo-mcp-connector-stripe/",
                env!("CARGO_PKG_VERSION")
            ))
            .build()
            .map_err(|e| format!("stripe http client: {e}"))?;
        Ok(Self {
            http,
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
        })
    }

    /// GET `/v1/customers`.
    pub async fn list_customers(
        &self,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let mut query = limit_query(args, 10, 100);
        push_str(args, &mut query, "email");
        push_str(args, &mut query, "starting_after");
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

    /// POST `/v1/charges`.
    pub async fn create_charge(
        &self,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let mut form = vec![
            ("amount".into(), required_u64(args, "amount")?.to_string()),
            ("currency".into(), required_str(args, "currency")?),
        ];
        push_str(args, &mut form, "customer");
        push_str(args, &mut form, "source");
        push_str(args, &mut form, "description");
        self.post_form("/v1/charges", form).await
    }

    /// GET `/v1/charges`.
    pub async fn list_charges(
        &self,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let mut query = limit_query(args, 10, 100);
        push_str(args, &mut query, "customer");
        push_str(args, &mut query, "starting_after");
        self.get("/v1/charges", query).await
    }

    /// POST `/v1/refunds`.
    pub async fn refund_charge(
        &self,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let mut form = vec![("charge".into(), required_str(args, "charge")?)];
        push_u64(args, &mut form, "amount");
        push_str(args, &mut form, "reason");
        self.post_form("/v1/refunds", form).await
    }

    /// POST `/v1/invoices`.
    pub async fn create_invoice(
        &self,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let mut form = vec![("customer".into(), required_str(args, "customer")?)];
        push_str(args, &mut form, "description");
        push_str(args, &mut form, "collection_method");
        push_bool(args, &mut form, "auto_advance");
        self.post_form("/v1/invoices", form).await
    }

    /// GET `/v1/invoices`.
    pub async fn list_invoices(
        &self,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let mut query = limit_query(args, 10, 100);
        push_str(args, &mut query, "customer");
        push_str(args, &mut query, "status");
        push_str(args, &mut query, "starting_after");
        self.get("/v1/invoices", query).await
    }

    async fn get(
        &self,
        path: &str,
        query: Vec<(String, String)>,
    ) -> Result<serde_json::Value, String> {
        let resp = self
            .http
            .get(format!("{}{}", self.base_url, path))
            .bearer_auth(&self.api_key)
            .query(&query)
            .send()
            .await
            .map_err(|e| format!("stripe GET {path}: {e}"))?;
        decode_response("GET", path, resp).await
    }

    async fn post_form(
        &self,
        path: &str,
        form: Vec<(String, String)>,
    ) -> Result<serde_json::Value, String> {
        let resp = self
            .http
            .post(format!("{}{}", self.base_url, path))
            .bearer_auth(&self.api_key)
            .form(&form)
            .send()
            .await
            .map_err(|e| format!("stripe POST {path}: {e}"))?;
        decode_response("POST", path, resp).await
    }
}

/// Convert a Stripe tool result into MCP text content.
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
        .map_err(|e| format!("stripe {method} {path}: read response: {e}"))?;
    let parsed = if text.trim().is_empty() {
        serde_json::json!({})
    } else {
        serde_json::from_str(&text)
            .map_err(|e| format!("stripe {method} {path}: decode response: {e}: {text}"))?
    };
    if status.is_success() {
        Ok(parsed)
    } else {
        Err(format!("stripe {method} {path}: HTTP {status}: {parsed}"))
    }
}

fn required_str(args: &serde_json::Value, field: &str) -> Result<String, String> {
    args.get(field)
        .and_then(|v| v.as_str())
        .filter(|v| !v.trim().is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("missing string field '{field}'"))
}

fn required_u64(args: &serde_json::Value, field: &str) -> Result<u64, String> {
    args.get(field)
        .and_then(|v| v.as_u64())
        .ok_or_else(|| format!("missing integer field '{field}'"))
}

fn limit_query(args: &serde_json::Value, default: u64, max: u64) -> Vec<(String, String)> {
    vec![(
        "limit".into(),
        args.get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(default)
            .clamp(1, max)
            .to_string(),
    )]
}

fn push_str(args: &serde_json::Value, out: &mut Vec<(String, String)>, field: &str) {
    if let Some(value) = args.get(field).and_then(|v| v.as_str()) {
        out.push((field.into(), value.to_string()));
    }
}

fn push_u64(args: &serde_json::Value, out: &mut Vec<(String, String)>, field: &str) {
    if let Some(value) = args.get(field).and_then(|v| v.as_u64()) {
        out.push((field.into(), value.to_string()));
    }
}

fn push_bool(args: &serde_json::Value, out: &mut Vec<(String, String)>, field: &str) {
    if let Some(value) = args.get(field).and_then(|v| v.as_bool()) {
        out.push((field.into(), value.to_string()));
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

    fn serve_once(body: &'static str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = [0; 4096];
            let n = stream.read(&mut buf).unwrap();
            let req = String::from_utf8_lossy(&buf[..n]).to_ascii_lowercase();
            assert!(req.contains("authorization: bearer sk_test"));
            let status = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n";
            let response = format!("{status}Content-Length: {}\r\n\r\n{body}", body.len());
            stream.write_all(response.as_bytes()).unwrap();
        });
        format!("http://{addr}")
    }

    #[tokio::test]
    async fn list_customers_uses_bearer_auth() {
        let base = serve_once(r#"{"object":"list","data":[]}"#);
        let client = StripeClient::new("sk_test".into(), base).unwrap();
        let out = client
            .list_customers(&serde_json::json!({"limit": 2}))
            .await
            .unwrap();
        assert_eq!(out["object"], "list");
    }

    #[tokio::test]
    async fn create_charge_posts_form() {
        let base = serve_once(r#"{"id":"ch_123","object":"charge"}"#);
        let client = StripeClient::new("sk_test".into(), base).unwrap();
        let out = client
            .create_charge(&serde_json::json!({
                "amount": 5000,
                "currency": "usd",
                "customer": "cus_123"
            }))
            .await
            .unwrap();
        assert_eq!(out["id"], "ch_123");
    }

    #[tokio::test]
    async fn refund_charge_posts_refund() {
        let base = serve_once(r#"{"id":"re_123","object":"refund"}"#);
        let client = StripeClient::new("sk_test".into(), base).unwrap();
        let out = client
            .refund_charge(&serde_json::json!({"charge": "ch_123"}))
            .await
            .unwrap();
        assert_eq!(out["object"], "refund");
    }

    #[tokio::test]
    async fn create_invoice_posts_invoice() {
        let base = serve_once(r#"{"id":"in_123","object":"invoice"}"#);
        let client = StripeClient::new("sk_test".into(), base).unwrap();
        let out = client
            .create_invoice(&serde_json::json!({"customer": "cus_123"}))
            .await
            .unwrap();
        assert_eq!(out["id"], "in_123");
    }
}
