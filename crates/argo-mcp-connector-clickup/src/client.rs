//! Thin ClickUp API HTTP client.
use crate::auth;
use argo_mcp::{CallToolResult, McpContent};
use serde_json::{Map, Value};

const CREATE_TASK_FIELDS: &[&str] = &[
    "name",
    "description",
    "markdown_content",
    "assignees",
    "archived",
    "group_assignees",
    "tags",
    "status",
    "priority",
    "due_date",
    "due_date_time",
    "time_estimate",
    "start_date",
    "start_date_time",
    "points",
    "notify_all",
    "parent",
    "links_to",
    "check_required_custom_fields",
    "custom_fields",
    "custom_item_id",
];

const UPDATE_TASK_FIELDS: &[&str] = &[
    "name",
    "description",
    "markdown_content",
    "status",
    "priority",
    "due_date",
    "due_date_time",
    "time_estimate",
    "start_date",
    "start_date_time",
    "points",
    "assignees",
    "group_assignees",
    "watchers",
    "archived",
    "custom_item_id",
];

/// ClickUp API client authenticated with a personal token.
#[derive(Clone)]
pub struct ClickUpClient {
    http: reqwest::Client,
    base_url: String,
    token: String,
}

impl ClickUpClient {
    /// Build from `CLICKUP_TOKEN`.
    pub fn from_env() -> Result<Self, String> {
        Self::new(auth::token_from_env()?, auth::base_url_from_env())
    }

    /// Build with an explicit token and base URL.
    pub fn new(token: String, base_url: String) -> Result<Self, String> {
        if token.trim().is_empty() {
            return Err(format!("missing {}", auth::CLICKUP_TOKEN_ENV));
        }
        let http = reqwest::Client::builder()
            .user_agent(concat!(
                "argo-mcp-connector-clickup/",
                env!("CARGO_PKG_VERSION")
            ))
            .build()
            .map_err(|e| format!("clickup http client: {e}"))?;
        Ok(Self {
            http,
            base_url: base_url.trim_end_matches('/').to_string(),
            token,
        })
    }

    /// GET `/team`.
    pub async fn list_workspaces(&self, _args: &Value) -> Result<Value, String> {
        self.get("/team", Vec::new()).await
    }

    /// GET `/team/{workspace_id}/space`.
    pub async fn list_spaces(&self, args: &Value) -> Result<Value, String> {
        let workspace_id = required_any_id(args, &["workspace_id", "team_id"])?;
        let mut query = Vec::new();
        push_bool(args, &mut query, "archived");
        self.get(&format!("/team/{workspace_id}/space"), query)
            .await
    }

    /// GET `/space/{space_id}/list`.
    pub async fn list_lists(&self, args: &Value) -> Result<Value, String> {
        let space_id = required_id(args, "space_id")?;
        let mut query = Vec::new();
        push_bool(args, &mut query, "archived");
        self.get(&format!("/space/{space_id}/list"), query).await
    }

    /// GET `/list/{list_id}/task`.
    pub async fn list_tasks(&self, args: &Value) -> Result<Value, String> {
        let list_id = required_id(args, "list_id")?;
        self.get(&format!("/list/{list_id}/task"), task_list_query(args))
            .await
    }

    /// GET `/task/{task_id}`.
    pub async fn get_task(&self, args: &Value) -> Result<Value, String> {
        let task_id = required_id(args, "task_id")?;
        self.get(&format!("/task/{task_id}"), task_query(args))
            .await
    }

    /// POST `/list/{list_id}/task`.
    pub async fn create_task(&self, args: &Value) -> Result<Value, String> {
        let list_id = required_id(args, "list_id")?;
        let mut body = copy_body(args, CREATE_TASK_FIELDS);
        body.insert("name".into(), required_str(args, "name")?.into());
        self.post_json(&format!("/list/{list_id}/task"), Value::Object(body))
            .await
    }

    /// PUT `/task/{task_id}`.
    pub async fn update_task(&self, args: &Value) -> Result<Value, String> {
        let task_id = required_id(args, "task_id")?;
        let body = copy_body(args, UPDATE_TASK_FIELDS);
        if body.is_empty() {
            return Err("update_task needs at least one mutable field".into());
        }
        self.put_json(
            &format!("/task/{task_id}"),
            task_query(args),
            Value::Object(body),
        )
        .await
    }

    /// POST `/task/{task_id}/comment`.
    pub async fn add_comment(&self, args: &Value) -> Result<Value, String> {
        let task_id = required_id(args, "task_id")?;
        let mut body = Map::new();
        body.insert(
            "comment_text".into(),
            required_any_str(args, &["comment_text", "text", "body"])?.into(),
        );
        body.insert(
            "notify_all".into(),
            args.get("notify_all")
                .and_then(Value::as_bool)
                .unwrap_or(false)
                .into(),
        );
        copy_value(args, &mut body, "assignee");
        copy_value(args, &mut body, "group_assignee");
        self.post_json_with_query(
            &format!("/task/{task_id}/comment"),
            task_query(args),
            Value::Object(body),
        )
        .await
    }

