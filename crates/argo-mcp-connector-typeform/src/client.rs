//! Thin Typeform REST API client.

use crate::auth::{TypeformConfig, TYPEFORM_TOKEN_ENV};
use argo_mcp::{CallToolResult, McpContent};

/// Typeform REST client authenticated with a bearer token.
#[derive(Clone)]
pub struct TypeformClient {
    http: reqwest::Client,
    base_url: String,
    token: String,
}

impl TypeformClient {
    /// Build from `TYPEFORM_TOKEN`.
    pub fn from_env() -> Result<Self, String> {
        let cfg = TypeformConfig::from_env()?;
        Self::new(cfg.token, cfg.base_url)
    }

    /// Build with explicit personal access token and REST API base URL.
    pub fn new(token: String, base_url: String) -> Result<Self, String> {
        if token.trim().is_empty() {
            return Err(format!("missing {TYPEFORM_TOKEN_ENV}"));
        }
        let http = reqwest::Client::builder()
            .user_agent(concat!(
                "argo-mcp-connector-typeform/",
                env!("CARGO_PKG_VERSION")
            ))
            .build()
            .map_err(|e| format!("typeform http client: {e}"))?;
        Ok(Self {
            http,
            base_url: base_url.trim_end_matches('/').to_string(),
            token,
        })
    }

    /// GET `/forms`.
    pub async fn list_forms(&self, args: &serde_json::Value) -> Result<serde_json::Value, String> {
        self.get("/forms", forms_query(args)).await
    }

    /// GET `/forms/{form_id}`.
    pub async fn get_form(&self, args: &serde_json::Value) -> Result<serde_json::Value, String> {
        let form_id = clean_segment(&required_str(args, "form_id")?)?;
        self.get(&format!("/forms/{form_id}"), Vec::new()).await
    }

    /// GET `/forms/{form_id}/responses`.
    pub async fn list_responses(
        &self,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let form_id = clean_segment(&required_str(args, "form_id")?)?;
        self.get(
            &format!("/forms/{form_id}/responses"),
            responses_query(args),
        )
        .await
    }

    /// GET `/forms/{form_id}/responses/{response_id}`.
    pub async fn get_response(
        &self,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let form_id = clean_segment(&required_str(args, "form_id")?)?;
        let response_id = clean_segment(&required_str(args, "response_id")?)?;
        self.get(
            &format!("/forms/{form_id}/responses/{response_id}"),
            Vec::new(),
        )
        .await
    }

    /// POST `/forms` with a complete Typeform form definition.
    pub async fn create_form(
        &self,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let form = required_object(args, "form")?;
        self.post_json("/forms", form).await
    }

    /// PUT `/forms/{form_id}` with a complete Typeform form definition.
    pub async fn update_form(
        &self,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let form_id = clean_segment(&required_str(args, "form_id")?)?;
        let form = required_object(args, "form")?;
        self.put_json(&format!("/forms/{form_id}"), form).await
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
            .map_err(|e| format!("typeform GET {path}: {e}"))?;
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
            .map_err(|e| format!("typeform POST {path}: {e}"))?;
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
            .bearer_auth(&self.token)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("typeform PUT {path}: {e}"))?;
        decode_response("PUT", path, resp).await
    }
}

