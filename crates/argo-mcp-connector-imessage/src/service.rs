//! iMessage MCP service implementation.

use crate::client::{to_call_result, IMessageMcpClient};
use argo_mcp::{
    CallToolParams, CallToolResult, McpContent, McpResult, McpServerService, McpToolSchema,
    ServerInfo,
};
use async_trait::async_trait;

/// Blue Bubbles iMessage MCP service.
#[derive(Default)]
pub struct IMessageService;

#[async_trait]
impl McpServerService for IMessageService {
    fn server_info(&self) -> ServerInfo {
        ServerInfo {
            name: "argo-mcp-connector-imessage".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        }
    }

    fn list_tools(&self) -> Vec<McpToolSchema> {
        crate::schemas::all()
    }

    async fn call_tool(&self, params: CallToolParams) -> McpResult<CallToolResult> {
        let client = match IMessageMcpClient::from_env() {
            Ok(c) => c,
            Err(text) => {
                return Ok(CallToolResult {
                    content: vec![McpContent::Text { text }],
                    is_error: true,
                });
            }
        };
        let out = match params.name.as_str() {
            "imessage.send_message" => client.send_message(&params.arguments).await,
            "imessage.list_chats" => client.list_chats(&params.arguments).await,
            "imessage.list_messages" => client.list_messages(&params.arguments).await,
            "imessage.get_message" => client.get_message(&params.arguments).await,
            "imessage.send_reaction" => client.send_reaction(&params.arguments).await,
            other => Err(format!("unknown imessage tool: {other}")),
        };
        Ok(to_call_result(out))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[tokio::test]
    async fn list_advertises_five_imessage_tools() {
        let tools = IMessageService.list_tools();
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "imessage.send_message",
                "imessage.list_chats",
                "imessage.list_messages",
                "imessage.get_message",
                "imessage.send_reaction"
            ]
        );
    }

    #[tokio::test]
    async fn missing_env_returns_tool_error() {
        std::env::remove_var(crate::auth::BLUE_BUBBLES_URL_ENV);
        std::env::remove_var(crate::auth::BLUE_BUBBLES_PASSWORD_ENV);
        let result = IMessageService
            .call_tool(CallToolParams {
                name: "imessage.list_chats".into(),
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
        argo_mcp::run_loop(&IMessageService, input, &mut out)
            .await
            .unwrap();
        let line = String::from_utf8(out).unwrap();
        let value: serde_json::Value = serde_json::from_str(line.trim()).unwrap();
        let tools = value["result"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 5);
        assert_eq!(tools[0]["inputSchema"]["approval_required"], true);
        assert_eq!(tools[1]["inputSchema"]["approval_required"], false);
    }
}
