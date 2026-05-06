//! Thin Asana API HTTP client.
use crate::auth;
use argo_mcp::{CallToolResult, McpContent};

/// Asana API client authenticated with a personal access token.
#[derive(Clone)]
pub struct AsanaClient {
    http: reqwest::Client,
    base_url: String,
    pat: String,
}

impl AsanaClient {
    /// Build from `ASANA_PAT`.
    pub fn from_env() -> Result<Self, String> {
        Self::new(auth::pat_from_env()?, auth::base_url_from_env())
    }

    /// Build with an explicit token and base URL.
    pub fn new(pat: String, base_url: String) -> Result<Self, String> {
        if pat.trim().is_empty() {
            return Err(format!("missing {}", auth::ASANA_PAT_ENV));
        }
        let http = reqwest::Client::builder()
            .user_agent(concat!(
                "argo-mcp-connector-asana/",
                env!("CARGO_PKG_VERSION")
            ))
            .build()
            .map_err(|e| format!("asana http client: {e}"))?;
        Ok(Self {
            http,
            base_url: base_url.trim_end_matches('/').to_string(),
            pat,
        })
    }

    /// GET `/workspaces`.
    pub async fn list_workspaces(
        &self,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        self.get("/workspaces", common_query(args)).await
    }

    /// GET `/projects`.
    pub async fn list_projects(
        &self,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let mut query = common_query(args);
        push_str(args, &mut query, "workspace");
        push_str(args, &mut query, "team");
        push_bool(args, &mut query, "archived");
        self.get("/projects", query).await
    }

    /// GET `/tasks` scoped by project.
    pub async fn list_tasks(&self, args: &serde_json::Value) -> Result<serde_json::Value, String> {
        let mut query = common_query(args);
        query.push(("project".into(), required_str(args, "project")?));
        push_str(args, &mut query, "completed_since");
        self.get("/tasks", query).await
    }

    /// GET `/tasks/{task_gid}`.
    pub async fn get_task(&self, args: &serde_json::Value) -> Result<serde_json::Value, String> {
        let gid = clean_gid(&required_str(args, "task_gid")?)?;
        self.get(&format!("/tasks/{gid}"), common_query(args)).await
    }

    /// POST `/tasks`.
    pub async fn create_task(&self, args: &serde_json::Value) -> Result<serde_json::Value, String> {
        let mut data = serde_json::Map::new();
        data.insert("name".into(), required_str(args, "name")?.into());
        copy_value(args, &mut data, "workspace");
        copy_value(args, &mut data, "projects");
        copy_value(args, &mut data, "assignee");
        copy_value(args, &mut data, "due_on");
        copy_value(args, &mut data, "notes");
        self.post_json("/tasks", serde_json::json!({ "data": data }))
            .await
    }

    /// PUT `/tasks/{task_gid}`.
    pub async fn update_task(&self, args: &serde_json::Value) -> Result<serde_json::Value, String> {
        let gid = clean_gid(&required_str(args, "task_gid")?)?;
        let mut data = serde_json::Map::new();
        copy_value(args, &mut data, "name");
        copy_value(args, &mut data, "assignee");
        copy_value(args, &mut data, "due_on");
        copy_value(args, &mut data, "notes");
        copy_value(args, &mut data, "completed");
        if data.is_empty() {
            return Err("update_task needs at least one mutable field".into());
        }
        self.put_json(
            &format!("/tasks/{gid}"),
            serde_json::json!({ "data": data }),
        )
        .await
    }

    /// Mark a task complete via `PUT /tasks/{task_gid}`.
    pub async fn complete_task(
        &self,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let gid = clean_gid(&required_str(args, "task_gid")?)?;
        self.put_json(
            &format!("/tasks/{gid}"),
            serde_json::json!({ "data": { "completed": true } }),
        )
        .await
    }

