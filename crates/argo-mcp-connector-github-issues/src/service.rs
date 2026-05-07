//! GitHub Issues MCP service implementation.

use crate::client::{to_call_result, GithubIssuesClient};
use argo_mcp::{
    CallToolParams, CallToolResult, McpContent, McpResult, McpServerService, McpToolSchema,
    ServerInfo,
};
use async_trait::async_trait;

/// GitHub Issues MCP service.
#[derive(Default)]
pub struct GithubIssuesService;

#[async_trait]
impl McpServerService for GithubIssuesService {
    fn server_info(&self) -> ServerInfo {
        ServerInfo {
            name: "argo-mcp-connector-github-issues".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        }
    }

    fn list_tools(&self) -> Vec<McpToolSchema> {
        crate::schemas::all()
    }

    async fn call_tool(&self, params: CallToolParams) -> McpResult<CallToolResult> {
        let client = match GithubIssuesClient::from_env() {
            Ok(c) => c,
            Err(text) => {
                return Ok(CallToolResult {
                    content: vec![McpContent::Text { text }],
                    is_error: true,
                });
            }
        };
        let out = match params.name.as_str() {
            "github_issues.list_issues" => client.list_issues(&params.arguments).await,
            "github_issues.get_issue" => client.get_issue(&params.arguments).await,
            "github_issues.create_issue" => client.create_issue(&params.arguments).await,
            "github_issues.update_issue" => client.update_issue(&params.arguments).await,
            "github_issues.add_comment" => client.add_comment(&params.arguments).await,
            "github_issues.close_issue" => client.close_issue(&params.arguments).await,
            "github_issues.label_issue" => client.label_issue(&params.arguments).await,
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
    async fn list_advertises_seven_github_issue_tools() {
        let tools = GithubIssuesService.list_tools();
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "github_issues.list_issues",
                "github_issues.get_issue",
                "github_issues.create_issue",
                "github_issues.update_issue",
                "github_issues.add_comment",
                "github_issues.close_issue",
                "github_issues.label_issue"
            ]
        );
    }

    #[tokio::test]
    async fn missing_auth_env_returns_tool_error() {
        std::env::remove_var(crate::auth::GITHUB_TOKEN_ENV);
        let result = GithubIssuesService
            .call_tool(CallToolParams {
                name: "github_issues.list_issues".into(),
                arguments: serde_json::json!({"owner": "octo", "repo": "repo"}),
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
        argo_mcp::run_loop(&GithubIssuesService, input, &mut out)
            .await
            .unwrap();
        let line = String::from_utf8(out).unwrap();
        let value: serde_json::Value = serde_json::from_str(line.trim()).unwrap();
        let tools = value["result"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 7);
        assert_eq!(tools[2]["inputSchema"]["approval_required"], true);
    }
}
