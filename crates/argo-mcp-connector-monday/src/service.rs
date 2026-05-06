//! Monday.com MCP service implementation.
use crate::client::{to_call_result, MondayClient};
use argo_mcp::{
    CallToolParams, CallToolResult, McpContent, McpResult, McpServerService, McpToolSchema,
    ServerInfo,
};
use async_trait::async_trait;

/// Monday.com MCP service.
#[derive(Default)]
pub struct MondayService;

#[async_trait]
impl McpServerService for MondayService {
    fn server_info(&self) -> ServerInfo {
        ServerInfo {
            name: "argo-mcp-connector-monday".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        }
    }

    fn list_tools(&self) -> Vec<McpToolSchema> {
        crate::schemas::all()
    }

    async fn call_tool(&self, params: CallToolParams) -> McpResult<CallToolResult> {
        let client = match MondayClient::from_env() {
            Ok(c) => c,
            Err(text) => {
                return Ok(CallToolResult {
                    content: vec![McpContent::Text { text }],
                    is_error: true,
                });
            }
        };
        let out = match params.name.as_str() {
            "monday.list_boards" => client.list_boards(&params.arguments).await,
            "monday.list_items" => client.list_items(&params.arguments).await,
            "monday.get_item" => client.get_item(&params.arguments).await,
            "monday.create_item" => client.create_item(&params.arguments).await,
            "monday.update_item_status" => client.update_item_status(&params.arguments).await,
            "monday.add_update" => client.add_update(&params.arguments).await,
            "monday.list_groups" => client.list_groups(&params.arguments).await,
            other => Err(format!("unknown tool: {other}")),
        };
        Ok(to_call_result(out))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[tokio::test]
    async fn list_advertises_seven_monday_tools() {
        let tools = MondayService.list_tools();
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "monday.list_boards",
                "monday.list_items",
                "monday.get_item",
                "monday.create_item",
                "monday.update_item_status",
                "monday.add_update",
                "monday.list_groups"
            ]
        );
    }

    #[tokio::test]
    async fn missing_token_returns_tool_error() {
        std::env::remove_var(crate::auth::MONDAY_API_TOKEN_ENV);
        let result = MondayService
            .call_tool(CallToolParams {
                name: "monday.list_boards".into(),
                arguments: serde_json::json!({}),
            })
            .await
            .unwrap();
        assert!(result.is_error);
    }

    #[tokio::test]
    async fn stdio_wire_lists_tools() {
        let input = Cursor::new(
            br#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}
"#
            .to_vec(),
        );
        let mut out = Vec::new();
        argo_mcp::run_loop(&MondayService, input, &mut out)
            .await
            .unwrap();
        let line = String::from_utf8(out).unwrap();
        let value: serde_json::Value = serde_json::from_str(line.trim()).unwrap();
        let tools = value["result"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 7);
        assert_eq!(tools[3]["inputSchema"]["approval_required"], true);
        assert_eq!(tools[6]["inputSchema"]["approval_required"], false);
    }
}
