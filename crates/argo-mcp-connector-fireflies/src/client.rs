//! Thin Fireflies.ai GraphQL API client.
use crate::auth;
use argo_mcp::{CallToolResult, McpContent};
use serde_json::{json, Map, Value};

const MEETING_FIELDS: &str = r#"
    id
    title
    host_email
    organizer_email
    participants
    date
    transcript_url
    audio_url
    video_url
    duration
    meeting_link
    is_live
    summary {
      keywords
      action_items
      outline
      overview
      short_summary
      meeting_type
      topics_discussed
    }
    meeting_attendees {
      displayName
      email
      name
    }
    user {
      user_id
      email
      name
    }
"#;

const TRANSCRIPT_FIELDS: &str = r#"
    MEETING_FIELDS_PLACEHOLDER
    sentences {
      index
      speaker_name
      speaker_id
      text
      raw_text
      start_time
      end_time
    }
    speakers {
      id
      name
    }
"#;

const LIST_MEETINGS_QUERY: &str = r#"
query ListMeetings(
  $keyword: String
  $scope: TranscriptsQueryScope
  $fromDate: DateTime
  $toDate: DateTime
  $date: Float
  $limit: Int
  $skip: Int
  $hostEmail: String
  $userId: String
  $mine: Boolean
  $organizers: [String]
  $participants: [String]
  $channelId: String
) {
  transcripts(
    keyword: $keyword
    scope: $scope
    fromDate: $fromDate
    toDate: $toDate
    date: $date
    limit: $limit
    skip: $skip
    host_email: $hostEmail
    user_id: $userId
    mine: $mine
    organizers: $organizers
    participants: $participants
    channel_id: $channelId
  ) {
    MEETING_FIELDS_PLACEHOLDER
  }
}
"#;

const GET_MEETING_QUERY: &str = r#"
query GetMeeting($id: String!) {
  transcript(id: $id) {
    MEETING_FIELDS_PLACEHOLDER
  }
}
"#;

const GET_TRANSCRIPT_QUERY: &str = r#"
query GetTranscript($id: String!) {
  transcript(id: $id) {
    TRANSCRIPT_FIELDS_PLACEHOLDER
  }
}
"#;

const LIST_USERS_QUERY: &str = r#"
query ListUsers {
  users {
    user_id
    email
    name
    num_transcripts
    recent_meeting
    minutes_consumed
    is_admin
    integrations
  }
}
"#;

/// Fireflies.ai GraphQL API client authenticated with an API key.
#[derive(Clone)]
pub struct FirefliesClient {
    http: reqwest::Client,
    base_url: String,
    api_key: String,
}

impl FirefliesClient {
    /// Build from `FIREFLIES_API_KEY` and optional `FIREFLIES_BASE_URL`.
    pub fn from_env() -> Result<Self, String> {
        Self::new(auth::api_key_from_env()?, auth::base_url_from_env())
    }

    /// Build with an explicit API key and GraphQL endpoint.
    pub fn new(api_key: String, base_url: String) -> Result<Self, String> {
        if api_key.trim().is_empty() {
            return Err(format!("missing {}", auth::FIREFLIES_API_KEY_ENV));
        }
        let http = reqwest::Client::builder()
            .user_agent(concat!(
                "argo-mcp-connector-fireflies/",
                env!("CARGO_PKG_VERSION")
            ))
            .build()
            .map_err(|e| format!("fireflies http client: {e}"))?;
        Ok(Self {
            http,
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
        })
    }

    /// Query Fireflies.ai transcripts as meeting metadata.
    pub async fn list_meetings(&self, args: &Value) -> Result<Value, String> {
        let query = meeting_query(LIST_MEETINGS_QUERY);
        self.graphql(&query, Value::Object(transcript_variables(args, false)?))
            .await
    }

    /// Fetch one Fireflies.ai meeting by transcript id.
    pub async fn get_meeting(&self, args: &Value) -> Result<Value, String> {
        let id = required_id(args)?;
        let query = meeting_query(GET_MEETING_QUERY);
        self.graphql(&query, json!({"id": id})).await
    }

