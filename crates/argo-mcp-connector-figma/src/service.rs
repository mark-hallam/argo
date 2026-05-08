//! Figma MCP service implementation.

use crate::client::{to_call_result, FigmaClient};
use argo_mcp::{
    CallToolParams, CallToolResult, McpContent, McpResult, McpServerService, McpToolSchema,
    ServerInfo,
};
use async_trait::async_trait;

/// Figma MCP service.
#[derive(Default)]
pub struct FigmaService;

#[async_trait]
impl McpServerService for FigmaService {
    fn server_info(&self) -> ServerInfo {
        ServerInfo {
            name: "argo-mcp-connector-figma".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        }
    }

    fn list_tools(&self) -> Vec<McpToolSchema> {
        crate::schemas::all()
    }

    async fn call_tool(&self, params: CallToolParams) -> McpResult<CallToolResult> {
        let client = match FigmaClient::from_env() {
            Ok(c) => c,
            Err(text) => {
                return Ok(CallToolResult {
                    content: vec![McpContent::Text { text }],
                    is_error: true,
                });
            }
        };
        let out = match params.name.as_str() {
            "figma.list_files" => client.list_files(&params.arguments).await,
            "figma.get_file" => client.get_file(&params.arguments).await,
            "figma.get_file_components" => client.get_file_components(&params.arguments).await,
            "figma.list_comments" => client.list_comments(&params.arguments).await,
            "figma.add_comment" => client.add_comment(&params.arguments).await,
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
    async fn list_advertises_five_figma_tools() {
        let tools = FigmaService.list_tools();
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "figma.list_files",
                "figma.get_file",
                "figma.get_file_components",
                "figma.list_comments",
                "figma.add_comment",
            ]
        );
    }

    #[tokio::test]
    async fn stdio_wire_lists_tools() {
        let input = Cursor::new(
            br#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}
"#
            .to_vec(),
        );
        let mut out = Vec::new();
        argo_mcp::run_loop(&FigmaService, input, &mut out)
            .await
            .unwrap();
        let line = String::from_utf8(out).unwrap();
        let value: serde_json::Value = serde_json::from_str(line.trim()).unwrap();
        let tools = value["result"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 5);
        assert_eq!(tools[3]["inputSchema"]["approval_required"], false);
        assert_eq!(tools[4]["inputSchema"]["approval_required"], true);
    }
}
