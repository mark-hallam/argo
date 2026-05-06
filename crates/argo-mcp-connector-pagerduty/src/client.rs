//! Thin PagerDuty REST API client.
use crate::auth;
use argo_mcp::{CallToolResult, McpContent};
use reqwest::Method;
use serde_json::{json, Map, Value};

const ACCEPT_HEADER: &str = "application/vnd.pagerduty+json;version=2";

/// PagerDuty REST API client authenticated with an API key.
#[derive(Clone)]
pub struct PagerDutyClient {
    http: reqwest::Client,
    base_url: String,
    api_key: String,
}

impl PagerDutyClient {
    /// Build from `PAGERDUTY_API_KEY` and optional `PAGERDUTY_BASE_URL`.
    pub fn from_env() -> Result<Self, String> {
        Self::new(auth::api_key_from_env()?, auth::base_url_from_env())
    }

    /// Build with an explicit API key and base URL.
    pub fn new(api_key: String, base_url: String) -> Result<Self, String> {
        if api_key.trim().is_empty() {
            return Err(format!("missing {}", auth::PAGERDUTY_API_KEY_ENV));
        }
        let http = reqwest::Client::builder()
            .user_agent(concat!(
                "argo-mcp-connector-pagerduty/",
                env!("CARGO_PKG_VERSION")
            ))
            .build()
            .map_err(|e| format!("pagerduty http client: {e}"))?;
        Ok(Self {
            http,
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
        })
    }

    /// GET `/incidents`.
    pub async fn list_incidents(&self, args: &Value) -> Result<Value, String> {
        let mut query = paging_query(args);
        push_string(args, &mut query, "since", "since");
        push_string(args, &mut query, "until", "until");
        push_string(args, &mut query, "date_range", "date_range");
        push_string(args, &mut query, "incident_key", "incident_key");
        push_string(args, &mut query, "sort_by", "sort_by");
        push_string(args, &mut query, "time_zone", "time_zone");
        push_array(args, &mut query, "statuses", "statuses[]");
        push_array(args, &mut query, "service_ids", "service_ids[]");
        push_array(args, &mut query, "team_ids", "team_ids[]");
        push_array(args, &mut query, "user_ids", "user_ids[]");
        push_array(args, &mut query, "urgencies", "urgencies[]");
        push_array(args, &mut query, "include", "include[]");
        self.get("/incidents", query).await
    }

    /// GET `/incidents/{incident_id}`.
    pub async fn get_incident(&self, args: &Value) -> Result<Value, String> {
        let id = path_segment(&required_str(args, "incident_id")?)?;
        self.get(&format!("/incidents/{id}"), Vec::new()).await
    }

    /// POST `/incidents`.
    pub async fn create_incident(&self, args: &Value) -> Result<Value, String> {
        let mut incident = Map::new();
        incident.insert("type".into(), "incident".into());
        incident.insert("title".into(), required_str(args, "title")?.into());
        incident.insert(
            "service".into(),
            json!({
                "id": required_str(args, "service_id")?,
                "type": "service_reference"
            }),
        );
        copy_string_value(args, &mut incident, "urgency", "urgency");
        copy_string_value(args, &mut incident, "incident_key", "incident_key");
        if let Some(details) = optional_str(args, "details") {
            incident.insert(
                "body".into(),
                json!({"type": "incident_body", "details": details}),
            );
        }
        if let Some(priority_id) = optional_str(args, "priority_id") {
            incident.insert(
                "priority".into(),
                json!({"id": priority_id, "type": "priority_reference"}),
            );
        }
        self.post(
            "/incidents",
            json!({"incident": Value::Object(incident)}),
            from_email(args),
        )
        .await
    }

    /// PUT `/incidents/{incident_id}` with status `acknowledged`.
    pub async fn acknowledge_incident(&self, args: &Value) -> Result<Value, String> {
        self.update_incident_status(args, "acknowledged").await
    }

    /// PUT `/incidents/{incident_id}` with status `resolved`.
    pub async fn resolve_incident(&self, args: &Value) -> Result<Value, String> {
        self.update_incident_status(args, "resolved").await
    }

