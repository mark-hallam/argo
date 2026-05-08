//! Mailchimp MCP service implementation.

use crate::client::{to_call_result, MailchimpClient};
use argo_mcp::{
    CallToolParams, CallToolResult, McpContent, McpResult, McpServerService, McpToolSchema,
    ServerInfo,
};
use async_trait::async_trait;

/// Mailchimp MCP service.
#[derive(Default)]
pub struct MailchimpService;

#[async_trait]
impl McpServerService for MailchimpService {
    fn server_info(&self) -> ServerInfo {
        ServerInfo {
            name: "argo-mcp-connector-mailchimp".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        }
    }

    fn list_tools(&self) -> Vec<McpToolSchema> {
        crate::schemas::all()
    }

    async fn call_tool(&self, params: CallToolParams) -> McpResult<CallToolResult> {
        let client = match MailchimpClient::from_env() {
            Ok(c) => c,
            Err(text) => {
                return Ok(CallToolResult {
                    content: vec![McpContent::Text { text }],
                    is_error: true,
                });
            }
        };
        let out = match params.name.as_str() {
            "mailchimp.list_audiences" => client.list_audiences(&params.arguments).await,
            "mailchimp.list_campaigns" => client.list_campaigns(&params.arguments).await,
            "mailchimp.get_campaign" => client.get_campaign(&params.arguments).await,
            "mailchimp.send_campaign" => client.send_campaign(&params.arguments).await,
            "mailchimp.list_subscribers" => client.list_subscribers(&params.arguments).await,
            "mailchimp.add_subscriber" => client.add_subscriber(&params.arguments).await,
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
    async fn list_advertises_six_mailchimp_tools() {
        let tools = MailchimpService.list_tools();
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "mailchimp.list_audiences",
                "mailchimp.list_campaigns",
                "mailchimp.get_campaign",
                "mailchimp.send_campaign",
                "mailchimp.list_subscribers",
                "mailchimp.add_subscriber",
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
        argo_mcp::run_loop(&MailchimpService, input, &mut out)
            .await
            .unwrap();
        let line = String::from_utf8(out).unwrap();
        let value: serde_json::Value = serde_json::from_str(line.trim()).unwrap();
        let tools = value["result"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 6);
        assert_eq!(tools[2]["inputSchema"]["approval_required"], false);
        assert_eq!(tools[3]["inputSchema"]["approval_required"], true);
    }
}
