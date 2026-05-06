//! Asana MCP service implementation.
use crate::client::{to_call_result, AsanaClient};
use argo_mcp::{
    CallToolParams, CallToolResult, McpContent, McpResult, McpServerService, McpToolSchema,
    ServerInfo,
};
use async_trait::async_trait;

/// Asana MCP service.
#[derive(Default)]
pub struct AsanaService;

#[async_trait]
impl McpServerService for AsanaService {
    fn server_info(&self) -> ServerInfo {
        ServerInfo {
            name: "argo-mcp-connector-asana".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        }
    }

    fn list_tools(&self) -> Vec<McpToolSchema> {
        crate::schemas::all()
    }

    async fn call_tool(&self, params: CallToolParams) -> McpResult<CallToolResult> {
        let client = match AsanaClient::from_env() {
            Ok(c) => c,
            Err(text) => {
                return Ok(CallToolResult {
                    content: vec![McpContent::Text { text }],
                    is_error: true,
                });
            }
        };
        let out = match params.name.as_str() {
            "asana.list_workspaces" => client.list_workspaces(&params.arguments).await,
            "asana.list_projects" => client.list_projects(&params.arguments).await,
            "asana.list_tasks" => client.list_tasks(&params.arguments).await,
            "asana.get_task" => client.get_task(&params.arguments).await,
            "asana.create_task" => client.create_task(&params.arguments).await,
            "asana.update_task" => client.update_task(&params.arguments).await,
            "asana.complete_task" => client.complete_task(&params.arguments).await,
            "asana.add_comment" => client.add_comment(&params.arguments).await,
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
    async fn list_advertises_eight_asana_tools() {
        let tools = AsanaService.list_tools();
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "asana.list_workspaces",
                "asana.list_projects",
                "asana.list_tasks",
                "asana.get_task",
                "asana.create_task",
                "asana.update_task",
                "asana.complete_task",
                "asana.add_comment"
            ]
        );
    }

    #[tokio::test]
    async fn missing_pat_returns_tool_error() {
        std::env::remove_var(crate::auth::ASANA_PAT_ENV);
        let result = AsanaService
            .call_tool(CallToolParams {
                name: "asana.list_workspaces".into(),
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
        argo_mcp::run_loop(&AsanaService, input, &mut out)
            .await
            .unwrap();
        let line = String::from_utf8(out).unwrap();
        let value: serde_json::Value = serde_json::from_str(line.trim()).unwrap();
        let tools = value["result"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 8);
        assert_eq!(tools[4]["inputSchema"]["approval_required"], true);
    }
}
