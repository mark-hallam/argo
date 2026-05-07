use argo_mcp::{
    CallToolParams, CallToolResult, McpContent, McpError, McpResult, McpServerService,
    McpToolSchema, ServerInfo,
};
use async_trait::async_trait;
use reqwest::Client;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::{json, Value};

pub const SLACK_BOT_TOKEN_ENV: &str = "SLACK_BOT_TOKEN";
const SLACK_API_BASE_URL_ENV: &str = "SLACK_API_BASE_URL";
const DEFAULT_SLACK_API_BASE_URL: &str = "https://slack.com/api";

#[derive(Clone)]
pub struct SlackClient {
    http: Client,
    token: String,
    base_url: String,
}

impl SlackClient {
    pub fn from_env() -> McpResult<Self> {
        let token = argo_config::cred::get(SLACK_BOT_TOKEN_ENV)
            .ok()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
            .ok_or_else(missing_token)?;
        let base_url = std::env::var(SLACK_API_BASE_URL_ENV)
            .ok()
            .map(|value| value.trim().trim_end_matches('/').to_owned())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| DEFAULT_SLACK_API_BASE_URL.to_owned());
        Ok(Self {
            http: Client::new(),
            token,
            base_url,
        })
    }

    pub fn with_token(token: impl Into<String>) -> McpResult<Self> {
        Self::with_base_url(token, DEFAULT_SLACK_API_BASE_URL)
    }

    pub fn with_base_url(token: impl Into<String>, base_url: impl Into<String>) -> McpResult<Self> {
        let token = token.into();
        if token.trim().is_empty() {
            return Err(missing_token());
        }
        let base_url = base_url.into().trim().trim_end_matches('/').to_owned();
        if base_url.is_empty() {
            return Err(McpError::Io("Slack API base URL is empty".into()));
        }
        Ok(Self {
            http: Client::new(),
            token,
            base_url,
        })
    }

    async fn post_form(&self, method: &str, form: Vec<(String, String)>) -> McpResult<Value> {
        let response = self
            .http
            .post(format!("{}/{}", self.base_url, method))
            .bearer_auth(&self.token)
            .form(&form)
            .send()
            .await
            .map_err(|err| McpError::Io(format!("Slack {method} request failed: {err}")))?;
        decode_response(method, response).await
    }

    async fn get_query(&self, method: &str, query: Vec<(String, String)>) -> McpResult<Value> {
        let response = self
            .http
            .get(format!("{}/{}", self.base_url, method))
            .bearer_auth(&self.token)
            .query(&query)
            .send()
            .await
            .map_err(|err| McpError::Io(format!("Slack {method} request failed: {err}")))?;
        decode_response(method, response).await
    }

    pub async fn post_message(&self, args: PostMessageArgs) -> McpResult<Value> {
        let mut form = vec![
            ("channel".to_owned(), args.channel),
            ("text".to_owned(), args.text),
        ];
        if let Some(thread_ts) = args.thread_ts {
            form.push(("thread_ts".to_owned(), thread_ts));
        }
        self.post_form("chat.postMessage", form).await
    }

    pub async fn list_channels(&self, args: ListChannelsArgs) -> McpResult<Value> {
        let mut query = Vec::new();
        push_optional(&mut query, "cursor", args.cursor);
        push_optional(&mut query, "types", args.types);
        push_optional_number(&mut query, "limit", args.limit);
        self.get_query("conversations.list", query).await
    }

    pub async fn read_recent(&self, args: ReadRecentArgs) -> McpResult<Value> {
        let mut query = vec![("channel".to_owned(), args.channel)];
        push_optional(&mut query, "cursor", args.cursor);
        push_optional_number(&mut query, "limit", args.limit);
        self.get_query("conversations.history", query).await
    }

    pub async fn dm_user(&self, args: DmUserArgs) -> McpResult<Value> {
        let opened = self
            .post_form("conversations.open", vec![("users".to_owned(), args.user)])
            .await?;
        let channel = opened
            .pointer("/channel/id")
            .and_then(Value::as_str)
            .ok_or_else(|| McpError::Decode("Slack conversations.open omitted channel.id".into()))?
            .to_owned();
        self.post_message(PostMessageArgs {
            channel,
            text: args.text,
            thread_ts: None,
        })
        .await
    }

    pub async fn search(&self, args: SearchArgs) -> McpResult<Value> {
        let mut query = vec![("query".to_owned(), args.query)];
        push_optional_number(&mut query, "count", args.count);
        push_optional(&mut query, "sort", args.sort);
        push_optional(&mut query, "sort_dir", args.sort_dir);
        self.get_query("search.messages", query).await
    }
}

pub struct SlackService {
    client: SlackClient,
}

impl SlackService {
    pub fn from_env() -> McpResult<Self> {
        Ok(Self {
            client: SlackClient::from_env()?,
        })
    }

    pub fn new(client: SlackClient) -> Self {
        Self { client }
    }
}

#[async_trait]
impl McpServerService for SlackService {
    fn server_info(&self) -> ServerInfo {
        ServerInfo {
            name: "argo-mcp-connector-slack".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        }
    }

    fn list_tools(&self) -> Vec<McpToolSchema> {
        slack_tools()
    }

