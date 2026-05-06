//! Linear MCP service implementation.
use crate::client::{to_call_result, LinearClient};
use argo_mcp::{
    CallToolParams, CallToolResult, McpContent, McpResult, McpServerService, McpToolSchema,
    ServerInfo,
};
use async_trait::async_trait;

/// Linear MCP service.
#[derive(Default)]
pub struct LinearService;

#[async_trait]
impl McpServerService for LinearService {
    fn server_info(&self) -> ServerInfo {
        ServerInfo {
            name: "argo-mcp-connector-linear".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        }
    }

    fn list_tools(&self) -> Vec<McpToolSchema> {
        crate::schemas::all()
    }

    async fn call_tool(&self, params: CallToolParams) -> McpResult<CallToolResult> {
        let client = match LinearClient::from_env() {
            Ok(c) => c,
            Err(text) => {
                return Ok(CallToolResult {
                    content: vec![McpContent::Text { text }],
                    is_error: true,
                });
            }
        };
        let out = match params.name.as_str() {
            "linear.list_teams" => client.list_teams(&params.arguments).await,
            "linear.list_issues" => client.list_issues(&params.arguments).await,
            "linear.get_issue" => client.get_issue(&params.arguments).await,
            "linear.create_issue" => client.create_issue(&params.arguments).await,
            "linear.update_issue" => client.update_issue(&params.arguments).await,
            "linear.add_comment" => client.add_comment(&params.arguments).await,
            "linear.list_workflows" => client.list_workflows(&params.arguments).await,
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
    async fn list_advertises_seven_linear_tools() {
        let tools = LinearService.list_tools();
        let names: Vec<&str> = tools.iter().map(|tool| tool.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "linear.list_teams",
                "linear.list_issues",
                "linear.get_issue",
                "linear.create_issue",
                "linear.update_issue",
                "linear.add_comment",
                "linear.list_workflows",
            ]
        );
    }

    #[tokio::test]
    async fn missing_api_key_returns_tool_error() {
        std::env::remove_var(crate::auth::LINEAR_API_KEY_ENV);
        let result = LinearService
            .call_tool(CallToolParams {
                name: "linear.list_teams".into(),
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
        argo_mcp::run_loop(&LinearService, input, &mut out)
            .await
            .expect("run loop");
        let line = String::from_utf8(out).expect("utf8");
        let value: serde_json::Value = serde_json::from_str(line.trim()).expect("json");
        let tools = value["result"]["tools"].as_array().expect("tools");
        assert_eq!(tools.len(), 7);
        assert_eq!(tools[3]["inputSchema"]["approval_required"], true);
    }
}