    /// Fetch one Fireflies.ai transcript with sentence-level text.
    pub async fn get_transcript(&self, args: &Value) -> Result<Value, String> {
        let id = required_id(args)?;
        let query = transcript_query(GET_TRANSCRIPT_QUERY);
        self.graphql(&query, json!({"id": id})).await
    }

    /// Search Fireflies.ai transcripts via the GraphQL `transcripts(keyword:)` filter.
    pub async fn search_transcripts(&self, args: &Value) -> Result<Value, String> {
        let query = meeting_query(LIST_MEETINGS_QUERY);
        self.graphql(&query, Value::Object(transcript_variables(args, true)?))
            .await
    }

    /// Query Fireflies.ai team users.
    pub async fn list_users(&self, _args: &Value) -> Result<Value, String> {
        self.graphql(LIST_USERS_QUERY, json!({})).await
    }

    async fn graphql(&self, query: &str, variables: Value) -> Result<Value, String> {
        let body = json!({"query": query, "variables": variables});
        let resp = self
            .http
            .post(&self.base_url)
            .bearer_auth(&self.api_key)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("fireflies graphql: {e}"))?;
        decode_response(resp).await
    }
}

/// Convert a Fireflies.ai tool result into MCP text content.
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

async fn decode_response(resp: reqwest::Response) -> Result<Value, String> {
    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| format!("fireflies graphql: read response: {e}"))?;
    let parsed = if text.trim().is_empty() {
        json!({})
    } else {
        serde_json::from_str(&text)
            .map_err(|e| format!("fireflies graphql: decode response: {e}: {text}"))?
    };
    if !status.is_success() {
        return Err(format!("fireflies graphql: HTTP {status}: {parsed}"));
    }
    if parsed
        .get("errors")
        .and_then(Value::as_array)
        .is_some_and(|errors| !errors.is_empty())
    {
        return Err(format!("fireflies graphql errors: {parsed}"));
    }
    Ok(parsed)
}

fn transcript_variables(args: &Value, require_keyword: bool) -> Result<Map<String, Value>, String> {
    let mut variables = Map::new();
    if require_keyword {
        variables.insert("keyword".into(), required_str(args, "keyword")?.into());
    } else {
        copy_string(args, &mut variables, "keyword", "keyword");
    }
    copy_string(args, &mut variables, "scope", "scope");
    copy_string_alias(args, &mut variables, &["from_date", "fromDate"], "fromDate");
    copy_string_alias(args, &mut variables, &["to_date", "toDate"], "toDate");
    copy_number(args, &mut variables, "date", "date");
    copy_u64(args, &mut variables, "limit", "limit", 1, 50);
    copy_u64(args, &mut variables, "skip", "skip", 0, u64::MAX);
    copy_string(args, &mut variables, "host_email", "hostEmail");
    copy_string(args, &mut variables, "user_id", "userId");
    copy_bool(args, &mut variables, "mine", "mine");
    copy_array(args, &mut variables, "organizers", "organizers");
    copy_array(args, &mut variables, "participants", "participants");
    copy_string(args, &mut variables, "channel_id", "channelId");
    Ok(variables)
}

fn meeting_query(template: &str) -> String {
    template.replace("MEETING_FIELDS_PLACEHOLDER", MEETING_FIELDS)
}

fn transcript_query(template: &str) -> String {
    let fields = TRANSCRIPT_FIELDS.replace("MEETING_FIELDS_PLACEHOLDER", MEETING_FIELDS);
    template.replace("TRANSCRIPT_FIELDS_PLACEHOLDER", &fields)
}

