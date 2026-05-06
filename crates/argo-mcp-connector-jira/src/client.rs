//! Thin Jira Cloud HTTP client.

use crate::auth::JiraConfig;
use argo_mcp::{CallToolResult, McpContent};
use reqwest::Method;
use serde_json::{json, Value};

/// Jira Cloud client authenticated with email + API token.
#[derive(Clone)]
pub struct JiraClient {
    http: reqwest::Client,
    config: JiraConfig,
}

impl JiraClient {
    /// Build from `JIRA_EMAIL`, `JIRA_API_TOKEN`, and `JIRA_BASE_URL`.
    ///
    /// ```no_run
    /// # fn doc() -> Result<(), String> {
    /// use argo_mcp_connector_jira::JiraClient;
    /// let _client = JiraClient::from_env()?;
    /// # Ok(()) }
    /// ```
    pub fn from_env() -> Result<Self, String> {
        Self::new(JiraConfig::from_env()?)
    }

    /// Build with explicit config.
    ///
    /// ```
    /// # fn doc() -> Result<(), String> {
    /// use argo_mcp_connector_jira::{JiraClient, JiraConfig};
    /// let cfg = JiraConfig::new("op@example.com", "token", "https://example.atlassian.net")?;
    /// let _client = JiraClient::new(cfg)?;
    /// # Ok(()) }
    /// ```
    pub fn new(config: JiraConfig) -> Result<Self, String> {
        let http = reqwest::Client::builder()
            .user_agent(concat!(
                "argo-mcp-connector-jira/",
                env!("CARGO_PKG_VERSION")
            ))
            .build()
            .map_err(|e| format!("jira http client: {e}"))?;
        Ok(Self { http, config })
    }

    /// GET `/rest/api/3/search` with JQL passthrough.
    pub async fn list_issues(&self, args: &Value) -> Result<Value, String> {
        let jql = required_str(args, "jql")?;
        let mut query = vec![("jql".to_string(), jql)];
        push_limit(args, &mut query, "maxResults", 20, 100);
        push_u64(args, &mut query, "startAt");
        push_csv_or_string(args, &mut query, "fields");
        self.get("/rest/api/3/search", query).await
    }

    /// GET `/rest/api/3/issue/{issue_key}`.
    pub async fn get_issue(&self, args: &Value) -> Result<Value, String> {
        let issue_key = path_segment(&required_str(args, "issue_key")?)?;
        let mut query = Vec::new();
        push_csv_or_string(args, &mut query, "fields");
        self.get(&format!("/rest/api/3/issue/{issue_key}"), query)
            .await
    }

    /// POST `/rest/api/3/issue`.
    pub async fn create_issue(&self, args: &Value) -> Result<Value, String> {
        let fields = required_object(args, "fields")?;
        self.post("/rest/api/3/issue", json!({"fields": fields}))
            .await
    }

    /// PUT `/rest/api/3/issue/{issue_key}`.
    pub async fn update_issue(&self, args: &Value) -> Result<Value, String> {
        let issue_key = path_segment(&required_str(args, "issue_key")?)?;
        let mut body = serde_json::Map::new();
        if let Some(fields) = args.get("fields").filter(|v| v.is_object()) {
            body.insert("fields".into(), fields.clone());
        }
        if let Some(update) = args.get("update").filter(|v| v.is_object()) {
            body.insert("update".into(), update.clone());
        }
        if body.is_empty() {
            return Err("update_issue requires object field 'fields' or 'update'".into());
        }
        self.put(
            &format!("/rest/api/3/issue/{issue_key}"),
            Value::Object(body),
        )
        .await
    }

    /// POST `/rest/api/3/issue/{issue_key}/comment`.
    pub async fn comment_issue(&self, args: &Value) -> Result<Value, String> {
        let issue_key = path_segment(&required_str(args, "issue_key")?)?;
        let body = match args.get("body") {
            Some(Value::String(text)) => json!({"body": adf_doc(text)}),
            Some(Value::Object(_)) => json!({"body": args["body"].clone()}),
            _ => return Err("comment_issue requires string or object field 'body'".into()),
        };
        self.post(&format!("/rest/api/3/issue/{issue_key}/comment"), body)
            .await
    }

    /// POST `/rest/api/3/issue/{issue_key}/transitions`.
    pub async fn transition_issue(&self, args: &Value) -> Result<Value, String> {
        let issue_key = path_segment(&required_str(args, "issue_key")?)?;
        let transition_id = required_str(args, "transition_id")?;
        let mut body = json!({"transition": {"id": transition_id}});
        copy_optional(args, &mut body, "fields");
        copy_optional(args, &mut body, "update");
        self.post(&format!("/rest/api/3/issue/{issue_key}/transitions"), body)
            .await
    }

    /// GET `/rest/api/3/project/search`.
    pub async fn list_projects(&self, args: &Value) -> Result<Value, String> {
        let mut query = Vec::new();
        push_optional_string(args, &mut query, "query");
        push_limit(args, &mut query, "maxResults", 20, 100);
        push_u64(args, &mut query, "startAt");
        self.get("/rest/api/3/project/search", query).await
    }

