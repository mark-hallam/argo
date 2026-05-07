//! Thin GitHub Issues REST API client.

use crate::auth;
use argo_mcp::{CallToolResult, McpContent};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT};

const ACCEPT_GITHUB_JSON: &str = "application/vnd.github+json";
const API_VERSION_HEADER: &str = "X-GitHub-Api-Version";
const API_VERSION: &str = "2022-11-28";

/// GitHub Issues REST client authenticated with a bearer PAT.
#[derive(Clone)]
pub struct GithubIssuesClient {
    http: reqwest::Client,
    base_url: String,
    token: String,
}

impl GithubIssuesClient {
    /// Build from `GITHUB_TOKEN`.
    pub fn from_env() -> Result<Self, String> {
        Self::new(auth::token_from_env()?, auth::base_url_from_env())
    }

    /// Build with an explicit token and API base URL.
    pub fn new(token: String, base_url: String) -> Result<Self, String> {
        if token.trim().is_empty() {
            return Err(format!("missing {}", auth::GITHUB_TOKEN_ENV));
        }
        let http = reqwest::Client::builder()
            .user_agent(concat!(
                "argo-mcp-connector-github-issues/",
                env!("CARGO_PKG_VERSION")
            ))
            .default_headers(default_headers()?)
            .build()
            .map_err(|e| format!("github issues http client: {e}"))?;
        Ok(Self {
            http,
            base_url: base_url.trim_end_matches('/').to_string(),
            token,
        })
    }

    /// GET `/repos/{owner}/{repo}/issues`.
    pub async fn list_issues(&self, args: &serde_json::Value) -> Result<serde_json::Value, String> {
        let (owner, repo) = repo_path(args)?;
        let mut query = limit_query(args, 30, 100);
        push_str_or_default(args, &mut query, "state", "open");
        push_str(args, &mut query, "labels");
        push_str(args, &mut query, "since");
        self.get(&format!("/repos/{owner}/{repo}/issues"), query)
            .await
    }

    /// GET `/repos/{owner}/{repo}/issues/{issue_number}`.
    pub async fn get_issue(&self, args: &serde_json::Value) -> Result<serde_json::Value, String> {
        let (owner, repo) = repo_path(args)?;
        let number = required_u64(args, "issue_number")?;
        self.get(
            &format!("/repos/{owner}/{repo}/issues/{number}"),
            Vec::new(),
        )
        .await
    }

    /// POST `/repos/{owner}/{repo}/issues`.
    pub async fn create_issue(
        &self,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let (owner, repo) = repo_path(args)?;
        let mut body = serde_json::Map::new();
        body.insert("title".into(), required_value(args, "title")?);
        push_value(args, &mut body, "body");
        push_value(args, &mut body, "labels");
        self.post_json(
            &format!("/repos/{owner}/{repo}/issues"),
            serde_json::Value::Object(body),
        )
        .await
    }

    /// PATCH `/repos/{owner}/{repo}/issues/{issue_number}`.
    pub async fn update_issue(
        &self,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let (owner, repo) = repo_path(args)?;
        let number = required_u64(args, "issue_number")?;
        let mut body = serde_json::Map::new();
        for field in ["title", "body", "state", "labels"] {
            push_value(args, &mut body, field);
        }
        if body.is_empty() {
            return Err("github issues update_issue: no update fields supplied".into());
        }
        self.patch_json(
            &format!("/repos/{owner}/{repo}/issues/{number}"),
            serde_json::Value::Object(body),
        )
        .await
    }

    /// POST `/repos/{owner}/{repo}/issues/{issue_number}/comments`.
    pub async fn add_comment(&self, args: &serde_json::Value) -> Result<serde_json::Value, String> {
        let (owner, repo) = repo_path(args)?;
        let number = required_u64(args, "issue_number")?;
        self.post_json(
            &format!("/repos/{owner}/{repo}/issues/{number}/comments"),
            serde_json::json!({"body": required_str(args, "body")?}),
        )
        .await
    }

