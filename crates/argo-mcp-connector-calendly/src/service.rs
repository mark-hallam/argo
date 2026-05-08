//! Calendly MCP service implementation.

use crate::client::{to_call_result, CalendlyClient};
use argo_mcp::{
    CallToolParams, CallToolResult, McpContent, McpResult, McpServerService, McpToolSchema,
    ServerInfo,
};
use async_trait::async_trait;

/// Calendly MCP service.
#[derive(Default)]
pub struct CalendlyService;

#[async_trait]
impl McpServerService for CalendlyService {
    fn server_info(&self) -> ServerInfo {
        ServerInfo {
            name: "argo-mcp-connector-calendly".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        }
    }

    fn list_tools(&self) -> Vec<McpToolSchema> {
        crate::schemas::all()
    }

    async fn call_tool(&self, params: CallToolParams) -> McpResult<CallToolResult> {
        let client = match CalendlyClient::from_env() {
            Ok(c) => c,
            Err(text) => {
                return Ok(CallToolResult {
                    content: vec![McpContent::Text { text }],
                    is_error: true,
                });
            }
        };
        let out = match params.name.as_str() {
            "calendly.list_event_types" => client.list_event_types(&params.arguments).await,
            "calendly.list_scheduled_events" => {
                client.list_scheduled_events(&params.arguments).await
            }
            "calendly.get_event" => client.get_event(&params.arguments).await,
            "calendly.cancel_event" => client.cancel_event(&params.arguments).await,
            "calendly.get_invitee" => client.get_invitee(&params.arguments).await,
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
    async fn list_advertises_five_calendly_tools() {
        let tools = CalendlyService.list_tools();
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "calendly.list_event_types",
                "calendly.list_scheduled_events",
                "calendly.get_event",
                "calendly.cancel_event",
                "calendly.get_invitee",
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
        argo_mcp::run_loop(&CalendlyService, input, &mut out)
            .await
            .unwrap();
        let line = String::from_utf8(out).unwrap();
        let value: serde_json::Value = serde_json::from_str(line.trim()).unwrap();
        let tools = value["result"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 5);
        assert_eq!(tools[2]["inputSchema"]["approval_required"], false);
        assert_eq!(tools[3]["inputSchema"]["approval_required"], true);
    }
}
