//! PagerDuty MCP service implementation.
use crate::client::{to_call_result, PagerDutyClient};
use argo_mcp::{
    CallToolParams, CallToolResult, McpContent, McpResult, McpServerService, McpToolSchema,
    ServerInfo,
};
use async_trait::async_trait;

/// PagerDuty MCP service.
#[derive(Default)]
pub struct PagerDutyService;

#[async_trait]
impl McpServerService for PagerDutyService {
    fn server_info(&self) -> ServerInfo {
        ServerInfo {
            name: "argo-mcp-connector-pagerduty".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        }
    }

    fn list_tools(&self) -> Vec<McpToolSchema> {
        crate::schemas::all()
    }

    async fn call_tool(&self, params: CallToolParams) -> McpResult<CallToolResult> {
        let client = match PagerDutyClient::from_env() {
            Ok(c) => c,
            Err(text) => {
                return Ok(CallToolResult {
                    content: vec![McpContent::Text { text }],
                    is_error: true,
                });
            }
        };
        let out = match params.name.as_str() {
            "pagerduty.list_incidents" => client.list_incidents(&params.arguments).await,
            "pagerduty.get_incident" => client.get_incident(&params.arguments).await,
            "pagerduty.create_incident" => client.create_incident(&params.arguments).await,
            "pagerduty.acknowledge_incident" => {
                client.acknowledge_incident(&params.arguments).await
            }
            "pagerduty.resolve_incident" => client.resolve_incident(&params.arguments).await,
            "pagerduty.list_services" => client.list_services(&params.arguments).await,
            "pagerduty.list_oncalls" => client.list_oncalls(&params.arguments).await,
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
    async fn list_advertises_seven_pagerduty_tools() {
        let tools = PagerDutyService.list_tools();
        let names: Vec<&str> = tools.iter().map(|tool| tool.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "pagerduty.list_incidents",
                "pagerduty.get_incident",
                "pagerduty.create_incident",
                "pagerduty.acknowledge_incident",
                "pagerduty.resolve_incident",
                "pagerduty.list_services",
                "pagerduty.list_oncalls",
            ]
        );
    }

    #[tokio::test]
    async fn missing_api_key_returns_tool_error() {
        std::env::remove_var(crate::auth::PAGERDUTY_API_KEY_ENV);
        let result = PagerDutyService
            .call_tool(CallToolParams {
                name: "pagerduty.list_incidents".into(),
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
        argo_mcp::run_loop(&PagerDutyService, input, &mut out)
            .await
            .expect("run loop");
        let line = String::from_utf8(out).expect("utf8");
        let value: serde_json::Value = serde_json::from_str(line.trim()).expect("json");
        let tools = value["result"]["tools"].as_array().expect("tools");
        assert_eq!(tools.len(), 7);
        assert_eq!(tools[2]["inputSchema"]["approval_required"], true);
    }
}