    /// Close an issue via PATCH state=`closed`.
    pub async fn close_issue(&self, args: &serde_json::Value) -> Result<serde_json::Value, String> {
        let (owner, repo) = repo_path(args)?;
        let number = required_u64(args, "issue_number")?;
        self.patch_json(
            &format!("/repos/{owner}/{repo}/issues/{number}"),
            serde_json::json!({"state": "closed"}),
        )
        .await
    }

    /// Add and/or remove labels on an issue.
    pub async fn label_issue(&self, args: &serde_json::Value) -> Result<serde_json::Value, String> {
        let (owner, repo) = repo_path(args)?;
        let number = required_u64(args, "issue_number")?;
        let add = optional_string_array(args, "add")?;
        let remove = optional_string_array(args, "remove")?;
        if add.is_empty() && remove.is_empty() {
            return Err("github issues label_issue: add or remove labels required".into());
        }
        let mut removed = Vec::new();
        let mut added = serde_json::Value::Null;
        if !add.is_empty() {
            added = self
                .post_json(
                    &format!("/repos/{owner}/{repo}/issues/{number}/labels"),
                    serde_json::json!({"labels": add}),
                )
                .await?;
        }
        for label in remove {
            let encoded = encode_path_segment(&label);
            self.delete(&format!(
                "/repos/{owner}/{repo}/issues/{number}/labels/{encoded}"
            ))
            .await?;
            removed.push(label);
        }
        Ok(serde_json::json!({"added": added, "removed": removed}))
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
            .map_err(|e| format!("github issues GET {path}: {e}"))?;
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
            .map_err(|e| format!("github issues POST {path}: {e}"))?;
        decode_response("POST", path, resp).await
    }

    async fn patch_json(
        &self,
        path: &str,
        body: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let resp = self
            .http
            .patch(format!("{}{}", self.base_url, path))
            .bearer_auth(&self.token)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("github issues PATCH {path}: {e}"))?;
        decode_response("PATCH", path, resp).await
    }

    async fn delete(&self, path: &str) -> Result<serde_json::Value, String> {
        let resp = self
            .http
            .delete(format!("{}{}", self.base_url, path))
            .bearer_auth(&self.token)
            .send()
            .await
            .map_err(|e| format!("github issues DELETE {path}: {e}"))?;
        decode_response("DELETE", path, resp).await
    }
}

/// Convert a GitHub Issues result into MCP text content.
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

fn default_headers() -> Result<HeaderMap, String> {
    let mut headers = HeaderMap::new();
    headers.insert(ACCEPT, HeaderValue::from_static(ACCEPT_GITHUB_JSON));
    headers.insert(API_VERSION_HEADER, HeaderValue::from_static(API_VERSION));
    Ok(headers)
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
        .map_err(|e| format!("github issues {method} {path}: read response: {e}"))?;
    let parsed = if text.trim().is_empty() {
        serde_json::json!({})
    } else {
        serde_json::from_str(&text)
            .map_err(|e| format!("github issues {method} {path}: decode response: {e}: {text}"))?
    };
    if status.is_success() {
        Ok(parsed)
    } else {
        Err(format!(
            "github issues {method} {path}: HTTP {status}: {parsed}"
        ))
    }
}

fn repo_path(args: &serde_json::Value) -> Result<(String, String), String> {
    Ok((
        clean_segment(&required_str(args, "owner")?)?,
        clean_segment(&required_str(args, "repo")?)?,
    ))
}

