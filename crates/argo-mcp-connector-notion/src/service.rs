//! MCP service implementation for Notion.
use crate::client::{NotionClient, NotionConfig};
use argo_mcp::{
    CallToolParams, CallToolResult, McpContent, McpError, McpResult, McpServerService,
    McpToolSchema, ServerInfo,
};
use async_trait::async_trait;
use serde_json::{json, Value};

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// MCP service exposing Notion tools.
#[derive(Debug, Clone)]
pub struct NotionMcpService {
    client: NotionClient,
}

impl NotionMcpService {
    /// Build from an existing API client.
    #[must_use]
    pub fn new(client: NotionClient) -> Self {
        Self { client }
    }

    /// Build from `NOTION_INTEGRATION_TOKEN` or `NOTION_TEST_TOKEN`.
    pub fn from_env() -> McpResult<Self> {
        Ok(Self::new(NotionClient::new(NotionConfig::from_env()?)))
    }
}

#[async_trait]
impl McpServerService for NotionMcpService {
    fn server_info(&self) -> ServerInfo {
        ServerInfo {
            name: "notion".into(),
            version: VERSION.into(),
        }
    }

    fn list_tools(&self) -> Vec<McpToolSchema> {
        vec![
            tool(
                "notion.search",
                "Search Notion pages and databases.",
                json!({
                    "type": "object",
                    "properties": {
                        "query": {"type": "string"},
                        "page_size": {"type": "integer"},
                        "filter": {"type": "object"},
                        "sort": {"type": "object"}
                    }
                }),
            ),
            tool(
                "notion.read_page",
                "Read a Notion page object.",
                json!({
                    "type": "object",
                    "required": ["page_id"],
                    "properties": {"page_id": {"type": "string"}}
                }),
            ),
            tool(
                "notion.create_page",
                "Create a Notion page.",
                json!({
                    "type": "object",
                    "required": ["parent", "properties"],
                    "properties": {
                        "parent": {"type": "object"},
                        "properties": {"type": "object"},
                        "children": {"type": "array"}
                    }
                }),
            ),
            tool(
                "notion.append_block",
                "Append child blocks under a Notion block.",
                json!({
                    "type": "object",
                    "required": ["block_id", "children"],
                    "properties": {
                        "block_id": {"type": "string"},
                        "children": {"type": "array"}
                    }
                }),
            ),
            tool(
                "notion.update_database",
                "Update a Notion database schema.",
                json!({
                    "type": "object",
                    "required": ["database_id"],
                    "properties": {
                        "database_id": {"type": "string"},
                        "title": {"type": "array"},
                        "description": {"type": "array"},
                        "properties": {"type": "object"}
                    }
                }),
            ),
        ]
    }

    async fn call_tool(&self, params: CallToolParams) -> McpResult<CallToolResult> {
        let value = match params.name.as_str() {
            "notion.search" => self.client.search(&params.arguments).await?,
            "notion.read_page" => self.client.read_page(&params.arguments).await?,
            "notion.create_page" => self.client.create_page(&params.arguments).await?,
            "notion.append_block" => self.client.append_block(&params.arguments).await?,
            "notion.update_database" => self.client.update_database(&params.arguments).await?,
            other => {
                return Err(McpError::Rpc {
                    code: -32602,
                    message: format!("unknown notion tool: {other}"),
                    data: None,
                })
            }
        };
        Ok(json_result(value))
    }
}

fn tool(name: &str, description: &str, input_schema: Value) -> McpToolSchema {
    McpToolSchema {
        name: name.into(),
        description: description.into(),
        input_schema,
    }
}

fn json_result(value: Value) -> CallToolResult {
    CallToolResult {
        content: vec![McpContent::Text {
            text: serde_json::to_string(&value).unwrap_or_else(|_| "{}".into()),
        }],
        is_error: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn service() -> NotionMcpService {
        NotionMcpService::new(NotionClient::new(NotionConfig {
            integration_token: "test".into(),
            base_url: "http://127.0.0.1:1".into(),
            notion_version: "2022-06-28".into(),
        }))
    }

    #[test]
    fn advertises_all_five_tools() {
        let names: Vec<String> = service().list_tools().into_iter().map(|t| t.name).collect();
        assert_eq!(names.len(), 5);
        assert!(names.contains(&"notion.search".into()));
        assert!(names.contains(&"notion.read_page".into()));
        assert!(names.contains(&"notion.create_page".into()));
        assert!(names.contains(&"notion.append_block".into()));
        assert!(names.contains(&"notion.update_database".into()));
    }
}
