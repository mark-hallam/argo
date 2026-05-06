//! Jira MCP service implementation.

use crate::client::{to_call_result, JiraClient};
use argo_mcp::{
    CallToolParams, CallToolResult, McpContent, McpResult, McpServerService, McpToolSchema,
    ServerInfo,
};
use async_trait::async_trait;

/// MCP service exposing Jira Cloud tools.
#[derive(Default)]
pub struct JiraService {
    client_override: Option<JiraClient>,
}

impl JiraService {
    /// Build a service with a fixed client for tests.
    ///
    /// ```
    /// # fn doc() -> Result<(), String> {
    /// use argo_mcp_connector_jira::{JiraClient, JiraConfig, JiraService};
    /// let cfg = JiraConfig::new("op@example.com", "token", "https://example.atlassian.net")?;
    /// let _svc = JiraService::with_client_for_tests(JiraClient::new(cfg)?);
    /// # Ok(()) }
    /// ```
    #[must_use]
    pub fn with_client_for_tests(client: JiraClient) -> Self {
        Self {
            client_override: Some(client),
        }
    }

    fn client(&self) -> Result<JiraClient, String> {
        match &self.client_override {
            Some(client) => Ok(client.clone()),
            None => JiraClient::from_env(),
        }
    }
}

#[async_trait]
impl McpServerService for JiraService {
    fn server_info(&self) -> ServerInfo {
        ServerInfo {
            name: "argo-mcp-connector-jira".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        }
    }

    fn list_tools(&self) -> Vec<McpToolSchema> {
        crate::schemas::all()
    }

    async fn call_tool(&self, params: CallToolParams) -> McpResult<CallToolResult> {
        let client = match self.client() {
            Ok(client) => client,
            Err(text) => {
                return Ok(CallToolResult {
                    content: vec![McpContent::Text { text }],
                    is_error: true,
                });
            }
        };
        let out = match params.name.as_str() {
            "jira.list_issues" => client.list_issues(&params.arguments).await,
            "jira.get_issue" => client.get_issue(&params.arguments).await,
            "jira.create_issue" => client.create_issue(&params.arguments).await,
            "jira.update_issue" => client.update_issue(&params.arguments).await,
            "jira.comment_issue" => client.comment_issue(&params.arguments).await,
            "jira.transition_issue" => client.transition_issue(&params.arguments).await,
            "jira.list_projects" => client.list_projects(&params.arguments).await,
            other => Err(format!("unknown jira tool: {other}")),
        };
        Ok(to_call_result(out))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use argo_mcp::{handle_request, JsonRpcRequest};

    #[tokio::test]
    async fn list_advertises_seven_jira_tools() {
        let tools = JiraService::default().list_tools();
        let names: Vec<&str> = tools.iter().map(|tool| tool.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "jira.list_issues",
                "jira.get_issue",
                "jira.create_issue",
                "jira.update_issue",
                "jira.comment_issue",
                "jira.transition_issue",
                "jira.list_projects",
            ]
        );
    }

    #[tokio::test]
    async fn missing_env_returns_tool_error() {
        std::env::remove_var("JIRA_EMAIL");
        std::env::remove_var("JIRA_API_TOKEN");
        std::env::remove_var("JIRA_BASE_URL");
        let result = JiraService::default()
            .call_tool(CallToolParams {
                name: "jira.list_projects".into(),
                arguments: serde_json::json!({}),
            })
            .await
            .expect("tool result");
        assert!(result.is_error);
        match &result.content[0] {
            McpContent::Text { text } => assert!(text.contains("JIRA_EMAIL")),
            _ => panic!("expected text"),
        }
    }

    #[tokio::test]
    async fn wire_tools_list_returns_jira_schemas() {
        let response = handle_request(
            &JiraService::default(),
            JsonRpcRequest::new(1, "tools/list", None),
        )
        .await;
        assert!(response.error.is_none());
        let result = response.result.expect("result");
        let tools = result["tools"].as_array().expect("tools");
        assert_eq!(tools.len(), 7);
        assert_eq!(tools[0]["name"], "jira.list_issues");
    }
}
