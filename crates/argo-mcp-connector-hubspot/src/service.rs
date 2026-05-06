//! HubSpot MCP service implementation.
use crate::client::{to_call_result, HubspotClient};
use argo_mcp::{
    CallToolParams, CallToolResult, McpContent, McpResult, McpServerService, McpToolSchema,
    ServerInfo,
};
use async_trait::async_trait;

/// HubSpot MCP service.
#[derive(Default)]
pub struct HubspotService;

#[async_trait]
impl McpServerService for HubspotService {
    fn server_info(&self) -> ServerInfo {
        ServerInfo {
            name: "argo-mcp-connector-hubspot".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        }
    }

    fn list_tools(&self) -> Vec<McpToolSchema> {
        crate::schemas::all()
    }

    async fn call_tool(&self, params: CallToolParams) -> McpResult<CallToolResult> {
        let client = match HubspotClient::from_env() {
            Ok(c) => c,
            Err(text) => {
                return Ok(CallToolResult {
                    content: vec![McpContent::Text { text }],
                    is_error: true,
                });
            }
        };
        let out = match params.name.as_str() {
            "hubspot.search_contacts" => client.search_contacts(&params.arguments).await,
            "hubspot.create_contact" => client.create_contact(&params.arguments).await,
            "hubspot.create_deal" => client.create_deal(&params.arguments).await,
            "hubspot.log_activity" => client.log_activity(&params.arguments).await,
            "hubspot.get_pipeline" => client.get_pipeline(&params.arguments).await,
            other => Err(format!("unknown tool: {other}")),
        };
        Ok(to_call_result(out))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn list_advertises_five_hubspot_tools() {
        let tools = HubspotService.list_tools();
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "hubspot.search_contacts",
                "hubspot.create_contact",
                "hubspot.create_deal",
                "hubspot.log_activity",
                "hubspot.get_pipeline"
            ]
        );
    }

    #[tokio::test]
    async fn missing_token_returns_tool_error() {
        std::env::remove_var("HUBSPOT_PRIVATE_APP_TOKEN");
        let result = HubspotService
            .call_tool(CallToolParams {
                name: "hubspot.search_contacts".into(),
                arguments: serde_json::json!({}),
            })
            .await
            .unwrap();
        assert!(result.is_error);
        match &result.content[0] {
            McpContent::Text { text } => assert!(text.contains("HUBSPOT_PRIVATE_APP_TOKEN")),
            _ => panic!("expected text"),
        }
    }
}
