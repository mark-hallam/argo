//! Stripe MCP service implementation.
use crate::client::{to_call_result, StripeClient};
use argo_mcp::{
    CallToolParams, CallToolResult, McpContent, McpResult, McpServerService, McpToolSchema,
    ServerInfo,
};
use async_trait::async_trait;

/// Stripe MCP service.
#[derive(Default)]
pub struct StripeService;

#[async_trait]
impl McpServerService for StripeService {
    fn server_info(&self) -> ServerInfo {
        ServerInfo {
            name: "argo-mcp-connector-stripe".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        }
    }

    fn list_tools(&self) -> Vec<McpToolSchema> {
        crate::schemas::all()
    }

    async fn call_tool(&self, params: CallToolParams) -> McpResult<CallToolResult> {
        let client = match StripeClient::from_env() {
            Ok(c) => c,
            Err(text) => {
                return Ok(CallToolResult {
                    content: vec![McpContent::Text { text }],
                    is_error: true,
                });
            }
        };
        let out = match params.name.as_str() {
            "stripe.list_customers" => client.list_customers(&params.arguments).await,
            "stripe.get_customer" => client.get_customer(&params.arguments).await,
            "stripe.create_charge" => client.create_charge(&params.arguments).await,
            "stripe.list_charges" => client.list_charges(&params.arguments).await,
            "stripe.refund_charge" => client.refund_charge(&params.arguments).await,
            "stripe.create_invoice" => client.create_invoice(&params.arguments).await,
            "stripe.list_invoices" => client.list_invoices(&params.arguments).await,
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
    async fn list_advertises_seven_stripe_tools() {
        let tools = StripeService.list_tools();
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "stripe.list_customers",
                "stripe.get_customer",
                "stripe.create_charge",
                "stripe.list_charges",
                "stripe.refund_charge",
                "stripe.create_invoice",
                "stripe.list_invoices"
            ]
        );
    }

    #[tokio::test]
    async fn missing_api_key_returns_tool_error() {
        std::env::remove_var(crate::auth::STRIPE_API_KEY_ENV);
        let result = StripeService
            .call_tool(CallToolParams {
                name: "stripe.list_customers".into(),
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
        argo_mcp::run_loop(&StripeService, input, &mut out)
            .await
            .unwrap();
        let line = String::from_utf8(out).unwrap();
        let value: serde_json::Value = serde_json::from_str(line.trim()).unwrap();
        let tools = value["result"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 7);
        assert_eq!(tools[2]["inputSchema"]["approval_required"], true);
    }
}
