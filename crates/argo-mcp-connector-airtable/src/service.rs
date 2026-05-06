//! Airtable MCP service implementation.
use crate::client::{to_call_result, AirtableClient};
use argo_mcp::{
    CallToolParams, CallToolResult, McpContent, McpResult, McpServerService, McpToolSchema,
    ServerInfo,
};
use async_trait::async_trait;

/// Airtable MCP service.
#[derive(Default)]
pub struct AirtableService;

#[async_trait]
impl McpServerService for AirtableService {
    fn server_info(&self) -> ServerInfo {
        ServerInfo {
            name: "argo-mcp-connector-airtable".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        }
    }

    fn list_tools(&self) -> Vec<McpToolSchema> {
        crate::schemas::all()
    }

    async fn call_tool(&self, params: CallToolParams) -> McpResult<CallToolResult> {
        let client = match AirtableClient::from_env() {
            Ok(c) => c,
            Err(text) => {
                return Ok(CallToolResult {
                    content: vec![McpContent::Text { text }],
                    is_error: true,
                });
            }
        };
        let out = match params.name.as_str() {
            "airtable.list_bases" => client.list_bases(&params.arguments).await,
            "airtable.list_tables" => client.list_tables(&params.arguments).await,
            "airtable.list_records" => client.list_records(&params.arguments).await,
            "airtable.get_record" => client.get_record(&params.arguments).await,
            "airtable.create_record" => client.create_record(&params.arguments).await,
            "airtable.update_record" => client.update_record(&params.arguments).await,
            "airtable.delete_record" => client.delete_record(&params.arguments).await,
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
    async fn list_advertises_seven_airtable_tools() {
        let tools = AirtableService.list_tools();
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "airtable.list_bases",
                "airtable.list_tables",
                "airtable.list_records",
                "airtable.get_record",
                "airtable.create_record",
                "airtable.update_record",
                "airtable.delete_record"
            ]
        );
    }

    #[tokio::test]
    async fn missing_pat_returns_tool_error() {
        std::env::remove_var(crate::auth::AIRTABLE_PAT_ENV);
        let result = AirtableService
            .call_tool(CallToolParams {
                name: "airtable.list_bases".into(),
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
        argo_mcp::run_loop(&AirtableService, input, &mut out)
            .await
            .unwrap();
        let line = String::from_utf8(out).unwrap();
        let value: serde_json::Value = serde_json::from_str(line.trim()).unwrap();
        let tools = value["result"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 7);
        assert_eq!(tools[4]["inputSchema"]["approval_required"], true);
    }
}