    /// GET `/services`.
    pub async fn list_services(&self, args: &Value) -> Result<Value, String> {
        let mut query = paging_query(args);
        push_string(args, &mut query, "query", "query");
        push_string(args, &mut query, "sort_by", "sort_by");
        push_array(args, &mut query, "team_ids", "team_ids[]");
        push_array(args, &mut query, "include", "include[]");
        self.get("/services", query).await
    }

    /// GET `/oncalls`.
    pub async fn list_oncalls(&self, args: &Value) -> Result<Value, String> {
        let mut query = paging_query(args);
        push_string(args, &mut query, "since", "since");
        push_string(args, &mut query, "until", "until");
        push_string(args, &mut query, "time_zone", "time_zone");
        push_bool(args, &mut query, "earliest", "earliest");
        push_array(args, &mut query, "user_ids", "user_ids[]");
        push_array(
            args,
            &mut query,
            "escalation_policy_ids",
            "escalation_policy_ids[]",
        );
        push_array(args, &mut query, "schedule_ids", "schedule_ids[]");
        push_array(args, &mut query, "include", "include[]");
        self.get("/oncalls", query).await
    }

    async fn update_incident_status(&self, args: &Value, status: &str) -> Result<Value, String> {
        let id = path_segment(&required_str(args, "incident_id")?)?;
        self.put(
            &format!("/incidents/{id}"),
            json!({"incident": {"type": "incident_reference", "status": status}}),
            from_email(args),
        )
        .await
    }

    async fn get(&self, path: &str, query: Vec<(String, String)>) -> Result<Value, String> {
        let req = self.request(Method::GET, path).query(&query);
        decode_response(path, req.send().await).await
    }

    async fn post(&self, path: &str, body: Value, from: Option<String>) -> Result<Value, String> {
        let mut req = self.request(Method::POST, path).json(&body);
        if let Some(from) = from {
            req = req.header("From", from);
        }
        decode_response(path, req.send().await).await
    }

    async fn put(&self, path: &str, body: Value, from: Option<String>) -> Result<Value, String> {
        let mut req = self.request(Method::PUT, path).json(&body);
        if let Some(from) = from {
            req = req.header("From", from);
        }
        decode_response(path, req.send().await).await
    }

    fn request(&self, method: Method, path: &str) -> reqwest::RequestBuilder {
        self.http
            .request(method, format!("{}{}", self.base_url, path))
            .header("Authorization", format!("Token token={}", self.api_key))
            .header("Accept", ACCEPT_HEADER)
            .header("Content-Type", "application/json")
    }
}

/// Convert a PagerDuty tool result into MCP text content.
pub fn to_call_result(result: Result<Value, String>) -> CallToolResult {
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
    path: &str,
    response: Result<reqwest::Response, reqwest::Error>,
) -> Result<Value, String> {
    let response = response.map_err(|e| format!("pagerduty {path}: request failed: {e}"))?;
    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|e| format!("pagerduty {path}: read response: {e}"))?;
    let parsed = if text.trim().is_empty() {
        json!({})
    } else {
        serde_json::from_str(&text)
            .map_err(|e| format!("pagerduty {path}: decode response: {e}: {text}"))?
    };
    if status.is_success() {
        return Ok(parsed);
    }
    Err(format!("pagerduty {path}: HTTP {status}: {parsed}"))
}

fn paging_query(args: &Value) -> Vec<(String, String)> {
    let mut query = Vec::new();
    push_limit(args, &mut query);
    push_u64(args, &mut query, "offset", "offset");
    push_bool(args, &mut query, "total", "total");
    query
}

fn push_limit(args: &Value, query: &mut Vec<(String, String)>) {
    let limit = args
        .get("limit")
        .and_then(Value::as_u64)
        .unwrap_or(25)
        .clamp(1, 100);
    query.push(("limit".into(), limit.to_string()));
}

fn push_u64(args: &Value, query: &mut Vec<(String, String)>, source: &str, param: &str) {
    if let Some(value) = args.get(source).and_then(Value::as_u64) {
        query.push((param.into(), value.to_string()));
    }
}

