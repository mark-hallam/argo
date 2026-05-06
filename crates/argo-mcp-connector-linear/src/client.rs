//! Thin Linear GraphQL API client.
use crate::auth;
use argo_mcp::{CallToolResult, McpContent};
use serde_json::{json, Map, Value};

const ISSUE_FIELDS: &str = r#"
    id
    identifier
    title
    description
    priority
    url
    createdAt
    updatedAt
    state { id name type }
    team { id key name }
    assignee { id name email }
"#;

const LIST_TEAMS_QUERY: &str = r#"
query ListTeams($first: Int, $after: String) {
  teams(first: $first, after: $after) {
    nodes { id key name description }
    pageInfo { hasNextPage endCursor }
  }
}
"#;

const LIST_ISSUES_QUERY: &str = r#"
query ListIssues($first: Int, $after: String, $filter: IssueFilter) {
  issues(first: $first, after: $after, filter: $filter) {
    nodes { ISSUE_FIELDS_PLACEHOLDER }
    pageInfo { hasNextPage endCursor }
  }
}
"#;

const GET_ISSUE_QUERY: &str = r#"
query GetIssue($id: String!) {
  issue(id: $id) { ISSUE_FIELDS_PLACEHOLDER }
}
"#;

const CREATE_ISSUE_QUERY: &str = r#"
mutation CreateIssue($input: IssueCreateInput!) {
  issueCreate(input: $input) {
    success
    issue { ISSUE_FIELDS_PLACEHOLDER }
  }
}
"#;

const UPDATE_ISSUE_QUERY: &str = r#"
mutation UpdateIssue($id: String!, $input: IssueUpdateInput!) {
  issueUpdate(id: $id, input: $input) {
    success
    issue { ISSUE_FIELDS_PLACEHOLDER }
  }
}
"#;

const ADD_COMMENT_QUERY: &str = r#"
mutation AddComment($input: CommentCreateInput!) {
  commentCreate(input: $input) {
    success
    comment { id body createdAt issue { id identifier title } }
  }
}
"#;

const LIST_WORKFLOWS_QUERY: &str = r#"
query ListWorkflows($first: Int, $after: String, $filter: WorkflowStateFilter) {
  workflowStates(first: $first, after: $after, filter: $filter) {
    nodes { id name type position team { id key name } }
    pageInfo { hasNextPage endCursor }
  }
}
"#;

/// Linear GraphQL API client authenticated with an API key.
#[derive(Clone)]
pub struct LinearClient {
    http: reqwest::Client,
    base_url: String,
    api_key: String,
}

impl LinearClient {
    /// Build from `LINEAR_API_KEY` and optional `LINEAR_BASE_URL`.
    pub fn from_env() -> Result<Self, String> {
        Self::new(auth::api_key_from_env()?, auth::base_url_from_env())
    }

    /// Build with an explicit API key and GraphQL endpoint.
    pub fn new(api_key: String, base_url: String) -> Result<Self, String> {
        if api_key.trim().is_empty() {
            return Err(format!("missing {}", auth::LINEAR_API_KEY_ENV));
        }
        let http = reqwest::Client::builder()
            .user_agent(concat!(
                "argo-mcp-connector-linear/",
                env!("CARGO_PKG_VERSION")
            ))
            .build()
            .map_err(|e| format!("linear http client: {e}"))?;
        Ok(Self {
            http,
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
        })
    }

    /// Query Linear teams.
    pub async fn list_teams(&self, args: &Value) -> Result<Value, String> {
        self.graphql(LIST_TEAMS_QUERY, Value::Object(paging_variables(args)))
            .await
    }

    /// Query Linear issues, preserving any GraphQL `filter` object unchanged.
    pub async fn list_issues(&self, args: &Value) -> Result<Value, String> {
        let mut variables = paging_variables(args);
        if let Some(filter) = args.get("filter").filter(|value| value.is_object()) {
            variables.insert("filter".into(), filter.clone());
        } else if let Some(team_id) = optional_str(args, "team_id") {
            variables.insert("filter".into(), json!({"team": {"id": {"eq": team_id}}}));
        }
        let query = issue_query(LIST_ISSUES_QUERY);
        self.graphql(&query, Value::Object(variables)).await
    }

    /// Fetch one Linear issue by UUID or identifier.
    pub async fn get_issue(&self, args: &Value) -> Result<Value, String> {
        let id = required_str(args, "issue_id")?;
        let query = issue_query(GET_ISSUE_QUERY);
        self.graphql(&query, json!({"id": id})).await
    }

    /// Create a Linear issue.
    pub async fn create_issue(&self, args: &Value) -> Result<Value, String> {
        let mut input = Map::new();
        input.insert("teamId".into(), required_str(args, "team_id")?.into());
        input.insert("title".into(), required_str(args, "title")?.into());
        copy_string(args, &mut input, "description", "description");
        copy_string(args, &mut input, "assignee_id", "assigneeId");
        copy_string(args, &mut input, "state_id", "stateId");
        copy_value(args, &mut input, "priority", "priority");
        copy_value(args, &mut input, "label_ids", "labelIds");
        let query = issue_query(CREATE_ISSUE_QUERY);
        self.graphql(&query, json!({"input": Value::Object(input)}))
            .await
    }

