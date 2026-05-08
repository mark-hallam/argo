//! Typeform MCP service implementation.

use crate::client::{to_call_result, TypeformClient};
use argo_mcp::{
    CallToolParams, CallToolResult, McpContent, McpResult, McpServerService, McpToolSchema,
    ServerInfo,
};
use async_trait::async_trait;

/// Typeform MCP service.
#[derive(Default)]
pub struct TypeformService;

#[async_trait]
impl McpServerService for TypeformService {
    fn server_info(&self) -> ServerInfo {
        ServerInfo {
            name: "argo-mcp-connector-typeform".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        }
    }

    fn list_tools(&self) -> Vec<McpToolSchema> {
        crate::schemas::all()
    }

    async fn call_tool(&self, params: CallToolParams) -> McpResult<CallToolResult> {
        let client = match TypeformClient::from_env() {
            Ok(c) => c,
            Err(text) => {
                return Ok(CallToolResult {
                    content: vec![McpContent::Text { text }],
                    is_error: true,
                });
            }
        };
        let out = match params.name.as_str() {
            "typeform.list_forms" => client.list_forms(&params.arguments).await,
            "typeform.get_form" => client.get_form(&params.arguments).await,
            "typeform.list_responses" => client.list_responses(&params.arguments).await,
            "typeform.get_response" => client.get_response(&params.arguments).await,
            "typeform.create_form" => client.create_form(&params.arguments).await,
            "typeform.update_form" => client.update_form(&params.arguments).await,
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
    async fn list_advertises_six_typeform_tools() {
        let tools = TypeformService.list_tools();
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "typeform.list_forms",
                "typeform.get_form",
                "typeform.list_responses",
                "typeform.get_response",
                "typeform.create_form",
                "typeform.update_form"
            ]
        );
    }

    #[tokio::test]
    async fn stdio_wire_lists_approval_gates() {
        let input = Cursor::new(
            br#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}
"#
            .to_vec(),
        );
        let mut out = Vec::new();
        argo_mcp::run_loop(&TypeformService, input, &mut out)
            .await
            .unwrap();
        let line = String::from_utf8(out).unwrap();
        let value: serde_json::Value = serde_json::from_str(line.trim()).unwrap();
        let tools = value["result"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 6);
        assert_eq!(tools[3]["inputSchema"]["approval_required"], false);
        assert_eq!(tools[4]["inputSchema"]["approval_required"], true);
        assert_eq!(tools[5]["inputSchema"]["approval_required"], true);
    }
}
