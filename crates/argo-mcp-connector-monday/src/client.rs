//! Thin Monday.com GraphQL API HTTP client.
use crate::auth;
use argo_mcp::{CallToolResult, McpContent};

const BOARD_FIELDS: &str = "id name description state board_kind workspace_id";
const ITEM_FIELDS: &str =
    "id name state created_at updated_at group { id title } column_values { id text value type }";

/// Monday.com API client authenticated with an API token.
#[derive(Clone)]
pub struct MondayClient {
    http: reqwest::Client,
    base_url: String,
    token: String,
}

impl MondayClient {
    /// Build from `MONDAY_API_TOKEN`.
    pub fn from_env() -> Result<Self, String> {
        Self::new(auth::token_from_env()?, auth::base_url_from_env())
    }

    /// Build with an explicit token and GraphQL endpoint.
    pub fn new(token: String, base_url: String) -> Result<Self, String> {
        if token.trim().is_empty() {
            return Err(format!("missing {}", auth::MONDAY_API_TOKEN_ENV));
        }
        let http = reqwest::Client::builder()
            .user_agent(concat!(
                "argo-mcp-connector-monday/",
                env!("CARGO_PKG_VERSION")
            ))
            .build()
            .map_err(|e| format!("monday http client: {e}"))?;
        Ok(Self {
            http,
            base_url: base_url.trim_end_matches('/').to_string(),
            token,
        })
    }

    /// List boards visible to the authenticated token.
    pub async fn list_boards(&self, args: &serde_json::Value) -> Result<serde_json::Value, String> {
        self.graphql(&list_boards_query(), board_list_variables(args))
            .await
    }

    /// List items on one board.
    pub async fn list_items(&self, args: &serde_json::Value) -> Result<serde_json::Value, String> {
        self.graphql(&list_items_query(), board_items_variables(args)?)
            .await
    }

    /// Fetch one item.
    pub async fn get_item(&self, args: &serde_json::Value) -> Result<serde_json::Value, String> {
        self.graphql(&get_item_query(), item_variables(args)?).await
    }

    /// Create one item.
    pub async fn create_item(&self, args: &serde_json::Value) -> Result<serde_json::Value, String> {
        self.graphql(&create_item_query(), create_item_variables(args)?)
            .await
    }

    /// Update one item's status column.
    pub async fn update_item_status(
        &self,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        self.graphql(&update_status_query(), update_status_variables(args)?)
            .await
    }

    /// Add an update/comment to one item.
    pub async fn add_update(&self, args: &serde_json::Value) -> Result<serde_json::Value, String> {
        self.graphql(&add_update_query(), add_update_variables(args)?)
            .await
    }

    /// List groups on one board.
    pub async fn list_groups(&self, args: &serde_json::Value) -> Result<serde_json::Value, String> {
        self.graphql(&list_groups_query(), board_id_variables(args)?)
            .await
    }

    async fn graphql(
        &self,
        query: &str,
        variables: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let body = serde_json::json!({
            "query": query,
            "variables": variables
        });
        let resp = self
            .http
            .post(&self.base_url)
            .bearer_auth(&self.token)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("monday GraphQL POST: {e}"))?;
        decode_response(resp).await
    }
}

/// Convert a Monday.com tool result into MCP text content.
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

async fn decode_response(resp: reqwest::Response) -> Result<serde_json::Value, String> {
    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| format!("monday GraphQL POST: read response: {e}"))?;
    let parsed = if text.trim().is_empty() {
        serde_json::json!({})
    } else {
        serde_json::from_str(&text)
            .map_err(|e| format!("monday GraphQL POST: decode response: {e}: {text}"))?
    };
    if !status.is_success() {
        return Err(format!("monday GraphQL POST: HTTP {status}: {parsed}"));
    }
    if parsed.get("errors").is_some_and(|errors| !errors.is_null()) {
        return Err(format!("monday GraphQL errors: {}", parsed["errors"]));
    }
    Ok(parsed)
}

fn board_list_variables(args: &serde_json::Value) -> serde_json::Value {
    let mut vars = serde_json::Map::new();
    push_u64(args, &mut vars, "limit", 100);
    push_u64(args, &mut vars, "page", 500);
    serde_json::Value::Object(vars)
}

