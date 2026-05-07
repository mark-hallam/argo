//! Thin Trello REST API client.

use crate::auth::{TrelloAuth, TrelloConfig};
use argo_mcp::{CallToolResult, McpContent};

/// Trello API client authenticated with key/token query params.
#[derive(Clone)]
pub struct TrelloClient {
    http: reqwest::Client,
    base_url: String,
    auth: TrelloAuth,
}

impl TrelloClient {
    /// Build from `TRELLO_KEY` and `TRELLO_TOKEN`.
    pub fn from_env() -> Result<Self, String> {
        let cfg = TrelloConfig::from_env()?;
        Self::new(cfg.auth, cfg.base_url)
    }

    /// Build with explicit auth and API base URL.
    pub fn new(auth: TrelloAuth, base_url: String) -> Result<Self, String> {
        let http = reqwest::Client::builder()
            .user_agent(concat!(
                "argo-mcp-connector-trello/",
                env!("CARGO_PKG_VERSION")
            ))
            .build()
            .map_err(|e| format!("trello http client: {e}"))?;
        Ok(Self {
            http,
            base_url: base_url.trim_end_matches('/').to_string(),
            auth,
        })
    }

    /// GET `/members/{member}/boards`.
    pub async fn list_boards(&self, args: &serde_json::Value) -> Result<serde_json::Value, String> {
        let member = clean_segment(&optional_str(args, "member").unwrap_or_else(|| "me".into()))?;
        let mut query = Vec::new();
        push_str_or_default(args, &mut query, "filter", "open");
        push_str(args, &mut query, "fields");
        self.get(&format!("/members/{member}/boards"), query).await
    }

    /// GET `/boards/{board_id}/cards`.
    pub async fn list_cards(&self, args: &serde_json::Value) -> Result<serde_json::Value, String> {
        let board = clean_segment(&required_str(args, "board_id")?)?;
        let mut query = Vec::new();
        push_str_or_default(args, &mut query, "filter", "open");
        push_str(args, &mut query, "fields");
        self.get(&format!("/boards/{board}/cards"), query).await
    }

    /// GET `/cards/{card_id}`.
    pub async fn get_card(&self, args: &serde_json::Value) -> Result<serde_json::Value, String> {
        let card = clean_segment(&required_str(args, "card_id")?)?;
        self.get(&format!("/cards/{card}"), Vec::new()).await
    }

    /// POST `/cards`.
    pub async fn create_card(&self, args: &serde_json::Value) -> Result<serde_json::Value, String> {
        let mut query = vec![
            (
                "idList".into(),
                clean_segment(&required_str(args, "list_id")?)?,
            ),
            ("name".into(), required_str(args, "name")?),
        ];
        push_str(args, &mut query, "desc");
        push_str(args, &mut query, "pos");
        push_str(args, &mut query, "due");
        self.post_query("/cards", query).await
    }

    /// PUT `/cards/{card_id}`.
    pub async fn update_card(&self, args: &serde_json::Value) -> Result<serde_json::Value, String> {
        let card = clean_segment(&required_str(args, "card_id")?)?;
        let mut query = Vec::new();
        push_str(args, &mut query, "name");
        push_str(args, &mut query, "desc");
        push_bool(args, &mut query, "closed");
        push_str(args, &mut query, "due");
        if let Some(list) = optional_str(args, "list_id") {
            query.push(("idList".into(), clean_segment(&list)?));
        }
        if query.is_empty() {
            return Err("trello update_card: no update fields supplied".into());
        }
        self.put_query(&format!("/cards/{card}"), query).await
    }

    /// POST `/cards/{card_id}/actions/comments`.
    pub async fn add_comment(&self, args: &serde_json::Value) -> Result<serde_json::Value, String> {
        let card = clean_segment(&required_str(args, "card_id")?)?;
        let query = vec![("text".into(), required_str(args, "text")?)];
        self.post_query(&format!("/cards/{card}/actions/comments"), query)
            .await
    }

    /// Move a card via PUT `/cards/{card_id}` with `idList`.
    pub async fn move_card(&self, args: &serde_json::Value) -> Result<serde_json::Value, String> {
        let card = clean_segment(&required_str(args, "card_id")?)?;
        let list = clean_segment(&required_str(args, "list_id")?)?;
        self.put_query(&format!("/cards/{card}"), vec![("idList".into(), list)])
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
            .query(&self.auth.with_auth(query))
            .send()
            .await
            .map_err(|e| format!("trello GET {path}: {e}"))?;
        decode_response("GET", path, resp).await
    }