fn required_value(args: &serde_json::Value, field: &str) -> Result<serde_json::Value, String> {
    args.get(field)
        .filter(|v| !v.is_null())
        .cloned()
        .ok_or_else(|| format!("missing field '{field}'"))
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

fn push_value(
    args: &serde_json::Value,
    out: &mut serde_json::Map<String, serde_json::Value>,
    field: &str,
) {
    if let Some(value) = args.get(field).filter(|v| !v.is_null()) {
        out.insert(field.into(), value.clone());
    }
}

fn push_str(args: &serde_json::Value, out: &mut Vec<(String, String)>, field: &str) {
    if let Some(value) = args.get(field).and_then(|v| v.as_str()) {
        out.push((field.into(), value.to_string()));
    }
}

fn push_str_or_default(
    args: &serde_json::Value,
    out: &mut Vec<(String, String)>,
    field: &str,
    default: &str,
) {
    let value = args.get(field).and_then(|v| v.as_str()).unwrap_or(default);
    out.push((field.into(), value.to_string()));
}

fn limit_query(args: &serde_json::Value, default: u64, max: u64) -> Vec<(String, String)> {
    vec![(
        "per_page".into(),
        args.get("per_page")
            .or_else(|| args.get("limit"))
            .and_then(|v| v.as_u64())
            .unwrap_or(default)
            .clamp(1, max)
            .to_string(),
    )]
}

fn optional_string_array(args: &serde_json::Value, field: &str) -> Result<Vec<String>, String> {
    let Some(value) = args.get(field) else {
        return Ok(Vec::new());
    };
    let Some(values) = value.as_array() else {
        return Err(format!("field '{field}' must be an array of strings"));
    };
    values
        .iter()
        .map(|v| {
            v.as_str()
                .filter(|s| !s.trim().is_empty())
                .map(str::to_string)
                .ok_or_else(|| format!("field '{field}' must contain only strings"))
        })
        .collect()
}

fn clean_segment(raw: &str) -> Result<String, String> {
    if !raw.is_empty()
        && raw
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
    {
        Ok(raw.to_string())
    } else {
        Err(format!("unsafe GitHub path segment: {raw}"))
    }
}

fn encode_path_segment(raw: &str) -> String {
    raw.bytes()
        .flat_map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                vec![b as char]
            }
            _ => format!("%{b:02X}").chars().collect(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    fn client(base_url: String) -> GithubIssuesClient {
        GithubIssuesClient::new("ghp_token".into(), base_url).unwrap()
    }

    fn serve_once(assert_req: impl FnOnce(&str) + Send + 'static, body: &'static str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = [0; 4096];
            let n = stream.read(&mut buf).unwrap();
            let req = String::from_utf8_lossy(&buf[..n]);
            assert_req(&req);
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(response.as_bytes()).unwrap();
        });
        format!("http://{addr}")
    }

    #[tokio::test]
    async fn list_issues_sets_required_github_headers() {
        let base = serve_once(
            |req| {
                assert!(req.contains("GET /repos/octo/repo/issues?per_page=30&state=open"));
                assert!(req
                    .to_ascii_lowercase()
                    .contains("authorization: bearer ghp_token"));
                assert!(req
                    .to_ascii_lowercase()
                    .contains("user-agent: argo-mcp-connector-github-issues/"));
                assert!(req
                    .to_ascii_lowercase()
                    .contains("accept: application/vnd.github+json"));
            },
            r#"[{"number":1,"title":"bug"}]"#,
        );
        let value = client(base)
            .list_issues(&serde_json::json!({"owner": "octo", "repo": "repo"}))
            .await
            .unwrap();
        assert_eq!(value[0]["number"], 1);
    }

    #[tokio::test]
    async fn create_issue_posts_json_body() {
        let base = serve_once(
            |req| {
                assert!(req.contains("POST /repos/octo/repo/issues"));
                assert!(req.contains(r#""title":"Bug""#));
                assert!(req.contains(r#""labels":["triage"]"#));
            },
            r#"{"number":2,"title":"Bug"}"#,
        );
        let value = client(base)
            .create_issue(&serde_json::json!({
                "owner": "octo", "repo": "repo", "title": "Bug", "labels": ["triage"]
            }))
            .await
            .unwrap();
        assert_eq!(value["number"], 2);
    }

    #[tokio::test]
    async fn close_issue_patches_closed_state() {
        let base = serve_once(
            |req| {
                assert!(req.contains("PATCH /repos/octo/repo/issues/7"));
                assert!(req.contains(r#""state":"closed""#));
            },
            r#"{"number":7,"state":"closed"}"#,
        );
        let value = client(base)
            .close_issue(&serde_json::json!({"owner": "octo", "repo": "repo", "issue_number": 7}))
            .await
            .unwrap();
        assert_eq!(value["state"], "closed");
    }

    #[test]
    fn label_paths_are_percent_encoded() {
        assert_eq!(encode_path_segment("needs review"), "needs%20review");
        assert_eq!(encode_path_segment("a/b"), "a%2Fb");
    }
}
