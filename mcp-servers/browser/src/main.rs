//! `argo-mcp-browser` — browser MCP server (§28.AA).
//!
//! First cut ships the **schema + dispatcher** for the 12 tools
//! the spec calls out (Hermes parity). Chromiumoxide bindings
//! (the actual headless-Chrome driver) land per §12.2.1 — they're
//! a heavy dep we don't want to pull in until the wire surface
//! has settled.
//!
//! Today the tool calls return a clear "backend not wired"
//! content block + `isError = true` so callers can introspect
//! the schemas and handle missing browser rigs gracefully.
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

mod service;

use argo_mcp::run_loop;
use argo_types::ArgoResult;
use std::io::{self, BufReader};

#[tokio::main]
async fn main() {
    let exit = match run().await {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("argo-mcp-browser: {e}");
            1
        }
    };
    std::process::exit(exit);
}

async fn run() -> ArgoResult<()> {
    let svc = service::BrowserService;
    let stdin = io::stdin();
    let stdout = io::stdout();
    run_loop(&svc, BufReader::new(stdin.lock()), stdout.lock())
        .await
        .map_err(|e| argo_types::ArgoError::Provider(format!("browser mcp: {e}")))?;
    Ok(())
}