    async fn get(&self, path: &str, query: Vec<(String, String)>) -> Result<Value, String> {
        let request = self.request(Method::GET, path).query(&query);
        decode_response(path, request.send().await).await
    }

    async fn post(&self, path: &str, body: Value) -> Result<Value, String> {
        let request = self.request(Method::POST, path).json(&body);
        decode_response(path, request.send().await).await
    }

    async fn put(&self, path: &str, body: Value) -> Result<Value, String> {
        let request = self.request(Method::PUT, path).json(&body);
        decode_response(path, request.send().await).await
    }

    fn request(&self, method: Method, path: &str) -> reqwest::RequestBuilder {
        self.http
            .request(method, format!("{}{}", self.config.base_url, path))
            .basic_auth(&self.config.email, Some(&self.config.api_token))
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
    }
}

/// Convert a tool result into MCP text content.
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
    let response = response.map_err(|e| format!("jira {path}: request failed: {e}"))?;
    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|e| format!("jira {path}: read response: {e}"))?;
    let parsed = if text.trim().is_empty() {
        json!({})
    } else {
        serde_json::from_str(&text)
            .map_err(|e| format!("jira {path}: decode response: {e}: {text}"))?
    };
    if status.is_success() {
        return Ok(parsed);
    }
    Err(format!("jira {path}: HTTP {status}: {parsed}"))
}

fn required_str(args: &Value, field: &str) -> Result<String, String> {
    args.get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("missing string field '{field}'"))
}

fn required_object(args: &Value, field: &str) -> Result<Value, String> {
    args.get(field)
        .filter(|value| value.is_object())
        .cloned()
        .ok_or_else(|| format!("missing object field '{field}'"))
}

fn copy_optional(args: &Value, body: &mut Value, field: &str) {
    if let Some(value) = args.get(field) {
        body[field] = value.clone();
    }
}

fn push_optional_string(args: &Value, query: &mut Vec<(String, String)>, field: &str) {
    if let Some(value) = args.get(field).and_then(Value::as_str) {
        if !value.trim().is_empty() {
            query.push((field.to_string(), value.to_string()));
        }
    }
}

fn push_csv_or_string(args: &Value, query: &mut Vec<(String, String)>, field: &str) {
    match args.get(field) {
        Some(Value::String(value)) if !value.trim().is_empty() => {
            query.push((field.to_string(), value.to_string()));
        }
        Some(Value::Array(values)) => {
            let joined = values
                .iter()
                .filter_map(Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .collect::<Vec<_>>()
                .join(",");
            if !joined.is_empty() {
                query.push((field.to_string(), joined));
            }
        }
        _ => {}
    }
}

fn push_u64(args: &Value, query: &mut Vec<(String, String)>, field: &str) {
    if let Some(value) = args.get(field).and_then(Value::as_u64) {
        query.push((field.to_string(), value.to_string()));
    }
}

fn push_limit(
    args: &Value,
    query: &mut Vec<(String, String)>,
    field: &str,
    default: u64,
    max: u64,
) {
    let value = args
        .get("limit")
        .or_else(|| args.get(field))
        .and_then(Value::as_u64)
        .unwrap_or(default)
        .clamp(1, max);
    query.push((field.to_string(), value.to_string()));
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
        Err(format!("unsafe Jira path segment: {raw}"))
    }
}

fn adf_doc(text: &str) -> Value {
    json!({
        "type": "doc",
        "version": 1,
        "content": [{
            "type": "paragraph",
            "content": [{"type": "text", "text": text}]
        }]
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[test]
    fn path_segments_are_strict() {
        assert_eq!(path_segment("ARGO-123").expect("valid key"), "ARGO-123");
        assert!(path_segment("../ARGO-123").is_err());
        assert!(path_segment("ARGO/123").is_err());
    }

    #[test]
    fn string_comments_convert_to_adf() {
        let doc = adf_doc("Looks good");
        assert_eq!(doc["type"], "doc");
        assert_eq!(doc["content"][0]["content"][0]["text"], "Looks good");
    }

    #[tokio::test]
    async fn list_issues_preserves_jql_query() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let addr = listener.local_addr().expect("addr");
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept");
            let mut buf = vec![0_u8; 2048];
            let n = socket.read(&mut buf).await.expect("read");
            let req = String::from_utf8_lossy(&buf[..n]).to_string();
            socket
                .write_all(b"HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: 13\r\n\r\n{\"issues\":[]}")
                .await
                .expect("write");
            req
        });
        let cfg =
            JiraConfig::new("op@example.com", "tok", format!("http://{addr}")).expect("config");
        let client = JiraClient::new(cfg).expect("client");
        let out = client
            .list_issues(&json!({"jql": "project = ARGO ORDER BY created", "limit": 7}))
            .await
            .expect("list issues");
        assert_eq!(out["issues"].as_array().expect("issues").len(), 0);
        let req = server.await.expect("server");
        assert!(req.starts_with("GET /rest/api/3/search?"));
        assert!(req.contains("jql=project+%3D+ARGO+ORDER+BY+created"));
        assert!(req.contains("maxResults=7"));
    }
}