fn board_items_variables(args: &serde_json::Value) -> Result<serde_json::Value, String> {
    let mut vars = board_id_variables_map(args)?;
    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(25);
    vars.insert("limit".into(), limit.clamp(1, 100).into());
    Ok(serde_json::Value::Object(vars))
}

fn board_id_variables(args: &serde_json::Value) -> Result<serde_json::Value, String> {
    Ok(serde_json::Value::Object(board_id_variables_map(args)?))
}

fn board_id_variables_map(
    args: &serde_json::Value,
) -> Result<serde_json::Map<String, serde_json::Value>, String> {
    let board_id = required_str(args, "board_id")?;
    let mut vars = serde_json::Map::new();
    vars.insert("board_ids".into(), serde_json::json!([board_id]));
    Ok(vars)
}

fn item_variables(args: &serde_json::Value) -> Result<serde_json::Value, String> {
    let item_id = required_str(args, "item_id")?;
    Ok(serde_json::json!({"item_ids": [item_id]}))
}

fn create_item_variables(args: &serde_json::Value) -> Result<serde_json::Value, String> {
    let mut vars = serde_json::Map::new();
    vars.insert("board_id".into(), required_str(args, "board_id")?.into());
    vars.insert(
        "item_name".into(),
        required_any_str(args, &["item_name", "name"])?.into(),
    );
    push_optional_str(args, &mut vars, "group_id");
    if let Some(column_values) = column_values_arg(args)? {
        vars.insert("column_values".into(), column_values.into());
    }
    Ok(serde_json::Value::Object(vars))
}

fn update_status_variables(args: &serde_json::Value) -> Result<serde_json::Value, String> {
    let mut vars = serde_json::Map::new();
    vars.insert("board_id".into(), required_str(args, "board_id")?.into());
    vars.insert("item_id".into(), required_str(args, "item_id")?.into());
    vars.insert(
        "column_id".into(),
        optional_str(args, "column_id")
            .unwrap_or_else(|| "status".into())
            .into(),
    );
    vars.insert(
        "value".into(),
        required_any_str(args, &["status", "value"])?.into(),
    );
    Ok(serde_json::Value::Object(vars))
}

fn add_update_variables(args: &serde_json::Value) -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "item_id": required_str(args, "item_id")?,
        "body": required_str(args, "body")?
    }))
}

fn column_values_arg(args: &serde_json::Value) -> Result<Option<String>, String> {
    match args.get("column_values") {
        None => Ok(None),
        Some(value) if value.is_object() || value.is_array() => Ok(Some(value.to_string())),
        Some(value) => value
            .as_str()
            .filter(|s| !s.trim().is_empty())
            .map(|s| Some(s.to_string()))
            .ok_or_else(|| "column_values must be an object, array, or JSON string".into()),
    }
}

fn required_str(args: &serde_json::Value, field: &str) -> Result<String, String> {
    args.get(field)
        .and_then(|v| v.as_str())
        .filter(|v| !v.trim().is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("missing string field '{field}'"))
}

fn required_any_str(args: &serde_json::Value, fields: &[&str]) -> Result<String, String> {
    fields
        .iter()
        .find_map(|field| required_str(args, field).ok())
        .ok_or_else(|| format!("missing string field '{}', or '{}'", fields[0], fields[1]))
}

fn optional_str(args: &serde_json::Value, field: &str) -> Option<String> {
    args.get(field)
        .and_then(|v| v.as_str())
        .filter(|v| !v.trim().is_empty())
        .map(str::to_string)
}

fn push_optional_str(
    args: &serde_json::Value,
    vars: &mut serde_json::Map<String, serde_json::Value>,
    field: &str,
) {
    if let Some(value) = optional_str(args, field) {
        vars.insert(field.into(), value.into());
    }
}

fn push_u64(
    args: &serde_json::Value,
    vars: &mut serde_json::Map<String, serde_json::Value>,
    field: &str,
    max: u64,
) {
    if let Some(value) = args.get(field).and_then(|v| v.as_u64()) {
        vars.insert(field.into(), value.clamp(1, max).into());
    }
}

fn list_boards_query() -> String {
    format!("query ArgoListBoards($limit: Int, $page: Int) {{ boards(limit: $limit, page: $page) {{ {BOARD_FIELDS} }} }}")
}

fn list_items_query() -> String {
    format!("query ArgoListItems($board_ids: [ID!], $limit: Int!) {{ boards(ids: $board_ids) {{ id items_page(limit: $limit) {{ cursor items {{ {ITEM_FIELDS} }} }} }} }}")
}

