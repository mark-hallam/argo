//! `argo-mcp-image-gen` — stdio MCP server binary.
//!
//! Reads JSON-RPC requests from stdin, writes responses to stdout.
//! Config path is `--config <path>` (defaults to `./argo.toml`);
//! `--require-confirm` gates calls with estimate ≥ 20¢.

use argo_mcp::run_loop;
use argo_mcp_image_gen::{
    AuditSink, BackendKind, ImageBackend, ImageGenConfig, ImageGenMcpServer,
    OpenAiCompatibleBackend, StubBackend,
};
use std::io::{self, BufReader};
use std::path::PathBuf;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let mut config_path = PathBuf::from("argo.toml");
    let mut require_confirm = false;
    while let Some(a) = args.next() {
        match a.as_str() {
            "--config" => {
                config_path = PathBuf::from(args.next().ok_or("--config needs a path")?);
            }
            "--require-confirm" => require_confirm = true,
            other => return Err(format!("unknown flag: {other}")),
        }
    }
    let text = std::fs::read_to_string(&config_path)
        .map_err(|e| format!("read {}: {e}", config_path.display()))?;
    let cfg = ImageGenConfig::from_toml_str(&text)?;
    let backend: Arc<dyn ImageBackend> = match cfg.backend {
        BackendKind::Openai | BackendKind::StableDiffusion => {
            let key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
            Arc::new(OpenAiCompatibleBackend::new(cfg.api_base.clone(), key)?)
        }
        BackendKind::Stub => Arc::new(StubBackend),
    };
    let audit_path = std::env::var("ARGO_HOME")
        .map(|h| PathBuf::from(h).join("audit/image_gen.ndjson"))
        .unwrap_or_else(|_| {
            dirs_home().join(".argo").join("audit").join("image_gen.ndjson")
        });
    let server = ImageGenMcpServer::new(
        backend,
        cfg.model_default,
        cfg.size_default,
        require_confirm,
        AuditSink::File(audit_path),
    );
    let stdin = io::stdin();
    let stdout = io::stdout();
    run_loop(&server, BufReader::new(stdin.lock()), stdout.lock())
        .await
        .map_err(|e| format!("mcp loop: {e}"))?;
    Ok(())
}

fn dirs_home() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}