fn required_id(args: &Value) -> Result<String, String> {
    optional_str(args, "meeting_id")
        .or_else(|| optional_str(args, "transcript_id"))
        .ok_or_else(|| "missing string field 'meeting_id'".into())
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

fn copy_string(args: &Value, out: &mut Map<String, Value>, source: &str, target: &str) {
    if let Some(value) = optional_str(args, source) {
        out.insert(target.into(), value.into());
    }
}

fn copy_string_alias(args: &Value, out: &mut Map<String, Value>, sources: &[&str], target: &str) {
    if let Some(value) = sources.iter().find_map(|source| optional_str(args, source)) {
        out.insert(target.into(), value.into());
    }
}

fn copy_number(args: &Value, out: &mut Map<String, Value>, source: &str, target: &str) {
    if let Some(value) = args.get(source).filter(|value| value.is_number()) {
        out.insert(target.into(), value.clone());
    }
}

fn copy_u64(
    args: &Value,
    out: &mut Map<String, Value>,
    source: &str,
    target: &str,
    min: u64,
    max: u64,
) {
    if let Some(value) = args.get(source).and_then(Value::as_u64) {
        out.insert(target.into(), value.clamp(min, max).into());
    }
}

fn copy_bool(args: &Value, out: &mut Map<String, Value>, source: &str, target: &str) {
    if let Some(value) = args.get(source).and_then(Value::as_bool) {
        out.insert(target.into(), value.into());
    }
}

fn copy_array(args: &Value, out: &mut Map<String, Value>, source: &str, target: &str) {
    if let Some(value) = args.get(source).filter(|value| value.is_array()) {
        out.insert(target.into(), value.clone());
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
            let mut buf = vec![0_u8; 16_384];
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
    async fn list_meetings_uses_bearer_auth_and_date_filters() {
        let (base, server) = serve_once(r#"{"data":{"transcripts":[]}}"#).await;
        let client = FirefliesClient::new("ff_test".into(), base).expect("client");
        let out = client
            .list_meetings(&json!({
                "from_date": "2026-05-01T00:00:00.000Z",
                "to_date": "2026-05-06T00:00:00.000Z",
                "limit": 99,
                "mine": true
            }))
            .await
            .expect("list meetings");
        assert!(out["data"]["transcripts"].is_array());
        let req = server.await.expect("server");
        assert!(req
            .to_ascii_lowercase()
            .contains("authorization: bearer ff_test"));
        let body = request_body(&req);
        assert_eq!(body["variables"]["fromDate"], "2026-05-01T00:00:00.000Z");
        assert_eq!(body["variables"]["toDate"], "2026-05-06T00:00:00.000Z");
        assert_eq!(body["variables"]["limit"], 50);
        assert_eq!(body["variables"]["mine"], true);
    }

    #[tokio::test]
    async fn search_transcripts_passes_keyword_and_scope_to_graphql() {
        let (base, server) = serve_once(r#"{"data":{"transcripts":[]}}"#).await;
        let client = FirefliesClient::new("ff_test".into(), base).expect("client");
        client
            .search_transcripts(&json!({
                "keyword": "budget review",
                "scope": "all",
                "participants": ["ops@example.com"]
            }))
            .await
            .expect("search transcripts");
        let req = server.await.expect("server");
        let body = request_body(&req);
        assert!(body["query"]
            .as_str()
            .expect("query")
            .contains("keyword: $keyword"));
        assert_eq!(body["variables"]["keyword"], "budget review");
        assert_eq!(body["variables"]["scope"], "all");
        assert_eq!(body["variables"]["participants"][0], "ops@example.com");
    }

    #[tokio::test]
    async fn get_transcript_requests_sentence_fields() {
        let (base, server) = serve_once(r#"{"data":{"transcript":{"id":"tr_1"}}}"#).await;
        let client = FirefliesClient::new("ff_test".into(), base).expect("client");
        client
            .get_transcript(&json!({"meeting_id": "tr_1"}))
            .await
            .expect("get transcript");
        let req = server.await.expect("server");
        let body = request_body(&req);
        assert!(body["query"].as_str().expect("query").contains("sentences"));
        assert_eq!(body["variables"]["id"], "tr_1");
    }

    #[tokio::test]
    async fn search_transcripts_requires_keyword() {
        let client =
            FirefliesClient::new("ff_test".into(), "http://127.0.0.1:9".into()).expect("client");
        let err = client.search_transcripts(&json!({})).await.unwrap_err();
        assert!(err.contains("keyword"));
    }
}
