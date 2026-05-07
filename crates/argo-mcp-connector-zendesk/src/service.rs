//! Zendesk MCP service implementation.

use crate::client::{to_call_result, ZendeskClient};
use argo_mcp::{
    CallToolParams, CallToolResult, McpContent, McpResult, McpServerService, McpToolSchema,
    ServerInfo,
};
use async_trait::async_trait;

/// Zendesk MCP service.
#[derive(Default)]
pub struct ZendeskService;

#[async_trait]
impl McpServerService for ZendeskService {
    fn server_info(&self) -> ServerInfo {
        ServerInfo {
            name: "argo-mcp-connector-zendesk".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        }
    }

    fn list_tools(&self) -> Vec<McpToolSchema> {
        crate::schemas::all()
    }

    async fn call_tool(&self, params: CallToolParams) -> McpResult<CallToolResult> {
        let client = match ZendeskClient::from_env() {
            Ok(c) => c,
            Err(text) => {
                return Ok(CallToolResult {
                    content: vec![McpContent::Text { text }],
                    is_error: true,
                });
            }
        };
        let out = match params.name.as_str() {
            "zendesk.list_tickets" => client.list_tickets(&params.arguments).await,
            "zendesk.get_ticket" => client.get_ticket(&params.arguments).await,
            "zendesk.create_ticket" => client.create_ticket(&params.arguments).await,
            "zendesk.update_ticket" => client.update_ticket(&params.arguments).await,
            "zendesk.add_comment" => client.add_comment(&params.arguments).await,
            "zendesk.list_users" => client.list_users(&params.arguments).await,
            "zendesk.list_macros" => client.list_macros(&params.arguments).await,
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
    async fn list_advertises_seven_zendesk_tools() {
        let tools = ZendeskService.list_tools();
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "zendesk.list_tickets",
                "zendesk.get_ticket",
                "zendesk.create_ticket",
                "zendesk.update_ticket",
                "zendesk.add_comment",
                "zendesk.list_users",
                "zendesk.list_macros"
            ]
        );
    }

    #[tokio::test]
    async fn missing_auth_env_returns_tool_error() {
        std::env::remove_var(crate::auth::ZENDESK_EMAIL_ENV);
        let result = ZendeskService
            .call_tool(CallToolParams {
                name: "zendesk.list_tickets".into(),
                arguments: serde_json::json!({"query": "status:open"}),
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
        argo_mcp::run_loop(&ZendeskService, input, &mut out)
            .await
            .unwrap();
        let line = String::from_utf8(out).unwrap();
        let value: serde_json::Value = serde_json::from_str(line.trim()).unwrap();
        let tools = value["result"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 7);
        assert_eq!(tools[2]["inputSchema"]["approval_required"], true);
    }
}
