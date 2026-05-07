//! Stripe events MCP service implementation.

use crate::client::{to_call_result, StripeEventsClient};
use argo_mcp::{
    CallToolParams, CallToolResult, McpContent, McpResult, McpServerService, McpToolSchema,
    ServerInfo,
};
use async_trait::async_trait;

/// Stripe events MCP service.
#[derive(Default)]
pub struct StripeEventsService;

#[async_trait]
impl McpServerService for StripeEventsService {
    fn server_info(&self) -> ServerInfo {
        ServerInfo {
            name: "argo-mcp-connector-stripe-events".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        }
    }

    fn list_tools(&self) -> Vec<McpToolSchema> {
        crate::schemas::all()
    }

    async fn call_tool(&self, params: CallToolParams) -> McpResult<CallToolResult> {
        let client = match StripeEventsClient::from_env() {
            Ok(c) => c,
            Err(text) => {
                return Ok(CallToolResult {
                    content: vec![McpContent::Text { text }],
                    is_error: true,
                });
            }
        };
        let out = match params.name.as_str() {
            "stripe_events.list_events" => client.list_events(&params.arguments).await,
            "stripe_events.get_event" => client.get_event(&params.arguments).await,
            "stripe_events.list_charges" => client.list_charges(&params.arguments).await,
            "stripe_events.list_customers" => client.list_customers(&params.arguments).await,
            "stripe_events.get_customer" => client.get_customer(&params.arguments).await,
            "stripe_events.list_subscriptions" => {
                client.list_subscriptions(&params.arguments).await
            }
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
    async fn list_advertises_six_stripe_events_tools() {
        let tools = StripeEventsService.list_tools();
        let names: Vec<&str> = tools.iter().map(|tool| tool.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "stripe_events.list_events",
                "stripe_events.get_event",
                "stripe_events.list_charges",
                "stripe_events.list_customers",
                "stripe_events.get_customer",
                "stripe_events.list_subscriptions",
            ]
        );
    }

    #[tokio::test]
    async fn missing_api_key_returns_tool_error() {
        std::env::remove_var(crate::auth::STRIPE_API_KEY_ENV);
        let result = StripeEventsService
            .call_tool(CallToolParams {
                name: "stripe_events.list_events".into(),
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
        argo_mcp::run_loop(&StripeEventsService, input, &mut out)
            .await
            .unwrap();
        let line = String::from_utf8(out).unwrap();
        let value: serde_json::Value = serde_json::from_str(line.trim()).unwrap();
        let tools = value["result"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 6);
        for tool in tools {
            assert_eq!(tool["inputSchema"]["approval_required"], false);
        }
    }
}
