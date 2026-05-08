//! SendGrid MCP service implementation.

use crate::client::{to_call_result, SendgridClient};
use argo_mcp::{
    CallToolParams, CallToolResult, McpContent, McpResult, McpServerService, McpToolSchema,
    ServerInfo,
};
use async_trait::async_trait;

/// SendGrid MCP service.
#[derive(Default)]
pub struct SendgridService;

#[async_trait]
impl McpServerService for SendgridService {
    fn server_info(&self) -> ServerInfo {
        ServerInfo {
            name: "argo-mcp-connector-sendgrid".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        }
    }

    fn list_tools(&self) -> Vec<McpToolSchema> {
        crate::schemas::all()
    }

    async fn call_tool(&self, params: CallToolParams) -> McpResult<CallToolResult> {
        let client = match SendgridClient::from_env() {
            Ok(c) => c,
            Err(text) => {
                return Ok(CallToolResult {
                    content: vec![McpContent::Text { text }],
                    is_error: true,
                });
            }
        };
        let out = match params.name.as_str() {
            "sendgrid.list_templates" => client.list_templates(&params.arguments).await,
            "sendgrid.send_email" => client.send_email(&params.arguments).await,
            "sendgrid.list_contacts" => client.list_contacts(&params.arguments).await,
            "sendgrid.add_contact" => client.add_contact(&params.arguments).await,
            "sendgrid.list_lists" => client.list_lists(&params.arguments).await,
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
    async fn list_advertises_five_sendgrid_tools() {
        let tools = SendgridService.list_tools();
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "sendgrid.list_templates",
                "sendgrid.send_email",
                "sendgrid.list_contacts",
                "sendgrid.add_contact",
                "sendgrid.list_lists",
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
        argo_mcp::run_loop(&SendgridService, input, &mut out)
            .await
            .unwrap();
        let line = String::from_utf8(out).unwrap();
        let value: serde_json::Value = serde_json::from_str(line.trim()).unwrap();
        let tools = value["result"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 5);
        assert_eq!(tools[0]["inputSchema"]["approval_required"], false);
        assert_eq!(tools[1]["inputSchema"]["approval_required"], true);
    }
}
