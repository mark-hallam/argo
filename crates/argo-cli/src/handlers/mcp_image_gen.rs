//! `argo mcp image-gen` — boot the image-generation MCP server
//! over stdio. Thin shim around [`argo_mcp_image_gen`] that wires
//! the `[mcp.image_gen]` config into the server constructor and
//! runs the JSON-RPC loop against stdin/stdout.

use argo_mcp::run_loop;
use argo_mcp_image_gen::{
    AuditSink, BackendKind, ImageBackend, ImageGenConfig, ImageGenMcpServer,
    OpenAiCompatibleBackend, StubBackend,
};
use argo_types::{ArgoError, ArgoResult};
use std::io::{self, BufReader};
use std::path::PathBuf;
use std::sync::Arc;

/// Parsed CLI args.
pub struct Args {
    /// `--config <path>`.
    pub config: String,
    /// `--require-confirm`.
    pub require_confirm: bool,
}

/// Entry point for `argo mcp image-gen`.
pub async fn run(args: Args) -> ArgoResult<()> {
    let path = PathBuf::from(&args.config);
    let text = std::fs::read_to_string(&path)
        .map_err(|e| ArgoError::Provider(format!("read {}: {e}", path.display())))?;
    let cfg = ImageGenConfig::from_toml_str(&text).map_err(ArgoError::Provider)?;
    let backend: Arc<dyn ImageBackend> = match cfg.backend {
        BackendKind::Openai | BackendKind::StableDiffusion => {
            let key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
            Arc::new(
                OpenAiCompatibleBackend::new(cfg.api_base.clone(), key)
                    .map_err(ArgoError::Provider)?,
            )
        }
        BackendKind::Stub => Arc::new(StubBackend),
    };
    let audit_path = std::env::var_os("HOME")
        .map(|h| PathBuf::from(h).join(".argo/audit/image_gen.ndjson"))
        .unwrap_or_else(|| PathBuf::from("./.argo/audit/image_gen.ndjson"));
    let server = ImageGenMcpServer::new(
        backend,
        cfg.model_default,
        cfg.size_default,
        args.require_confirm,
        AuditSink::File(audit_path),
    );
    let stdin = io::stdin();
    let stdout = io::stdout();
    run_loop(&server, BufReader::new(stdin.lock()), stdout.lock())
        .await
        .map_err(|e| ArgoError::Provider(format!("mcp image-gen loop: {e}")))?;
    Ok(())
}
