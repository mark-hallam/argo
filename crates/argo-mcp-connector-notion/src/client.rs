//! Thin Notion REST client.
use argo_mcp::{McpError, McpResult};
use reqwest::Method;
use serde_json::{json, Value};

const DEFAULT_BASE_URL: &str = "https://api.notion.com/v1";
const DEFAULT_NOTION_VERSION: &str = "2022-06-28";

/// Runtime configuration for the Notion connector.
#[derive(Debug, Clone)]
pub struct NotionConfig {
    /// Notion integration bearer token.
    pub integration_token: String,
    /// Notion API base URL, overridable for tests.
    pub base_url: String,
    /// Notion API version header.
    pub notion_version: String,
}

impl NotionConfig {
    /// Load config from environment.
    pub fn from_env() -> McpResult<Self> {
        let integration_token = argo_config::cred::get("NOTION_INTEGRATION_TOKEN")
            .or_else(|_| argo_config::cred::get("NOTION_TEST_TOKEN"))
            .map_err(|_| McpError::Initialize("NOTION_INTEGRATION_TOKEN is required".into()))?;
        Ok(Self {
            integration_token,
            base_url: std::env::var("NOTION_BASE_URL").unwrap_or_else(|_| DEFAULT_BASE_URL.into()),
            notion_version: std::env::var("NOTION_VERSION")
                .unwrap_or_else(|_| DEFAULT_NOTION_VERSION.into()),
        })
    }
}

/// Notion API client.
#[derive(Debug, Clone)]
pub struct NotionClient {
    http: reqwest::Client,
    config: NotionConfig,
}

impl NotionClient {
    /// Construct a client from explicit config.
    #[must_use]
    pub fn new(config: NotionConfig) -> Self {
        Self {
            http: reqwest::Client::new(),
            config,
        }
    }

    /// Search pages and databases.
    pub async fn search(&self, args: &Value) -> McpResult<Value> {
        let mut body = serde_json::Map::new();
        if let Some(query) = optional_str(args, "query") {
            body.insert("query".into(), json!(query));
        }
        if let Some(page_size) = args.get("page_size").and_then(Value::as_u64) {
            body.insert("page_size".into(), json!(page_size));
        }
        if let Some(filter) = args.get("filter") {
            body.insert("filter".into(), filter.clone());
        }
        if let Some(sort) = args.get("sort") {
            body.insert("sort".into(), sort.clone());
        }
        self.request(Method::POST, "search", Some(Value::Object(body)))
            .await
    }

    /// Read a page object.
    pub async fn read_page(&self, args: &Value) -> McpResult<Value> {
        let page_id = required_str(args, "page_id")?;
        self.request(Method::GET, &format!("pages/{}", encode(page_id)), None)
            .await
    }

    /// Create a page.
    pub async fn create_page(&self, args: &Value) -> McpResult<Value> {
        let parent = args
            .get("parent")
            .cloned()
            .ok_or_else(|| McpError::Initialize("missing parent".into()))?;
        let properties = args
            .get("properties")
            .cloned()
            .ok_or_else(|| McpError::Initialize("missing properties".into()))?;
        let mut body = serde_json::Map::new();
        body.insert("parent".into(), parent);
        body.insert("properties".into(), properties);
        if let Some(children) = args.get("children") {
            body.insert("children".into(), children.clone());
        }
        self.request(Method::POST, "pages", Some(Value::Object(body)))
            .await
    }

    /// Append child blocks under a block.
    pub async fn append_block(&self, args: &Value) -> McpResult<Value> {
        let block_id = required_str(args, "block_id")?;
        let children = args
            .get("children")
            .cloned()
            .ok_or_else(|| McpError::Initialize("missing children".into()))?;
        let body = json!({ "children": children });
        self.request(
            Method::PATCH,
            &format!("blocks/{}/children", encode(block_id)),
            Some(body),
        )
        .await
    }

    /// Update a database schema.
    pub async fn update_database(&self, args: &Value) -> McpResult<Value> {
        let database_id = required_str(args, "database_id")?;
        let mut body = serde_json::Map::new();
        if let Some(title) = args.get("title") {
            body.insert("title".into(), title.clone());
        }
        if let Some(description) = args.get("description") {
            body.insert("description".into(), description.clone());
        }
        if let Some(properties) = args.get("properties") {
            body.insert("properties".into(), properties.clone());
        }
        self.request(
            Method::PATCH,
            &format!("databases/{}", encode(database_id)),
            Some(Value::Object(body)),
        )
        .await
    }

    async fn request(&self, method: Method, path: &str, body: Option<Value>) -> McpResult<Value> {
        let url = format!("{}/{}", self.config.base_url.trim_end_matches('/'), path);
        let mut req = self
            .http
            .request(method, url)
            .bearer_auth(&self.config.integration_token)
            .header("Notion-Version", &self.config.notion_version);
        if let Some(body) = body {
            req = req.json(&body);
        }
        let response = req
            .send()
            .await
            .map_err(|e| McpError::Io(format!("notion request: {e}")))?;
        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(|e| McpError::Io(format!("notion response: {e}")))?;
        let parsed =
            serde_json::from_str::<Value>(&text).unwrap_or_else(|_| json!({ "raw": text }));
        if !status.is_success() {
            return Err(McpError::Rpc {
                code: i64::from(status.as_u16()),
                message: "notion api error".into(),
                data: Some(parsed),
            });
        }
        Ok(parsed)
    }
}

pub(crate) fn required_str<'a>(args: &'a Value, key: &str) -> McpResult<&'a str> {
    optional_str(args, key).ok_or_else(|| McpError::Initialize(format!("missing {key}")))
}

fn optional_str<'a>(args: &'a Value, key: &str) -> Option<&'a str> {
    args.get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
}

fn encode(segment: &str) -> String {
    urlencoding::encode(segment).into_owned()
}