    async fn post_query(
        &self,
        path: &str,
        query: Vec<(String, String)>,
    ) -> Result<serde_json::Value, String> {
        let resp = self
            .http
            .post(format!("{}{}", self.base_url, path))
            .query(&self.auth.with_auth(query))
            .send()
            .await
            .map_err(|e| format!("trello POST {path}: {e}"))?;
        decode_response("POST", path, resp).await
    }

    async fn put_query(
        &self,
        path: &str,
        query: Vec<(String, String)>,
    ) -> Result<serde_json::Value, String> {
        let resp = self
            .http
            .put(format!("{}{}", self.base_url, path))
            .query(&self.auth.with_auth(query))
            .send()
            .await
            .map_err(|e| format!("trello PUT {path}: {e}"))?;
        decode_response("PUT", path, resp).await
    }
}

/// Convert a Trello result into MCP text content.
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
        .map_err(|e| format!("trello {method} {path}: read response: {e}"))?;
    let parsed = if text.trim().is_empty() {
        serde_json::json!({})
    } else {
        serde_json::from_str(&text)
            .map_err(|e| format!("trello {method} {path}: decode response: {e}: {text}"))?
    };
    if status.is_success() {
        Ok(parsed)
    } else {
        Err(format!("trello {method} {path}: HTTP {status}: {parsed}"))
    }
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

fn push_str(args: &serde_json::Value, out: &mut Vec<(String, String)>, field: &str) {
    if let Some(value) = optional_str(args, field) {
        out.push((field.into(), value));
    }
}

fn push_str_or_default(
    args: &serde_json::Value,
    out: &mut Vec<(String, String)>,
    field: &str,
    default: &str,
) {
    out.push((
        field.into(),
        optional_str(args, field).unwrap_or_else(|| default.into()),
    ));
}

fn push_bool(args: &serde_json::Value, out: &mut Vec<(String, String)>, field: &str) {
    if let Some(value) = args.get(field).and_then(|v| v.as_bool()) {
        out.push((field.into(), value.to_string()));
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
        Err(format!("unsafe Trello path segment: {raw}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    fn client(base_url: String) -> TrelloClient {
        TrelloClient::new(
            TrelloAuth::new("key123".into(), "tok456".into()).unwrap(),
            base_url,
        )
        .unwrap()
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
    async fn list_boards_uses_query_auth_without_authorization_header() {
        let base = serve_once(
            |req| {
                assert!(req.contains("GET /members/me/boards?filter=open&key=key123&token=tok456"));
                assert!(!req.to_ascii_lowercase().contains("authorization:"));
            },
            r#"[{"id":"b1","name":"Ops"}]"#,
        );
        let value = client(base)
            .list_boards(&serde_json::json!({}))
            .await
            .unwrap();
        assert_eq!(value[0]["id"], "b1");
    }

    #[tokio::test]
    async fn create_card_posts_trello_card_query_fields() {
        let base = serve_once(
            |req| {
                assert!(req.contains(
                    "POST /cards?idList=list1&name=Fix+bug&desc=ship&key=key123&token=tok456"
                ));
            },
            r#"{"id":"card1","name":"Fix bug"}"#,
        );
        let value = client(base)
            .create_card(&serde_json::json!({
                "list_id": "list1",
                "name": "Fix bug",
                "desc": "ship"
            }))
            .await
            .unwrap();
        assert_eq!(value["id"], "card1");
    }

    #[tokio::test]
    async fn move_card_puts_id_list() {
        let base = serve_once(
            |req| {
                assert!(req.contains("PUT /cards/card1?idList=list2&key=key123&token=tok456"));
            },
            r#"{"id":"card1","idList":"list2"}"#,
        );
        let value = client(base)
            .move_card(&serde_json::json!({"card_id": "card1", "list_id": "list2"}))
            .await
            .unwrap();
        assert_eq!(value["idList"], "list2");
    }

    #[test]
    fn path_segments_reject_slashes() {
        let err = clean_segment("../card").unwrap_err();
        assert!(err.contains("unsafe Trello path segment"));
    }
}
