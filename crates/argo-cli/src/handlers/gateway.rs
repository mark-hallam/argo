//! `argo gateway` — start / stop / status. \`status\` and \`start\`
//! are wired; \`stop\` still needs the PID-file plumbing the §21.2
//! daemon lifecycle layer will land.
use crate::cli::GatewayCmd;
use argo_config::ArgoConfig;
use argo_types::{ArgoError, ArgoResult};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

pub async fn run(cmd: GatewayCmd) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let cfg = ArgoConfig::load(&cwd.join("argo.toml")).unwrap_or_default();
    match cmd {
        GatewayCmd::Status => status(&cfg),
        GatewayCmd::Start {
            bind,
            port,
            max_requests,
        } => start(&bind, port, max_requests).await,
        GatewayCmd::Stop => {
            println!(
                "argo gateway stop: requires the daemon's PID-file plumbing (§21.2 — separate PR)."
            );
            Ok(())
        }
    }
}

async fn start(bind: &str, port: u16, max_requests: Option<u64>) -> ArgoResult<()> {
    let addr = format!("{bind}:{port}");
    let listener = TcpListener::bind(&addr)
        .await
        .map_err(|e| ArgoError::Provider(format!("bind {addr}: {e}")))?;
    eprintln!("argo gateway: listening on {addr}");
    eprintln!("  GET /health  → {{\"status\":\"ok\"}}");
    eprintln!("  GET /version → {{\"version\":\"<crate>\"}}");
    eprintln!("press Ctrl-C to stop");
    let mut served = 0_u64;
    loop {
        let (mut stream, _peer) = listener
            .accept()
            .await
            .map_err(|e| ArgoError::Provider(format!("accept: {e}")))?;
        let mut buf = [0_u8; 1024];
        let n = stream.read(&mut buf).await.unwrap_or(0);
        let req = std::str::from_utf8(&buf[..n]).unwrap_or("");
        let body = if req.starts_with("GET /version ") {
            format!("{{\"version\":\"{}\"}}", env!("CARGO_PKG_VERSION"))
        } else if req.starts_with("GET /health ") {
            "{\"status\":\"ok\"}".to_string()
        } else {
            "{\"error\":\"not found\"}".to_string()
        };
        let resp = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        let _ = stream.write_all(resp.as_bytes()).await;
        let _ = stream.shutdown().await;
        served += 1;
        if max_requests.is_some_and(|cap| served >= cap) {
            eprintln!("argo gateway: served {served} request(s); exiting per --max-requests");
            return Ok(());
        }
    }
}

fn status(cfg: &ArgoConfig) -> ArgoResult<()> {
    println!("gateway:");
    println!(
        "  enabled:   {} (set [gateway].enabled in argo.toml)",
        cfg.gateway.enabled
    );
    if cfg.gateway.adapters.is_empty() {
        println!("  adapters:  (none configured)");
    } else {
        println!("  adapters:");
        for a in &cfg.gateway.adapters {
            println!("    - {a}");
        }
    }
    Ok(())
}
