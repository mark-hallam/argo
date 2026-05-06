//! ClickUp MCP service implementation.
use crate::client::{to_call_result, ClickUpClient};
use argo_mcp::{
    CallToolParams, CallToolResult, McpContent, McpResult, McpServerService, McpToolSchema,
    ServerInfo,
};
use async_trait::async_trait;

/// ClickUp MCP service.
#[derive(Default)]
pub struct ClickUpService;

#[async_trait]
impl McpServerService for ClickUpService {
    fn server_info(&self) -> ServerInfo {
        ServerInfo {
            name: "argo-mcp-connector-clickup".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        }
    }

    fn list_tools(&self) -> Vec<McpToolSchema> {
        crate::schemas::all()
    }

    async fn call_tool(&self, params: CallToolParams) -> McpResult<CallToolResult> {
        let client = match ClickUpClient::from_env() {
            Ok(c) => c,
            Err(text) => {
                return Ok(CallToolResult {
                    content: vec![McpContent::Text { text }],
                    is_error: true,
                });
            }
        };
        let out = match params.name.as_str() {
            "clickup.list_workspaces" => client.list_workspaces(&params.arguments).await,
            "clickup.list_spaces" => client.list_spaces(&params.arguments).await,
            "clickup.list_lists" => client.list_lists(&params.arguments).await,
            "clickup.list_tasks" => client.list_tasks(&params.arguments).await,
            "clickup.get_task" => client.get_task(&params.arguments).await,
            "clickup.create_task" => client.create_task(&params.arguments).await,
            "clickup.update_task" => client.update_task(&params.arguments).await,
            "clickup.add_comment" => client.add_comment(&params.arguments).await,
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
    async fn list_advertises_eight_clickup_tools() {
        let tools = ClickUpService.list_tools();
        let names: Vec<&str> = tools.iter().map(|tool| tool.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "clickup.list_workspaces",
                "clickup.list_spaces",
                "clickup.list_lists",
                "clickup.list_tasks",
                "clickup.get_task",
                "clickup.create_task",
                "clickup.update_task",
                "clickup.add_comment",
            ]
        );
    }

    #[tokio::test]
    async fn missing_token_returns_tool_error() {
        std::env::remove_var(crate::auth::CLICKUP_TOKEN_ENV);
        let result = ClickUpService
            .call_tool(CallToolParams {
                name: "clickup.list_workspaces".into(),
                arguments: serde_json::json!({}),
            })
            .await
            .expect("tool result");
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
        argo_mcp::run_loop(&ClickUpService, input, &mut out)
            .await
            .expect("run loop");
        let line = String::from_utf8(out).expect("utf8");
        let value: serde_json::Value = serde_json::from_str(line.trim()).expect("json");
        let tools = value["result"]["tools"].as_array().expect("tools");
        assert_eq!(tools.len(), 8);
        assert_eq!(tools[0]["inputSchema"]["approval_required"], false);
        assert_eq!(tools[5]["inputSchema"]["approval_required"], true);
    }
}
