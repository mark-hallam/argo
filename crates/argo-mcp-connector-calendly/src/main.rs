//! `argo-mcp-connector-calendly` - Calendly MCP server.
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

use argo_mcp::run_loop;
use argo_mcp_connector_calendly::CalendlyService;
use argo_types::ArgoResult;
use std::io::{self, BufReader};

#[tokio::main]
async fn main() {
    let exit = match run().await {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("argo-mcp-connector-calendly: {e}");
            1
        }
    };
    std::process::exit(exit);
}

async fn run() -> ArgoResult<()> {
    let svc = CalendlyService;
    let stdin = io::stdin();
    let stdout = io::stdout();
    run_loop(&svc, BufReader::new(stdin.lock()), stdout.lock())
        .await
        .map_err(|e| argo_types::ArgoError::Provider(format!("calendly mcp: {e}")))?;
    Ok(())
}