fn push_bool(args: &Value, query: &mut Vec<(String, String)>, source: &str, param: &str) {
    if let Some(value) = args.get(source).and_then(Value::as_bool) {
        query.push((param.into(), value.to_string()));
    }
}

fn push_string(args: &Value, query: &mut Vec<(String, String)>, source: &str, param: &str) {
    if let Some(value) = optional_str(args, source) {
        query.push((param.into(), value));
    }
}

fn push_array(args: &Value, query: &mut Vec<(String, String)>, source: &str, param: &str) {
    if let Some(values) = args.get(source).and_then(Value::as_array) {
        for value in values.iter().filter_map(Value::as_str) {
            if !value.trim().is_empty() {
                query.push((param.into(), value.to_string()));
            }
        }
    }
}

fn required_str(args: &Value, field: &str) -> Result<String, String> {
    optional_str(args, field).ok_or_else(|| format!("missing string field '{field}'"))
}

fn optional_str(args: &Value, field: &str) -> Option<String> {
    args.get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
}

fn copy_string_value(args: &Value, out: &mut Map<String, Value>, source: &str, target: &str) {
    if let Some(value) = optional_str(args, source) {
        out.insert(target.into(), value.into());
    }
}

fn from_email(args: &Value) -> Option<String> {
    optional_str(args, "from_email").or_else(auth::from_email_from_env)
}

fn path_segment(raw: &str) -> Result<String, String> {
    if !raw.is_empty()
        && !raw.contains("..")
        && raw
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
    {
        Ok(raw.to_string())
    } else {
        Err(format!("unsafe PagerDuty path segment: {raw}"))
    }
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

    fn request_body(req: &str) -> Value {
        let body = req.split("\r\n\r\n").nth(1).expect("http body");
        serde_json::from_str(body).expect("json body")
    }

    #[tokio::test]
    async fn list_incidents_uses_token_auth_and_arrays() {
        let (base, server) = serve_once(r#"{"incidents":[]}"#).await;
        let client = PagerDutyClient::new("pd_test".into(), base).expect("client");
        client
            .list_incidents(&json!({"statuses": ["triggered"], "limit": 5}))
            .await
            .expect("list incidents");
        let req = server.await.expect("server");
        let lower = req.to_ascii_lowercase();
        assert!(lower.contains("authorization: token token=pd_test"));
        assert!(req.starts_with("GET /incidents?"));
        assert!(req.contains("statuses%5B%5D=triggered"));
        assert!(req.contains("limit=5"));
    }

    #[tokio::test]
    async fn create_incident_posts_service_reference() {
        let (base, server) = serve_once(r#"{"incident":{"id":"P1"}}"#).await;
        let client = PagerDutyClient::new("pd_test".into(), base).expect("client");
        client
            .create_incident(&json!({
                "title": "API down",
                "service_id": "P_SERVICE",
                "details": "Synthetic failure",
                "from_email": "ops@example.com"
            }))
            .await
            .expect("create incident");
        let req = server.await.expect("server");
        assert!(req.to_ascii_lowercase().contains("from: ops@example.com"));
        let body = request_body(&req);
        assert_eq!(body["incident"]["service"]["id"], "P_SERVICE");
        assert_eq!(body["incident"]["body"]["details"], "Synthetic failure");
    }

    #[tokio::test]
    async fn resolve_incident_puts_resolved_status() {
        let (base, server) = serve_once(r#"{"incident":{"id":"P1","status":"resolved"}}"#).await;
        let client = PagerDutyClient::new("pd_test".into(), base).expect("client");
        client
            .resolve_incident(&json!({"incident_id": "P1", "from_email": "ops@example.com"}))
            .await
            .expect("resolve incident");
        let req = server.await.expect("server");
        assert!(req.starts_with("PUT /incidents/P1 "));
        let body = request_body(&req);
        assert_eq!(body["incident"]["status"], "resolved");
    }

    #[test]
    fn path_segments_are_strict() {
        assert_eq!(path_segment("P1_ABC-123").expect("valid"), "P1_ABC-123");
        assert!(path_segment("../P1").is_err());
        assert!(path_segment("P1/ABC").is_err());
    }
}
