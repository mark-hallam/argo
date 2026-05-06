use argo_mcp::McpResult;
use argo_mcp_connector_slack::SlackService;

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

async fn run() -> McpResult<()> {
    let service = SlackService::from_env()?;
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    argo_mcp::run_loop(&service, stdin.lock(), stdout.lock()).await
}
