//! Salesforce MCP service implementation.
use crate::client::{to_call_result, SalesforceClient};
use argo_mcp::{
    CallToolParams, CallToolResult, McpContent, McpResult, McpServerService, McpToolSchema,
    ServerInfo,
};
use async_trait::async_trait;

/// Salesforce MCP service.
#[derive(Default)]
pub struct SalesforceService;

#[async_trait]
impl McpServerService for SalesforceService {
    fn server_info(&self) -> ServerInfo {
        ServerInfo {
            name: "argo-mcp-connector-salesforce".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        }
    }

    fn list_tools(&self) -> Vec<McpToolSchema> {
        crate::schemas::all()
    }

    async fn call_tool(&self, params: CallToolParams) -> McpResult<CallToolResult> {
        let client = match SalesforceClient::from_env().await {
            Ok(c) => c,
            Err(text) => {
                return Ok(CallToolResult {
                    content: vec![McpContent::Text { text }],
                    is_error: true,
                });
            }
        };
        let out = match params.name.as_str() {
            "salesforce.query" => client.query(&params.arguments).await,
            "salesforce.get_object" => client.get_object(&params.arguments).await,
            "salesforce.create_object" => client.create_object(&params.arguments).await,
            "salesforce.update_object" => client.update_object(&params.arguments).await,
            "salesforce.list_accounts" => client.list_accounts(&params.arguments).await,
            "salesforce.list_opportunities" => client.list_opportunities(&params.arguments).await,
            "salesforce.add_note" => client.add_note(&params.arguments).await,
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
    async fn list_advertises_seven_salesforce_tools() {
        let tools = SalesforceService.list_tools();
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "salesforce.query",
                "salesforce.get_object",
                "salesforce.create_object",
                "salesforce.update_object",
                "salesforce.list_accounts",
                "salesforce.list_opportunities",
                "salesforce.add_note"
            ]
        );
    }

    #[tokio::test]
    async fn missing_oauth_env_returns_tool_error() {
        std::env::remove_var(crate::auth::SF_CLIENT_ID_ENV);
        let result = SalesforceService
            .call_tool(CallToolParams {
                name: "salesforce.query".into(),
                arguments: serde_json::json!({"soql": "SELECT Id FROM Account"}),
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
        argo_mcp::run_loop(&SalesforceService, input, &mut out)
            .await
            .unwrap();
        let line = String::from_utf8(out).unwrap();
        let value: serde_json::Value = serde_json::from_str(line.trim()).unwrap();
        let tools = value["result"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 7);
        assert_eq!(tools[2]["inputSchema"]["approval_required"], true);
    }
}