/// Convert a Typeform result into MCP text content.
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
        .map_err(|e| format!("typeform {method} {path}: read response: {e}"))?;
    let parsed = if text.trim().is_empty() {
        serde_json::json!({})
    } else {
        serde_json::from_str(&text)
            .map_err(|e| format!("typeform {method} {path}: decode response: {e}: {text}"))?
    };
    if status.is_success() {
        Ok(parsed)
    } else {
        Err(format!("typeform {method} {path}: HTTP {status}: {parsed}"))
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

fn forms_query(args: &serde_json::Value) -> Vec<(String, String)> {
    let mut query = vec![("page_size".into(), bounded_page_size(args).to_string())];
    push_optional_str(args, &mut query, "search");
    push_optional_str(args, &mut query, "workspace_id");
    query
}

fn responses_query(args: &serde_json::Value) -> Vec<(String, String)> {
    let mut query = Vec::new();
    push_optional_u64(args, &mut query, "page_size");
    push_optional_str(args, &mut query, "since");
    push_optional_str(args, &mut query, "until");
    query
}

fn bounded_page_size(args: &serde_json::Value) -> u64 {
    args.get("page_size")
        .and_then(|v| v.as_u64())
        .unwrap_or(25)
        .clamp(1, 1000)
}

fn push_optional_str(
    args: &serde_json::Value,
    query: &mut Vec<(String, String)>,
    field: &'static str,
) {
    if let Some(value) = args
        .get(field)
        .and_then(|v| v.as_str())
        .filter(|v| !v.trim().is_empty())
    {
        query.push((field.into(), value.to_string()));
    }
}

fn push_optional_u64(
    args: &serde_json::Value,
    query: &mut Vec<(String, String)>,
    field: &'static str,
) {
    if let Some(value) = args.get(field).and_then(|v| v.as_u64()) {
        query.push((field.into(), value.to_string()));
    }
}

fn clean_segment(raw: &str) -> Result<String, String> {
    if !raw.is_empty()
        && raw
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
    {
        Ok(raw.to_string())
    } else {
        Err(format!("unsafe Typeform segment: {raw}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    fn client(base_url: String) -> TypeformClient {
        TypeformClient::new("typeform_token".into(), base_url).unwrap()
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
    async fn list_forms_uses_bearer_auth_and_query() {
        let base = serve_once(
            |req| {
                assert!(req.contains("GET /forms?page_size=10&search=demo"));
                assert!(req
                    .to_ascii_lowercase()
                    .contains("authorization: bearer typeform_token"));
            },
            r#"{"items":[{"id":"form1"}]}"#,
        );
        let value = client(base)
            .list_forms(&serde_json::json!({"page_size": 10, "search": "demo"}))
            .await
            .unwrap();
        assert_eq!(value["items"][0]["id"], "form1");
    }

    #[tokio::test]
    async fn get_form_and_response_use_read_endpoints() {
        let form_base = serve_once(
            |req| assert!(req.contains("GET /forms/form1")),
            r#"{"id":"form1"}"#,
        );
        let form = client(form_base)
            .get_form(&serde_json::json!({"form_id": "form1"}))
            .await
            .unwrap();
        assert_eq!(form["id"], "form1");

        let response_base = serve_once(
            |req| assert!(req.contains("GET /forms/form1/responses/resp1")),
            r#"{"response_id":"resp1"}"#,
        );
        let response = client(response_base)
            .get_response(&serde_json::json!({
                "form_id": "form1",
                "response_id": "resp1"
            }))
            .await
            .unwrap();
        assert_eq!(response["response_id"], "resp1");
    }

    #[tokio::test]
    async fn list_responses_passes_window_through() {
        let base = serve_once(
            |req| {
                assert!(req.contains(
                    "GET /forms/form1/responses?page_size=100&since=2026-01-01&until=2026-01-31"
                ));
            },
            r#"{"items":[{"response_id":"resp1"}]}"#,
        );
        let value = client(base)
            .list_responses(&serde_json::json!({
                "form_id": "form1",
                "page_size": 100,
                "since": "2026-01-01",
                "until": "2026-01-31"
            }))
            .await
            .unwrap();
        assert_eq!(value["items"][0]["response_id"], "resp1");
    }

    #[tokio::test]
    async fn create_and_update_form_send_full_body() {
        let create_base = serve_once(
            |req| {
                assert!(req.contains("POST /forms"));
                assert!(req.contains(r#""title":"Demo""#));
            },
            r#"{"id":"form1"}"#,
        );
        let created = client(create_base)
            .create_form(&serde_json::json!({"form": {"title": "Demo"}}))
            .await
            .unwrap();
        assert_eq!(created["id"], "form1");

        let update_base = serve_once(
            |req| {
                assert!(req.contains("PUT /forms/form1"));
                assert!(req.contains(r#""title":"Updated""#));
            },
            r#"{"id":"form1","title":"Updated"}"#,
        );
        let updated = client(update_base)
            .update_form(&serde_json::json!({
                "form_id": "form1",
                "form": {"title": "Updated"}
            }))
            .await
            .unwrap();
        assert_eq!(updated["title"], "Updated");
    }

    #[test]
    fn unsafe_path_segments_are_rejected() {
        let err = clean_segment("../form").unwrap_err();
        assert!(err.contains("unsafe Typeform segment"));
    }
}