    async fn call_tool(&self, params: CallToolParams) -> McpResult<CallToolResult> {
        let value = match params.name.as_str() {
            "slack.post_message" => {
                self.client
                    .post_message(parse_args(params.arguments)?)
                    .await?
            }
            "slack.list_channels" => {
                self.client
                    .list_channels(parse_args(params.arguments)?)
                    .await?
            }
            "slack.read_recent" => {
                self.client
                    .read_recent(parse_args(params.arguments)?)
                    .await?
            }
            "slack.dm_user" => self.client.dm_user(parse_args(params.arguments)?).await?,
            "slack.search" => self.client.search(parse_args(params.arguments)?).await?,
            other => return Err(McpError::Io(format!("unknown Slack MCP tool: {other}"))),
        };
        Ok(text_result(value))
    }
}

#[derive(Debug, Deserialize)]
pub struct PostMessageArgs {
    channel: String,
    text: String,
    #[serde(default)]
    thread_ts: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListChannelsArgs {
    #[serde(default)]
    limit: Option<u32>,
    #[serde(default)]
    cursor: Option<String>,
    #[serde(default)]
    types: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ReadRecentArgs {
    channel: String,
    #[serde(default)]
    limit: Option<u32>,
    #[serde(default)]
    cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DmUserArgs {
    user: String,
    text: String,
}

#[derive(Debug, Deserialize)]
pub struct SearchArgs {
    query: String,
    #[serde(default)]
    count: Option<u32>,
    #[serde(default)]
    sort: Option<String>,
    #[serde(default)]
    sort_dir: Option<String>,
}

fn slack_tools() -> Vec<McpToolSchema> {
    vec![
        tool(
            "slack.post_message",
            "Post a message to a Slack channel.",
            json!({
                "channel": {"type": "string"},
                "text": {"type": "string"},
                "thread_ts": {"type": "string"}
            }),
            &["channel", "text"],
        ),
        tool(
            "slack.list_channels",
            "List Slack conversations visible to the bot token.",
            json!({
                "limit": {"type": "integer", "minimum": 1, "maximum": 1000},
                "cursor": {"type": "string"},
                "types": {"type": "string"}
            }),
            &[],
        ),
        tool(
            "slack.read_recent",
            "Read recent messages from a Slack channel.",
            json!({
                "channel": {"type": "string"},
                "limit": {"type": "integer", "minimum": 1, "maximum": 1000},
                "cursor": {"type": "string"}
            }),
            &["channel"],
        ),
        tool(
            "slack.dm_user",
            "Send a direct message to a Slack user.",
            json!({
                "user": {"type": "string"},
                "text": {"type": "string"}
            }),
            &["user", "text"],
        ),
        tool(
            "slack.search",
            "Search Slack messages visible to the bot token.",
            json!({
                "query": {"type": "string"},
                "count": {"type": "integer", "minimum": 1, "maximum": 100},
                "sort": {"type": "string"},
                "sort_dir": {"type": "string"}
            }),
            &["query"],
        ),
    ]
}

fn tool(name: &str, description: &str, properties: Value, required: &[&str]) -> McpToolSchema {
    McpToolSchema {
        name: name.into(),
        description: description.into(),
        input_schema: json!({
            "type": "object",
            "properties": properties,
            "required": required,
            "additionalProperties": false
        }),
    }
}

fn parse_args<T: DeserializeOwned>(arguments: Value) -> McpResult<T> {
    serde_json::from_value(arguments)
        .map_err(|err| McpError::Decode(format!("invalid Slack MCP tool arguments: {err}")))
}

fn text_result(value: Value) -> CallToolResult {
    CallToolResult {
        content: vec![McpContent::Text {
            text: value.to_string(),
        }],
        is_error: false,
    }
}

fn push_optional(query: &mut Vec<(String, String)>, name: &str, value: Option<String>) {
    if let Some(value) = value.filter(|value| !value.trim().is_empty()) {
        query.push((name.to_owned(), value));
    }
}

fn push_optional_number(query: &mut Vec<(String, String)>, name: &str, value: Option<u32>) {
    if let Some(value) = value {
        query.push((name.to_owned(), value.to_string()));
    }
}

async fn decode_response(method: &str, response: reqwest::Response) -> McpResult<Value> {
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|err| McpError::Io(format!("Slack {method} response read failed: {err}")))?;
    if !status.is_success() {
        return Err(McpError::Io(format!(
            "Slack {method} returned HTTP {status}: {body}"
        )));
    }
    let value: Value = serde_json::from_str(&body)
        .map_err(|err| McpError::Decode(format!("Slack {method} returned invalid JSON: {err}")))?;
    if value.get("ok").and_then(Value::as_bool) == Some(false) {
        let error = value
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or("unknown_error");
        return Err(McpError::Io(format!("Slack {method} API error: {error}")));
    }
    Ok(value)
}

fn missing_token() -> McpError {
    McpError::Io(format!(
        "{SLACK_BOT_TOKEN_ENV} is required for Slack MCP connector"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advertises_all_slack_tools() {
        let client = SlackClient::with_token("xoxb-test").expect("test token should be valid");
        let service = SlackService::new(client);
        let mut names: Vec<_> = service
            .list_tools()
            .into_iter()
            .map(|tool| tool.name)
            .collect();
        names.sort();
        assert_eq!(
            names,
            vec![
                "slack.dm_user",
                "slack.list_channels",
                "slack.post_message",
                "slack.read_recent",
                "slack.search"
            ]
        );
    }

    #[test]
    fn empty_token_fails_clearly() {
        match SlackClient::with_token(" ") {
            Ok(_) => panic!("blank tokens should fail"),
            Err(err) => assert!(err.to_string().contains(SLACK_BOT_TOKEN_ENV)),
        }
    }
}
