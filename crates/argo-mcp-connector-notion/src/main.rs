//! Stdio entrypoint for the Notion MCP connector.
use argo_mcp::McpResult;
use argo_mcp_connector_notion::NotionMcpService;

#[tokio::main]
async fn main() -> McpResult<()> {
    let service = NotionMcpService::from_env()?;
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    argo_mcp::run_loop(&service, stdin.lock(), stdout.lock()).await
}
