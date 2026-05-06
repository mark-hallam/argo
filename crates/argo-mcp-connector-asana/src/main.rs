//! `argo-mcp-connector-asana` - Asana MCP server.
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

use argo_mcp::run_loop;
use argo_mcp_connector_asana::AsanaService;
use argo_types::ArgoResult;
use std::io::{self, BufReader};

#[tokio::main]
async fn main() {
    let exit = match run().await {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("argo-mcp-connector-asana: {e}");
            1
        }
    };
    std::process::exit(exit);
}

async fn run() -> ArgoResult<()> {
    let svc = AsanaService;
    let stdin = io::stdin();
    let stdout = io::stdout();
    run_loop(&svc, BufReader::new(stdin.lock()), stdout.lock())
        .await
        .map_err(|e| argo_types::ArgoError::Provider(format!("asana mcp: {e}")))?;
    Ok(())
}
