//! Discord MCP service implementation.

use crate::client::{to_call_result, DiscordClient};
use argo_mcp::{
    CallToolParams, CallToolResult, McpContent, McpResult, McpServerService, McpToolSchema,
    ServerInfo,
};
use async_trait::async_trait;

/// Discord MCP service.
#[derive(Default)]
pub struct DiscordService;

#[async_trait]
impl McpServerService for DiscordService {
    fn server_info(&self) -> ServerInfo {
        ServerInfo {
            name: "argo-mcp-connector-discord".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        }
    }

    fn list_tools(&self) -> Vec<McpToolSchema> {
        crate::schemas::all()
    }

    async fn call_tool(&self, params: CallToolParams) -> McpResult<CallToolResult> {
        let client = match DiscordClient::from_env() {
            Ok(c) => c,
            Err(text) => {
                return Ok(CallToolResult {
                    content: vec![McpContent::Text { text }],
                    is_error: true,
                });
            }
        };
        let out = match params.name.as_str() {
            "discord.list_guilds" => client.list_guilds(&params.arguments).await,
            "discord.list_channels" => client.list_channels(&params.arguments).await,
            "discord.list_messages" => client.list_messages(&params.arguments).await,
            "discord.get_message" => client.get_message(&params.arguments).await,
            "discord.send_message" => client.send_message(&params.arguments).await,
            "discord.add_reaction" => client.add_reaction(&params.arguments).await,
            "discord.list_members" => client.list_members(&params.arguments).await,
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
    async fn list_advertises_seven_discord_tools() {
        let tools = DiscordService.list_tools();
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "discord.list_guilds",
                "discord.list_channels",
                "discord.list_messages",
                "discord.get_message",
                "discord.send_message",
                "discord.add_reaction",
                "discord.list_members"
            ]
        );
    }

    #[tokio::test]
    async fn missing_auth_env_returns_tool_error() {
        std::env::remove_var(crate::auth::DISCORD_BOT_TOKEN_ENV);
        let result = DiscordService
            .call_tool(CallToolParams {
                name: "discord.list_guilds".into(),
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
        argo_mcp::run_loop(&DiscordService, input, &mut out)
            .await
            .unwrap();
        let line = String::from_utf8(out).unwrap();
        let value: serde_json::Value = serde_json::from_str(line.trim()).unwrap();
        let tools = value["result"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 7);
        assert_eq!(tools[4]["inputSchema"]["approval_required"], true);
    }
}
