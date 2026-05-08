//! Thin Google Meet API v2 client.

use crate::auth;
use argo_mcp::{CallToolResult, McpContent};

/// Google Meet API client authenticated with an OAuth bearer token.
#[derive(Clone)]
pub struct GoogleMeetClient {
    http: reqwest::Client,
    base_url: String,
    token: String,
}

impl GoogleMeetClient {
    /// Build from `GOOGLE_OAUTH_TOKEN` and optional `GOOGLE_MEET_BASE_URL`.
    pub fn from_env() -> Result<Self, String> {
        Self::new(auth::token_from_env()?, auth::base_url_from_env())
    }

    /// Build with an explicit bearer token and API base URL.
    pub fn new(token: String, base_url: String) -> Result<Self, String> {
        if token.trim().is_empty() {
            return Err(format!("missing {}", auth::GOOGLE_OAUTH_TOKEN_ENV));
        }
        let http = reqwest::Client::builder()
            .user_agent(concat!(
                "argo-mcp-connector-google-meet/",
                env!("CARGO_PKG_VERSION")
            ))
            .build()
            .map_err(|e| format!("google meet http client: {e}"))?;
        Ok(Self {
            http,
            base_url: base_url.trim_end_matches('/').to_string(),
            token,
        })
    }

    /// GET `/conferenceRecords`.
    pub async fn list_meetings(
        &self,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let mut query = paging_query(args);
        push_query_str(args, &mut query, "filter");
        self.get("/conferenceRecords", query).await
    }

    /// POST `/spaces`, creating a meeting space/link.
    pub async fn schedule_meeting(
        &self,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let body = args
            .get("space")
            .filter(|value| value.is_object())
            .cloned()
            .unwrap_or_else(|| serde_json::json!({}));
        self.post_json("/spaces", body).await
    }

    /// GET `/conferenceRecords/{meeting_id}`.
    pub async fn get_meeting(&self, args: &serde_json::Value) -> Result<serde_json::Value, String> {
        let record = conference_record_path(args)?;
        self.get(&format!("/{record}"), Vec::new()).await
    }

    /// GET `/conferenceRecords/{meeting_id}/participants`.
    pub async fn list_participants(
        &self,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let record = conference_record_path(args)?;
        self.get(&format!("/{record}/participants"), paging_query(args))
            .await
    }

    /// GET `/conferenceRecords/{meeting_id}/recordings/{recording_id}`.
    pub async fn get_recording(
        &self,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let record = conference_record_path(args)?;
        let recording_id = encode_path_segment(&required_str(args, "recording_id")?);
        self.get(&format!("/{record}/recordings/{recording_id}"), Vec::new())
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
            .bearer_auth(&self.token)
            .query(&query)
            .send()
            .await
            .map_err(|e| format!("google meet GET {path}: {e}"))?;
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
            .map_err(|e| format!("google meet POST {path}: {e}"))?;
        decode_response("POST", path, resp).await
    }
}

/// Convert a Google Meet result into MCP text content.
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
        .map_err(|e| format!("google meet {method} {path}: read response: {e}"))?;
    let parsed = if text.trim().is_empty() {
        serde_json::json!({})
    } else {
        serde_json::from_str(&text)
            .map_err(|e| format!("google meet {method} {path}: decode response: {e}: {text}"))?
    };
    if status.is_success() {
        Ok(parsed)
    } else {
        Err(format!(
            "google meet {method} {path}: HTTP {status}: {parsed}"
        ))
    }
}

fn conference_record_path(args: &serde_json::Value) -> Result<String, String> {
    let id = required_str(args, "meeting_id")?;
    if let Some(suffix) = id.strip_prefix("conferenceRecords/") {
        Ok(format!("conferenceRecords/{}", encode_path_segment(suffix)))
    } else {
        Ok(format!("conferenceRecords/{}", encode_path_segment(&id)))
    }
}

fn paging_query(args: &serde_json::Value) -> Vec<(String, String)> {
    let page_size = args
        .get("page_size")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(25)
        .clamp(1, 100)
        .to_string();
    let mut query = vec![("pageSize".into(), page_size)];
    push_query_str(args, &mut query, "pageToken");
    query
}

fn push_query_str(args: &serde_json::Value, query: &mut Vec<(String, String)>, field: &str) {
    if let Some(value) = optional_str(args, field) {
        query.push((field.into(), value));
    }
}

fn required_str(args: &serde_json::Value, field: &str) -> Result<String, String> {
    optional_str(args, field).ok_or_else(|| format!("missing string field '{field}'"))
}

fn optional_str(args: &serde_json::Value, field: &str) -> Option<String> {
    args.get(field)
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
}

fn encode_path_segment(value: &str) -> String {
    let mut out = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(char::from(byte));
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
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

    fn request_body(req: &str) -> serde_json::Value {
        let body = req.split("\r\n\r\n").nth(1).expect("http body");
        serde_json::from_str(body).expect("json body")
    }

    #[tokio::test]
    async fn list_meetings_forwards_bearer_token() {
        let (base, server) = serve_once(r#"{"conferenceRecords":[]}"#).await;
        let client = GoogleMeetClient::new("google_token".into(), base).expect("client");
        client
            .list_meetings(&serde_json::json!({"page_size": 3, "filter": "space.name=\"x\""}))
            .await
            .expect("meetings");
        let req = server.await.expect("server");
        assert!(req
            .to_ascii_lowercase()
            .contains("authorization: bearer google_token"));
        assert!(req.starts_with("GET /conferenceRecords?pageSize=3&filter=space.name%3D%22x%22 "));
    }

    #[tokio::test]
    async fn schedule_meeting_posts_space_body_with_bearer() {
        let (base, server) =
            serve_once(r#"{"name":"spaces/abc","meetingUri":"https://meet.google.com/a"}"#).await;
        let client = GoogleMeetClient::new("google_token".into(), base).expect("client");
        client
            .schedule_meeting(&serde_json::json!({"space": {"config": {"accessType": "TRUSTED"}}}))
            .await
            .expect("schedule");
        let req = server.await.expect("server");
        assert!(req
            .to_ascii_lowercase()
            .contains("authorization: bearer google_token"));
        assert!(req.starts_with("POST /spaces "));
        assert_eq!(request_body(&req)["config"]["accessType"], "TRUSTED");
    }

    #[tokio::test]
    async fn participants_and_recordings_use_conference_record_path() {
        let (base, server) = serve_once(r#"{"participants":[]}"#).await;
        let client = GoogleMeetClient::new("google_token".into(), base).expect("client");
        client
            .list_participants(&serde_json::json!({"meeting_id": "abc", "page_size": 2}))
            .await
            .expect("participants");
        let req = server.await.expect("server");
        assert!(req.starts_with("GET /conferenceRecords/abc/participants?pageSize=2 "));

        let (base, server) = serve_once(r#"{"name":"recordings/r1"}"#).await;
        let client = GoogleMeetClient::new("google_token".into(), base).expect("client");
        client
            .get_recording(&serde_json::json!({"meeting_id": "abc", "recording_id": "r1"}))
            .await
            .expect("recording");
        let req = server.await.expect("server");
        assert!(req.starts_with("GET /conferenceRecords/abc/recordings/r1 "));
    }
}