    async fn get(&self, path: &str, query: Vec<(String, String)>) -> Result<Value, String> {
        let resp = self
            .http
            .get(format!("{}{}", self.base_url, path))
            .bearer_auth(&self.token)
            .query(&query)
            .send()
            .await
            .map_err(|e| format!("clickup GET {path}: {e}"))?;
        decode_response("GET", path, resp).await
    }

    async fn post_json(&self, path: &str, body: Value) -> Result<Value, String> {
        self.post_json_with_query(path, Vec::new(), body).await
    }

    async fn post_json_with_query(
        &self,
        path: &str,
        query: Vec<(String, String)>,
        body: Value,
    ) -> Result<Value, String> {
        let resp = self
            .http
            .post(format!("{}{}", self.base_url, path))
            .bearer_auth(&self.token)
            .query(&query)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("clickup POST {path}: {e}"))?;
        decode_response("POST", path, resp).await
    }

    async fn put_json(
        &self,
        path: &str,
        query: Vec<(String, String)>,
        body: Value,
    ) -> Result<Value, String> {
        let resp = self
            .http
            .put(format!("{}{}", self.base_url, path))
            .bearer_auth(&self.token)
            .query(&query)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("clickup PUT {path}: {e}"))?;
        decode_response("PUT", path, resp).await
    }
}

/// Convert a ClickUp tool result into MCP text content.
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
    method: &str,
    path: &str,
    resp: reqwest::Response,
) -> Result<Value, String> {
    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| format!("clickup {method} {path}: read response: {e}"))?;
    let parsed = if text.trim().is_empty() {
        serde_json::json!({})
    } else {
        serde_json::from_str(&text)
            .map_err(|e| format!("clickup {method} {path}: decode response: {e}: {text}"))?
    };
    if status.is_success() {
        Ok(parsed)
    } else {
        Err(format!("clickup {method} {path}: HTTP {status}: {parsed}"))
    }
}

fn task_list_query(args: &Value) -> Vec<(String, String)> {
    let mut query = Vec::new();
    for field in [
        "archived",
        "include_markdown_description",
        "reverse",
        "subtasks",
        "include_closed",
        "include_timl",
    ] {
        push_bool(args, &mut query, field);
    }
    for field in [
        "page",
        "due_date_gt",
        "due_date_lt",
        "date_created_gt",
        "date_created_lt",
    ] {
        push_u64(args, &mut query, field);
    }
    for field in [
        "date_updated_gt",
        "date_updated_lt",
        "date_done_gt",
        "date_done_lt",
    ] {
        push_u64(args, &mut query, field);
    }
    push_str(args, &mut query, "order_by");
    push_array(args, &mut query, "statuses", "statuses[]");
    push_array(args, &mut query, "assignees", "assignees[]");
    push_array(args, &mut query, "watchers", "watchers[]");
    push_array(args, &mut query, "tags", "tags[]");
    push_jsonish(args, &mut query, "custom_fields");
    query
}

fn task_query(args: &Value) -> Vec<(String, String)> {
    let mut query = Vec::new();
    push_bool(args, &mut query, "custom_task_ids");
    push_str(args, &mut query, "team_id");
    push_bool(args, &mut query, "include_subtasks");
    push_bool(args, &mut query, "include_markdown_description");
    push_jsonish(args, &mut query, "custom_fields");
    query
}

fn copy_body(args: &Value, fields: &[&str]) -> Map<String, Value> {
    let mut body = Map::new();
    for field in fields {
        copy_value(args, &mut body, field);
    }
    body
}

fn copy_value(args: &Value, out: &mut Map<String, Value>, field: &str) {
    if let Some(value) = args.get(field) {
        out.insert(field.into(), value.clone());
    }
}

fn required_id(args: &Value, field: &str) -> Result<String, String> {
    clean_id(&required_str(args, field)?)
}

fn required_any_id(args: &Value, fields: &[&str]) -> Result<String, String> {
    clean_id(&required_any_str(args, fields)?)
}

fn required_str(args: &Value, field: &str) -> Result<String, String> {
    args.get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("missing string field '{field}'"))
}

fn required_any_str(args: &Value, fields: &[&str]) -> Result<String, String> {
    fields
        .iter()
        .find_map(|field| required_str(args, field).ok())
        .ok_or_else(|| format!("missing string field '{}'", fields[0]))
}

