//! `argo gateway` — start / stop / status. PID file at
//! \`<ARGO_HOME>/gateway.pid\` mediates the start↔stop handshake.
use crate::cli::GatewayCmd;
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_config::ArgoConfig;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use std::path::PathBuf;
use std::sync::Arc;
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

    // Open the session store so /sessions can serve. Failure to
    // open is logged but doesn't kill the gateway — /health and
    // /version still answer.
    let cwd_for_store =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let store = match open_store(&cwd_for_store).await {
        Ok(s) => Some(Arc::new(s)),
        Err(e) => {
            eprintln!("argo gateway: store open failed ({e}); /sessions will return 503");
            None
        }
    };

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
    eprintln!("  GET /health   → {{\"status\":\"ok\"}}");
    eprintln!("  GET /version  → {{\"version\":\"<crate>\"}}");
    eprintln!("  GET /sessions → recent sessions (JSON array)");
    eprintln!("  GET /skills   → active skills (JSON array)");
    eprintln!("  GET /memory   → active memory entries (JSON array)");
    eprintln!("press Ctrl-C or `argo gateway stop` to terminate");
    let mut served = 0_u64;
    let shutdown = shutdown_signal();
    tokio::pin!(shutdown);
    loop {
        tokio::select! {
            biased;
            _ = &mut shutdown => {
                eprintln!("argo gateway: shutdown signal received; exiting cleanly");
                return Ok(());
            }
            accept = listener.accept() => {
                let (mut stream, _peer) = accept
                    .map_err(|e| ArgoError::Provider(format!("accept: {e}")))?;
                let mut buf = [0_u8; 1024];
                let n = stream.read(&mut buf).await.unwrap_or(0);
                let req = std::str::from_utf8(&buf[..n]).unwrap_or("");
                let (status_line, body) = if req.starts_with("GET /version ") {
                    (
                        "200 OK",
                        format!("{{\"version\":\"{}\"}}", env!("CARGO_PKG_VERSION")),
                    )
                } else if req.starts_with("GET /health ") {
                    ("200 OK", "{\"status\":\"ok\"}".to_string())
                } else if req.starts_with("GET /sessions ") {
                    sessions_response(store.as_ref()).await
                } else if req.starts_with("GET /skills ") {
                    skills_response(store.as_ref()).await
                } else if req.starts_with("GET /memory ") {
                    memory_response(store.as_ref()).await
                } else {
                    (
                        "404 Not Found",
                        "{\"error\":\"not found\"}".to_string(),
                    )
                };
                let resp = format!(
                    "HTTP/1.1 {status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(resp.as_bytes()).await;
                let _ = stream.shutdown().await;
                served += 1;
                if max_requests.is_some_and(|cap| served >= cap) {
                    eprintln!(
                        "argo gateway: served {served} request(s); exiting per --max-requests"
                    );
                    return Ok(());
                }
            }
        }
    }
}

/// Future that resolves when SIGTERM or SIGINT (Ctrl-C) arrives.
/// On non-Unix targets we only watch Ctrl-C.
async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        let mut term = match signal(SignalKind::terminate()) {
            Ok(s) => s,
            Err(_) => return std::future::pending().await,
        };
        let mut int = match signal(SignalKind::interrupt()) {
            Ok(s) => s,
            Err(_) => return std::future::pending().await,
        };
        tokio::select! {
            _ = term.recv() => {}
            _ = int.recv() => {}
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
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

async fn open_store(cwd: &std::path::Path) -> ArgoResult<SessionStore> {
    let home = resolve_home(cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    Ok(store)
}

/// Build the response for `GET /sessions`. Returns a `(status_line,
/// body)` pair so the call-site can stay flat.
async fn sessions_response(store: Option<&Arc<SessionStore>>) -> (&'static str, String) {
    let Some(store) = store else {
        return (
            "503 Service Unavailable",
            "{\"error\":\"session store unavailable\"}".to_string(),
        );
    };
    match store.list_all_sessions(100).await {
        Ok(rows) => {
            let arr: Vec<serde_json::Value> = rows
                .iter()
                .map(|s| {
                    serde_json::json!({
                        "id": s.id.0.to_string(),
                        "model": s.model,
                        "provider": s.provider,
                        "title": s.title,
                        "started_at": s.started_at.to_rfc3339(),
                        "ended_at": s.ended_at.map(|d| d.to_rfc3339()),
                    })
                })
                .collect();
            (
                "200 OK",
                serde_json::to_string(&arr).unwrap_or_else(|_| "[]".to_string()),
            )
        }
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"list sessions: {e}\"}}"),
        ),
    }
}

async fn skills_response(store: Option<&Arc<SessionStore>>) -> (&'static str, String) {
    let Some(store) = store else {
        return (
            "503 Service Unavailable",
            "{\"error\":\"session store unavailable\"}".to_string(),
        );
    };
    let skills = argo_skill::SkillStore::new(Arc::clone(store));
    match skills.list_active().await {
        Ok(rows) => {
            let arr: Vec<serde_json::Value> = rows
                .iter()
                .map(|s| {
                    serde_json::json!({
                        "id": s.id.0.to_string(),
                        "name": s.frontmatter.name,
                        "version": s.frontmatter.version,
                        "lock": format!("{:?}", s.frontmatter.lock),
                        "hash": hex::encode(s.hash),
                        "created_at": s.created_at.to_rfc3339(),
                    })
                })
                .collect();
            (
                "200 OK",
                serde_json::to_string(&arr).unwrap_or_else(|_| "[]".to_string()),
            )
        }
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"list skills: {e}\"}}"),
        ),
    }
}

async fn memory_response(store: Option<&Arc<SessionStore>>) -> (&'static str, String) {
    use argo_memory::{MemoryFile, MemoryStore};
    let Some(store) = store else {
        return (
            "503 Service Unavailable",
            "{\"error\":\"session store unavailable\"}".to_string(),
        );
    };
    let workspace = match store.first_workspace().await {
        Ok(Some(w)) => w,
        Ok(None) => return ("200 OK", "[]".to_string()),
        Err(e) => {
            return (
                "500 Internal Server Error",
                format!("{{\"error\":\"first_workspace: {e}\"}}"),
            );
        }
    };
    let memory = MemoryStore::new(Arc::clone(store));
    let mut all: Vec<serde_json::Value> = Vec::new();
    for f in [
        MemoryFile::Memory,
        MemoryFile::User,
        MemoryFile::Soul,
        MemoryFile::Agents,
    ] {
        match memory.list_active(workspace, f).await {
            Ok(entries) => {
                for e in entries {
                    all.push(serde_json::json!({
                        "id": e.id.to_string(),
                        "file": format!("{:?}", e.file),
                        "bytes": e.body.len(),
                        "lock": format!("{:?}", e.lock),
                        "hash": hex::encode(e.hash),
                        "learned_at": e.learned_at.to_rfc3339(),
                    }));
                }
            }
            Err(e) => {
                return (
                    "500 Internal Server Error",
                    format!("{{\"error\":\"list {f:?}: {e}\"}}"),
                );
            }
        }
    }
    (
        "200 OK",
        serde_json::to_string(&all).unwrap_or_else(|_| "[]".to_string()),
    )
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