    /// POST `/tasks/{task_gid}/stories`.
    pub async fn add_comment(&self, args: &serde_json::Value) -> Result<serde_json::Value, String> {
        let gid = clean_gid(&required_str(args, "task_gid")?)?;
        let text = required_str(args, "text")?;
        self.post_json(
            &format!("/tasks/{gid}/stories"),
            serde_json::json!({ "data": { "text": text } }),
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
            .bearer_auth(&self.pat)
            .query(&query)
            .send()
            .await
            .map_err(|e| format!("asana GET {path}: {e}"))?;
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
            .bearer_auth(&self.pat)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("asana POST {path}: {e}"))?;
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
            .bearer_auth(&self.pat)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("asana PUT {path}: {e}"))?;
        decode_response("PUT", path, resp).await
    }
}

/// Convert an Asana tool result into MCP text content.
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
        .map_err(|e| format!("asana {method} {path}: read response: {e}"))?;
    let parsed = if text.trim().is_empty() {
        serde_json::json!({})
    } else {
        serde_json::from_str(&text)
            .map_err(|e| format!("asana {method} {path}: decode response: {e}: {text}"))?
    };
    if status.is_success() {
        Ok(parsed)
    } else {
        Err(format!("asana {method} {path}: HTTP {status}: {parsed}"))
    }
}

fn common_query(args: &serde_json::Value) -> Vec<(String, String)> {
    let mut query = Vec::new();
    push_u64(args, &mut query, "limit");
    push_str(args, &mut query, "offset");
    push_str(args, &mut query, "opt_fields");
    query
}

fn required_str(args: &serde_json::Value, field: &str) -> Result<String, String> {
    args.get(field)
        .and_then(|v| v.as_str())
        .filter(|v| !v.trim().is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("missing string field '{field}'"))
}

fn copy_value(
    args: &serde_json::Value,
    out: &mut serde_json::Map<String, serde_json::Value>,
    field: &str,
) {
    if let Some(value) = args.get(field) {
        out.insert(field.into(), value.clone());
    }
}

fn push_str(args: &serde_json::Value, out: &mut Vec<(String, String)>, field: &str) {
    if let Some(value) = args.get(field).and_then(|v| v.as_str()) {
        out.push((field.into(), value.to_string()));
    }
}

fn push_u64(args: &serde_json::Value, out: &mut Vec<(String, String)>, field: &str) {
    if let Some(value) = args.get(field).and_then(|v| v.as_u64()) {
        out.push((field.into(), value.clamp(1, 100).to_string()));
    }
}

fn push_bool(args: &serde_json::Value, out: &mut Vec<(String, String)>, field: &str) {
    if let Some(value) = args.get(field).and_then(|v| v.as_bool()) {
        out.push((field.into(), value.to_string()));
    }
}

fn clean_gid(raw: &str) -> Result<String, String> {
    if !raw.is_empty()
        && raw
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
    {
        Ok(raw.to_string())
    } else {
        Err(format!("unsafe Asana gid: {raw}"))
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
            assert!(req.contains("authorization: bearer pat_test"));
            let status = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n";
            let response = format!("{status}Content-Length: {}\r\n\r\n{body}", body.len());
            stream.write_all(response.as_bytes()).unwrap();
        });
        format!("http://{addr}")
    }

    #[tokio::test]
    async fn list_workspaces_uses_bearer_auth() {
        let base = serve_once(r#"{"data":[]}"#);
        let client = AsanaClient::new("pat_test".into(), base).unwrap();
        let out = client
            .list_workspaces(&serde_json::json!({}))
            .await
            .unwrap();
        assert_eq!(out["data"].as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn create_task_posts_json() {
        let base = serve_once(r#"{"data":{"gid":"120","name":"Draft brief"}}"#);
        let client = AsanaClient::new("pat_test".into(), base).unwrap();
        let out = client
            .create_task(&serde_json::json!({
                "name": "Draft brief",
                "workspace": "w1",
                "projects": ["p1"]
            }))
            .await
            .unwrap();
        assert_eq!(out["data"]["gid"], "120");
    }

    #[test]
    fn gids_are_path_safe() {
        assert_eq!(clean_gid("120_abc-DEF").unwrap(), "120_abc-DEF");
        assert!(clean_gid("../120").is_err());
    }
}