fn clean_id(raw: &str) -> Result<String, String> {
    if raw
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
    {
        Ok(raw.to_string())
    } else {
        Err(format!("unsafe ClickUp id: {raw}"))
    }
}

fn push_bool(args: &Value, query: &mut Vec<(String, String)>, field: &str) {
    if let Some(value) = args.get(field).and_then(Value::as_bool) {
        query.push((field.into(), value.to_string()));
    }
}

fn push_u64(args: &Value, query: &mut Vec<(String, String)>, field: &str) {
    if let Some(value) = args.get(field).and_then(Value::as_u64) {
        query.push((field.into(), value.to_string()));
    }
}

fn push_str(args: &Value, query: &mut Vec<(String, String)>, field: &str) {
    if let Some(value) = args.get(field).and_then(Value::as_str) {
        query.push((field.into(), value.to_string()));
    }
}

fn push_array(args: &Value, query: &mut Vec<(String, String)>, source: &str, target: &str) {
    if let Some(values) = args.get(source).and_then(Value::as_array) {
        for value in values {
            if let Some(value) = value.as_str() {
                query.push((target.into(), value.to_string()));
            } else if let Some(value) = value.as_u64() {
                query.push((target.into(), value.to_string()));
            }
        }
    }
}

fn push_jsonish(args: &Value, query: &mut Vec<(String, String)>, field: &str) {
    if let Some(value) = args.get(field) {
        if let Some(value) = value.as_str() {
            query.push((field.into(), value.to_string()));
        } else if value.is_array() || value.is_object() {
            query.push((field.into(), value.to_string()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    fn serve_once<F>(body: &'static str, status_line: &'static str, assert_req: F) -> String
    where
        F: FnOnce(&str) + Send + 'static,
    {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr");
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut buf = [0; 8192];
            let n = stream.read(&mut buf).expect("read");
            let req = String::from_utf8_lossy(&buf[..n]).to_string();
            assert!(req
                .to_ascii_lowercase()
                .contains("authorization: bearer token_test"));
            assert_req(&req);
            let response = format!(
                "{status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(response.as_bytes()).expect("write");
        });
        format!("http://{addr}/api/v2")
    }

    fn request_json(req: &str) -> Value {
        let body = req.split("\r\n\r\n").nth(1).expect("http body");
        serde_json::from_str(body).expect("json body")
    }

    #[tokio::test]
    async fn list_spaces_uses_bearer_auth_and_workspace_path() {
        let base = serve_once(r#"{"spaces":[]}"#, "HTTP/1.1 200 OK", |req| {
            assert!(req.starts_with("GET /api/v2/team/123/space?archived=true "));
        });
        let client = ClickUpClient::new("token_test".into(), base).expect("client");
        let out = client
            .list_spaces(&serde_json::json!({"workspace_id": "123", "archived": true}))
            .await
            .expect("list spaces");
        assert!(out["spaces"].is_array());
    }

    #[tokio::test]
    async fn list_tasks_repeats_array_filters() {
        let base = serve_once(r#"{"tasks":[]}"#, "HTTP/1.1 200 OK", |req| {
            assert!(req.starts_with("GET /api/v2/list/456/task?"));
            assert!(req.contains("statuses%5B%5D=in+progress"));
            assert!(req.contains("tags%5B%5D=urgent"));
        });
        let client = ClickUpClient::new("token_test".into(), base).expect("client");
        client
            .list_tasks(&serde_json::json!({
                "list_id": "456",
                "statuses": ["in progress"],
                "tags": ["urgent"]
            }))
            .await
            .expect("list tasks");
    }

    #[tokio::test]
    async fn create_task_posts_json_body() {
        let base = serve_once(r#"{"id":"task_1"}"#, "HTTP/1.1 200 OK", |req| {
            assert!(req.starts_with("POST /api/v2/list/456/task "));
            let body = request_json(req);
            assert_eq!(body["name"], "Ship connector");
            assert_eq!(body["priority"], 3);
            assert_eq!(body["assignees"][0], 183);
        });
        let client = ClickUpClient::new("token_test".into(), base).expect("client");
        let out = client
            .create_task(&serde_json::json!({
                "list_id": "456",
                "name": "Ship connector",
                "priority": 3,
                "assignees": [183]
            }))
            .await
            .expect("create task");
        assert_eq!(out["id"], "task_1");
    }

    #[tokio::test]
    async fn update_task_rejects_empty_patch() {
        let client =
            ClickUpClient::new("token_test".into(), "http://127.0.0.1:9".into()).expect("client");
        let err = client
            .update_task(&serde_json::json!({"task_id": "abc123"}))
            .await
            .unwrap_err();
        assert!(err.contains("at least one mutable field"));
    }
}
