//! Stdio entrypoint for the Google Calendar MCP connector.
use argo_mcp::McpResult;
use argo_mcp_connector_gcal::GcalMcpService;

#[tokio::main]
async fn main() -> McpResult<()> {
    let service = GcalMcpService::from_env().await?;
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    argo_mcp::run_loop(&service, stdin.lock(), stdout.lock()).await
}
