//! `argo gateway` — start / stop / status. PID file at
//! \`<ARGO_HOME>/gateway.pid\` mediates the start↔stop handshake.
use crate::cli::GatewayCmd;
use crate::home::{ensure_dir, resolve_home};
use argo_config::ArgoConfig;
use argo_types::{ArgoError, ArgoResult};
use std::path::PathBuf;
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
        GatewayCmd::Stop => stop().await,
    }
}

fn pid_path() -> ArgoResult<PathBuf> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    Ok(home.join("gateway.pid"))
}

async fn start(bind: &str, port: u16, max_requests: Option<u64>) -> ArgoResult<()> {
    let addr = format!("{bind}:{port}");
    let listener = TcpListener::bind(&addr)
        .await
        .map_err(|e| ArgoError::Provider(format!("bind {addr}: {e}")))?;

    // PID file. If one already exists with a live PID, refuse to
    // start (would conflict with the existing instance). If it
    // exists but the process is gone, overwrite.
    let pidfile = pid_path()?;
    if pidfile.exists() {
        if let Ok(prev) = std::fs::read_to_string(&pidfile) {
            if let Ok(prev_pid) = prev.trim().parse::<i32>() {
                if pid_is_alive(prev_pid) {
                    return Err(ArgoError::Provider(format!(
                        "gateway already running (pid {prev_pid}, see {})",
                        pidfile.display()
                    )));
                }
                eprintln!(
                    "argo gateway: stale pid {prev_pid} in {}; overwriting",
                    pidfile.display()
                );
            }
        }
    }
    let pid = std::process::id();
    std::fs::write(&pidfile, format!("{pid}\n"))
        .map_err(|e| ArgoError::Provider(format!("write {}: {e}", pidfile.display())))?;
    let _guard = PidGuard(pidfile.clone());

    eprintln!("argo gateway: listening on {addr} (pid {pid})");
    eprintln!("  GET /health  → {{\"status\":\"ok\"}}");
    eprintln!("  GET /version → {{\"version\":\"<crate>\"}}");
    eprintln!("press Ctrl-C or `argo gateway stop` to terminate");
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

async fn stop() -> ArgoResult<()> {
    let pidfile = pid_path()?;
    let raw = std::fs::read_to_string(&pidfile)
        .map_err(|e| ArgoError::Provider(format!("no PID file at {}: {e}", pidfile.display())))?;
    let pid: i32 = raw
        .trim()
        .parse()
        .map_err(|e| ArgoError::Provider(format!("invalid PID in {}: {e}", pidfile.display())))?;
    if !pid_is_alive(pid) {
        let _ = std::fs::remove_file(&pidfile);
        return Err(ArgoError::Provider(format!(
            "no live process at pid {pid} (cleaned stale {})",
            pidfile.display()
        )));
    }
    if let Err(e) = signal_term(pid) {
        return Err(ArgoError::Provider(format!("kill pid {pid}: {e}")));
    }
    println!("argo gateway: sent SIGTERM to pid {pid}");
    Ok(())
}

/// RAII helper: best-effort delete the PID file on drop so a
/// graceful exit (or panic) doesn't leave a stale file behind.
struct PidGuard(PathBuf);

impl Drop for PidGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

#[cfg(unix)]
fn pid_is_alive(pid: i32) -> bool {
    // signal 0 = "do not actually send a signal, just check that
    // the target process exists and we have permission to signal it".
    unsafe { libc::kill(pid, 0) == 0 }
}

#[cfg(not(unix))]
fn pid_is_alive(_pid: i32) -> bool {
    // Conservative on non-Unix: assume alive if we got this far.
    true
}

#[cfg(unix)]
fn signal_term(pid: i32) -> std::io::Result<()> {
    if unsafe { libc::kill(pid, libc::SIGTERM) } == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(not(unix))]
fn signal_term(_pid: i32) -> std::io::Result<()> {
    Err(std::io::Error::other(
        "SIGTERM not supported on this platform",
    ))
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
