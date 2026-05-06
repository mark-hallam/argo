//! Gmail MCP service implementation.
use crate::client::{to_call_result, GmailClient};
use argo_mcp::{
    CallToolParams, CallToolResult, McpContent, McpResult, McpServerService, McpToolSchema,
    ServerInfo,
};
use async_trait::async_trait;

/// Gmail MCP service.
#[derive(Default)]
pub struct GmailService {
    access_token_override: Option<String>,
}

impl GmailService {
    /// Build a service with a fixed access token for gated live tests.
    #[must_use]
    pub fn with_access_token_for_tests(token: String) -> Self {
        Self {
            access_token_override: Some(token),
        }
    }

    async fn client(&self) -> Result<GmailClient, String> {
        match &self.access_token_override {
            Some(token) => GmailClient::with_access_token(token.clone()),
            None => GmailClient::from_env().await,
        }
    }
}

#[async_trait]
impl McpServerService for GmailService {
    fn server_info(&self) -> ServerInfo {
        ServerInfo {
            name: "argo-mcp-connector-gmail".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        }
    }

    fn list_tools(&self) -> Vec<McpToolSchema> {
        crate::schemas::all()
    }

    async fn call_tool(&self, params: CallToolParams) -> McpResult<CallToolResult> {
        let mut client = match self.client().await {
            Ok(c) => c,
            Err(text) => {
                return Ok(CallToolResult {
                    content: vec![McpContent::Text { text }],
                    is_error: true,
                });
            }
        };
        let out = match params.name.as_str() {
            "gmail.search" => client.search(&params.arguments).await,
            "gmail.read" => client.read(&params.arguments).await,
            "gmail.draft" => client.draft(&params.arguments).await,
            "gmail.send" => client.send(&params.arguments).await,
            "gmail.label" => client.label(&params.arguments).await,
            other => Err(format!("unknown tool: {other}")),
        };
        Ok(to_call_result(out))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn list_advertises_five_gmail_tools() {
        let tools = GmailService::default().list_tools();
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "gmail.search",
                "gmail.read",
                "gmail.draft",
                "gmail.send",
                "gmail.label"
            ]
        );
    }

    #[tokio::test]
    async fn missing_oauth_env_returns_tool_error() {
        std::env::remove_var("ARGO_GMAIL_CLIENT_ID");
        std::env::remove_var("ARGO_GMAIL_CLIENT_SECRET");
        std::env::remove_var("ARGO_GMAIL_REFRESH_TOKEN");
        let result = GmailService::default()
            .call_tool(CallToolParams {
                name: "gmail.search".into(),
                arguments: serde_json::json!({}),
            })
            .await
            .unwrap();
        assert!(result.is_error);
    }
}
