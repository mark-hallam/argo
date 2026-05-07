//! Trello MCP service implementation.

use crate::client::{to_call_result, TrelloClient};
use argo_mcp::{
    CallToolParams, CallToolResult, McpContent, McpResult, McpServerService, McpToolSchema,
    ServerInfo,
};
use async_trait::async_trait;

/// Trello MCP service.
#[derive(Default)]
pub struct TrelloService;

#[async_trait]
impl McpServerService for TrelloService {
    fn server_info(&self) -> ServerInfo {
        ServerInfo {
            name: "argo-mcp-connector-trello".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        }
    }

    fn list_tools(&self) -> Vec<McpToolSchema> {
        crate::schemas::all()
    }

    async fn call_tool(&self, params: CallToolParams) -> McpResult<CallToolResult> {
        let client = match TrelloClient::from_env() {
            Ok(c) => c,
            Err(text) => {
                return Ok(CallToolResult {
                    content: vec![McpContent::Text { text }],
                    is_error: true,
                });
            }
        };
        let out = match params.name.as_str() {
            "trello.list_boards" => client.list_boards(&params.arguments).await,
            "trello.list_cards" => client.list_cards(&params.arguments).await,
            "trello.get_card" => client.get_card(&params.arguments).await,
            "trello.create_card" => client.create_card(&params.arguments).await,
            "trello.update_card" => client.update_card(&params.arguments).await,
            "trello.add_comment" => client.add_comment(&params.arguments).await,
            "trello.move_card" => client.move_card(&params.arguments).await,
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
    async fn list_advertises_seven_trello_tools() {
        let tools = TrelloService.list_tools();
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "trello.list_boards",
                "trello.list_cards",
                "trello.get_card",
                "trello.create_card",
                "trello.update_card",
                "trello.add_comment",
                "trello.move_card"
            ]
        );
    }

    #[tokio::test]
    async fn missing_auth_env_returns_tool_error() {
        std::env::remove_var(crate::auth::TRELLO_KEY_ENV);
        let result = TrelloService
            .call_tool(CallToolParams {
                name: "trello.list_boards".into(),
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
        argo_mcp::run_loop(&TrelloService, input, &mut out)
            .await
            .unwrap();
        let line = String::from_utf8(out).unwrap();
        let value: serde_json::Value = serde_json::from_str(line.trim()).unwrap();
        let tools = value["result"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 7);
        assert_eq!(tools[3]["inputSchema"]["approval_required"], true);
    }
}