    /// Update a Linear issue.
    pub async fn update_issue(&self, args: &Value) -> Result<Value, String> {
        let id = required_str(args, "issue_id")?;
        let mut input = Map::new();
        copy_string(args, &mut input, "title", "title");
        copy_string(args, &mut input, "description", "description");
        copy_string(args, &mut input, "assignee_id", "assigneeId");
        copy_string(args, &mut input, "state_id", "stateId");
        copy_value(args, &mut input, "priority", "priority");
        copy_value(args, &mut input, "label_ids", "labelIds");
        if input.is_empty() {
            return Err("update_issue needs at least one mutable field".into());
        }
        let query = issue_query(UPDATE_ISSUE_QUERY);
        self.graphql(&query, json!({"id": id, "input": Value::Object(input)}))
            .await
    }

    /// Add a comment to a Linear issue.
    pub async fn add_comment(&self, args: &Value) -> Result<Value, String> {
        let issue_id = required_str(args, "issue_id")?;
        let body = required_str(args, "body")?;
        self.graphql(
            ADD_COMMENT_QUERY,
            json!({"input": {"issueId": issue_id, "body": body}}),
        )
        .await
    }

    /// Query Linear workflow states.
    pub async fn list_workflows(&self, args: &Value) -> Result<Value, String> {
        let mut variables = paging_variables(args);
        if let Some(filter) = args.get("filter").filter(|value| value.is_object()) {
            variables.insert("filter".into(), filter.clone());
        } else if let Some(team_id) = optional_str(args, "team_id") {
            variables.insert("filter".into(), json!({"team": {"id": {"eq": team_id}}}));
        }
        self.graphql(LIST_WORKFLOWS_QUERY, Value::Object(variables))
            .await
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
            .map_err(|e| format!("linear graphql: {e}"))?;
        decode_response(resp).await
    }
}

/// Convert a Linear tool result into MCP text content.
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
        .map_err(|e| format!("linear graphql: read response: {e}"))?;
    let parsed = if text.trim().is_empty() {
        json!({})
    } else {
        serde_json::from_str(&text)
            .map_err(|e| format!("linear graphql: decode response: {e}: {text}"))?
    };
    if !status.is_success() {
        return Err(format!("linear graphql: HTTP {status}: {parsed}"));
    }
    if parsed
        .get("errors")
        .and_then(Value::as_array)
        .is_some_and(|errors| !errors.is_empty())
    {
        return Err(format!("linear graphql errors: {parsed}"));
    }
    Ok(parsed)
}

fn issue_query(template: &str) -> String {
    template.replace("ISSUE_FIELDS_PLACEHOLDER", ISSUE_FIELDS)
}

fn paging_variables(args: &Value) -> Map<String, Value> {
    let mut variables = Map::new();
    variables.insert("first".into(), first_arg(args).into());
    if let Some(after) = optional_str(args, "after") {
        variables.insert("after".into(), after.into());
    }
    variables
}

fn first_arg(args: &Value) -> u64 {
    args.get("first")
        .and_then(Value::as_u64)
        .unwrap_or(25)
        .clamp(1, 100)
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

fn copy_value(args: &Value, out: &mut Map<String, Value>, source: &str, target: &str) {
    if let Some(value) = args.get(source) {
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
    async fn list_teams_uses_bearer_auth() {
        let (base, server) = serve_once(r#"{"data":{"teams":{"nodes":[]}}}"#).await;
        let client = LinearClient::new("lin_test".into(), base).expect("client");
        let out = client
            .list_teams(&json!({"first": 3}))
            .await
            .expect("list teams");
        assert!(out["data"]["teams"]["nodes"].is_array());
        let req = server.await.expect("server");
        assert!(req
            .to_ascii_lowercase()
            .contains("authorization: bearer lin_test"));
    }

    #[tokio::test]
    async fn list_issues_preserves_filter_passthrough() {
        let (base, server) = serve_once(r#"{"data":{"issues":{"nodes":[]}}}"#).await;
        let client = LinearClient::new("lin_test".into(), base).expect("client");
        let filter = json!({"state": {"type": {"eq": "started"}}});
        client
            .list_issues(&json!({"filter": filter, "first": 7}))
            .await
            .expect("list issues");
        let req = server.await.expect("server");
        let body = request_body(&req);
        assert_eq!(body["variables"]["filter"], filter);
        assert_eq!(body["variables"]["first"], 7);
    }

    #[tokio::test]
    async fn create_issue_posts_issue_create_mutation() {
        let (base, server) = serve_once(r#"{"data":{"issueCreate":{"success":true}}}"#).await;
        let client = LinearClient::new("lin_test".into(), base).expect("client");
        client
            .create_issue(&json!({
                "team_id": "team-1",
                "title": "Ship Linear connector",
                "state_id": "state-1",
                "priority": 2
            }))
            .await
            .expect("create issue");
        let req = server.await.expect("server");
        let body = request_body(&req);
        assert!(body["query"]
            .as_str()
            .expect("query")
            .contains("issueCreate"));
        assert_eq!(body["variables"]["input"]["teamId"], "team-1");
        assert_eq!(body["variables"]["input"]["stateId"], "state-1");
    }

    #[tokio::test]
    async fn update_issue_rejects_empty_mutation() {
        let client =
            LinearClient::new("lin_test".into(), "http://127.0.0.1:9".into()).expect("client");
        let err = client
            .update_issue(&json!({"issue_id": "ARGO-1"}))
            .await
            .unwrap_err();
        assert!(err.contains("at least one mutable field"));
    }
}
