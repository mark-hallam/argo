//! Postmark MCP service implementation.

use crate::client::{to_call_result, PostmarkClient};
use argo_mcp::{
    CallToolParams, CallToolResult, McpContent, McpResult, McpServerService, McpToolSchema,
    ServerInfo,
};
use async_trait::async_trait;

/// Postmark MCP service.
#[derive(Default)]
pub struct PostmarkService;

#[async_trait]
impl McpServerService for PostmarkService {
    fn server_info(&self) -> ServerInfo {
        ServerInfo {
            name: "argo-mcp-connector-postmark".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        }
    }

    fn list_tools(&self) -> Vec<McpToolSchema> {
        crate::schemas::all()
    }

    async fn call_tool(&self, params: CallToolParams) -> McpResult<CallToolResult> {
        let client = match PostmarkClient::from_env() {
            Ok(c) => c,
            Err(text) => {
                return Ok(CallToolResult {
                    content: vec![McpContent::Text { text }],
                    is_error: true,
                });
            }
        };
        let out = match params.name.as_str() {
            "postmark.list_servers" => client.list_servers(&params.arguments).await,
            "postmark.send_email" => client.send_email(&params.arguments).await,
            "postmark.list_messages" => client.list_messages(&params.arguments).await,
            "postmark.get_message" => client.get_message(&params.arguments).await,
            "postmark.list_templates" => client.list_templates(&params.arguments).await,
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
    async fn list_advertises_five_postmark_tools() {
        let tools = PostmarkService.list_tools();
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "postmark.list_servers",
                "postmark.send_email",
                "postmark.list_messages",
                "postmark.get_message",
                "postmark.list_templates"
            ]
        );
    }

    #[tokio::test]
    async fn missing_auth_env_returns_tool_error() {
        std::env::remove_var(crate::auth::POSTMARK_SERVER_TOKEN_ENV);
        let result = PostmarkService
            .call_tool(CallToolParams {
                name: "postmark.list_servers".into(),
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
        argo_mcp::run_loop(&PostmarkService, input, &mut out)
            .await
            .unwrap();
        let line = String::from_utf8(out).unwrap();
        let value: serde_json::Value = serde_json::from_str(line.trim()).unwrap();
        let tools = value["result"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 5);
        assert_eq!(tools[1]["inputSchema"]["approval_required"], true);
    }
}
