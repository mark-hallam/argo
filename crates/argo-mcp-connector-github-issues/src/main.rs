//! `argo-mcp-connector-github-issues` - GitHub Issues MCP server.
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

use argo_mcp::run_loop;
use argo_mcp_connector_github_issues::GithubIssuesService;
use argo_types::ArgoResult;
use std::io::{self, BufReader};

#[tokio::main]
async fn main() {
    let exit = match run().await {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("argo-mcp-connector-github-issues: {e}");
            1
        }
    };
    std::process::exit(exit);
}

async fn run() -> ArgoResult<()> {
    let svc = GithubIssuesService;
    let stdin = io::stdin();
    let stdout = io::stdout();
    run_loop(&svc, BufReader::new(stdin.lock()), stdout.lock())
        .await
        .map_err(|e| argo_types::ArgoError::Provider(format!("github issues mcp: {e}")))?;
    Ok(())
}