fn get_item_query() -> String {
    format!("query ArgoGetItem($item_ids: [ID!]) {{ items(ids: $item_ids) {{ {ITEM_FIELDS} board {{ id name }} }} }}")
}

fn create_item_query() -> String {
    "mutation ArgoCreateItem($board_id: ID!, $group_id: String, $item_name: String!, $column_values: JSON) { create_item(board_id: $board_id, group_id: $group_id, item_name: $item_name, column_values: $column_values) { id name board { id } group { id title } } }".into()
}

fn update_status_query() -> String {
    "mutation ArgoUpdateItemStatus($board_id: ID!, $item_id: ID!, $column_id: String!, $value: String!) { change_simple_column_value(board_id: $board_id, item_id: $item_id, column_id: $column_id, value: $value) { id name } }".into()
}

fn add_update_query() -> String {
    "mutation ArgoAddUpdate($item_id: ID!, $body: String!) { create_update(item_id: $item_id, body: $body) { id body created_at } }".into()
}

fn list_groups_query() -> String {
    "query ArgoListGroups($board_ids: [ID!]) { boards(ids: $board_ids) { id groups { id title color position } } }".into()
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
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = [0; 8192];
            let n = stream.read(&mut buf).unwrap();
            let req = String::from_utf8_lossy(&buf[..n]).to_string();
            assert!(req
                .to_ascii_lowercase()
                .contains("authorization: bearer token_test"));
            assert_req(&req);
            let response = format!(
                "{status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(response.as_bytes()).unwrap();
        });
        format!("http://{addr}/v2")
    }

    fn request_json(req: &str) -> serde_json::Value {
        let body = req.split("\r\n\r\n").nth(1).unwrap();
        serde_json::from_str(body).unwrap()
    }

    #[tokio::test]
    async fn list_items_posts_board_scoped_graphql_with_bearer_auth() {
        let base = serve_once(
            r#"{"data":{"boards":[{"id":"123","items_page":{"items":[]}}]}}"#,
            "HTTP/1.1 200 OK",
            |req| {
                assert!(req.starts_with("POST /v2 "));
                let body = request_json(req);
                assert!(body["query"].as_str().unwrap().contains("ArgoListItems"));
                assert_eq!(body["variables"]["board_ids"][0], "123");
                assert_eq!(body["variables"]["limit"], 2);
            },
        );
        let client = MondayClient::new("token_test".into(), base).unwrap();
        let out = client
            .list_items(&serde_json::json!({"board_id": "123", "limit": 2}))
            .await
            .unwrap();
        assert_eq!(out["data"]["boards"][0]["id"], "123");
    }

    #[tokio::test]
    async fn create_item_serializes_column_values() {
        let base = serve_once(
            r#"{"data":{"create_item":{"id":"456","name":"Lead"}}}"#,
            "HTTP/1.1 200 OK",
            |req| {
                let body = request_json(req);
                assert!(body["query"].as_str().unwrap().contains("create_item"));
                assert_eq!(body["variables"]["board_id"], "123");
                assert_eq!(body["variables"]["item_name"], "Lead");
                assert_eq!(
                    body["variables"]["column_values"],
                    r#"{"Status":"Working"}"#
                );
            },
        );
        let client = MondayClient::new("token_test".into(), base).unwrap();
        let out = client
            .create_item(&serde_json::json!({
                "board_id": "123",
                "name": "Lead",
                "column_values": {"Status": "Working"}
            }))
            .await
            .unwrap();
        assert_eq!(out["data"]["create_item"]["id"], "456");
    }

    #[tokio::test]
    async fn graphql_errors_return_err() {
        let base = serve_once(
            r#"{"errors":[{"message":"bad query"}]}"#,
            "HTTP/1.1 200 OK",
            |_| {},
        );
        let client = MondayClient::new("token_test".into(), base).unwrap();
        let err = client
            .list_boards(&serde_json::json!({}))
            .await
            .unwrap_err();
        assert!(err.contains("bad query"));
    }

    #[test]
    fn update_status_defaults_to_status_column() {
        let vars = update_status_variables(&serde_json::json!({
            "board_id": "123",
            "item_id": "456",
            "status": "Done"
        }))
        .unwrap();
        assert_eq!(vars["column_id"], "status");
        assert_eq!(vars["value"], "Done");
    }
}
