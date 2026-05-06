//! Fireflies.ai MCP service implementation.
use crate::client::{to_call_result, FirefliesClient};
use argo_mcp::{
    CallToolParams, CallToolResult, McpContent, McpResult, McpServerService, McpToolSchema,
    ServerInfo,
};
use async_trait::async_trait;

/// Fireflies.ai MCP service.
#[derive(Default)]
pub struct FirefliesService;

#[async_trait]
impl McpServerService for FirefliesService {
    fn server_info(&self) -> ServerInfo {
        ServerInfo {
            name: "argo-mcp-connector-fireflies".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        }
    }

    fn list_tools(&self) -> Vec<McpToolSchema> {
        crate::schemas::all()
    }

    async fn call_tool(&self, params: CallToolParams) -> McpResult<CallToolResult> {
        let client = match FirefliesClient::from_env() {
            Ok(c) => c,
            Err(text) => {
                return Ok(CallToolResult {
                    content: vec![McpContent::Text { text }],
                    is_error: true,
                });
            }
        };
        let out = match params.name.as_str() {
            "fireflies.list_meetings" => client.list_meetings(&params.arguments).await,
            "fireflies.get_meeting" => client.get_meeting(&params.arguments).await,
            "fireflies.get_transcript" => client.get_transcript(&params.arguments).await,
            "fireflies.search_transcripts" => client.search_transcripts(&params.arguments).await,
            "fireflies.list_users" => client.list_users(&params.arguments).await,
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
    async fn list_advertises_five_fireflies_tools() {
        let tools = FirefliesService.list_tools();
        let names: Vec<&str> = tools.iter().map(|tool| tool.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "fireflies.list_meetings",
                "fireflies.get_meeting",
                "fireflies.get_transcript",
                "fireflies.search_transcripts",
                "fireflies.list_users",
            ]
        );
    }

    #[tokio::test]
    async fn missing_api_key_returns_tool_error() {
        std::env::remove_var(crate::auth::FIREFLIES_API_KEY_ENV);
        let result = FirefliesService
            .call_tool(CallToolParams {
                name: "fireflies.list_meetings".into(),
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
        argo_mcp::run_loop(&FirefliesService, input, &mut out)
            .await
            .expect("run loop");
        let line = String::from_utf8(out).expect("utf8");
        let value: serde_json::Value = serde_json::from_str(line.trim()).expect("json");
        let tools = value["result"]["tools"].as_array().expect("tools");
        assert_eq!(tools.len(), 5);
        assert_eq!(tools[0]["inputSchema"]["approval_required"], false);
        assert_eq!(tools[4]["inputSchema"]["approval_required"], false);
    }
}
