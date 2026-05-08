//! Gateway lifecycle commands — `start` + `stop` + the supporting
//! `pid_path` / `open_store` helpers. Extracted from `mod.rs` to keep
//! that file under the 500-LOC ceiling.

use crate::home::{cwd_workspace, db_path, ensure_dir, resolve_home};
use argo_orchestrator::{
    spawn_duty_scheduler, ClosedGate, DutyScheduler, DutyStore, OrchestratorStore,
};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

use super::lifecycle::{pid_is_alive, shutdown_signal, signal_term, PidGuard};
use super::start_banner::print_banner;
use super::start_banner_config::load_layered;
use super::{
    metrics, routes, routes_channels, routes_orchestrator, routes_skills, routes_voice, streaming,
    sweepers,
};

const DEFAULT_DUTY_SCHEDULER_SECS: u64 = 60;

struct AbortOnDrop {
    handle: JoinHandle<()>,
}

impl AbortOnDrop {
    fn new(handle: JoinHandle<()>) -> Self {
        Self { handle }
    }
}

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

pub(super) fn pid_path() -> ArgoResult<PathBuf> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    Ok(home.join("gateway.pid"))
}

async fn open_store(cwd: &std::path::Path) -> ArgoResult<SessionStore> {
    let home = resolve_home(cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    Ok(store)
}

pub(super) async fn start(
    bind: &str,
    port: u16,
    max_requests: Option<u64>,
    cli_quiet: bool,
) -> ArgoResult<()> {
    let addr = format!("{bind}:{port}");
    let listener = TcpListener::bind(&addr)
        .await
        .map_err(|e| ArgoError::Provider(format!("bind {addr}: {e}")))?;

    // Open the session store so the routes that touch it can serve.
    // Failure to open is logged but doesn't kill the gateway —
    // /health and /version still answer.
    let cwd_for_store =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let store = match open_store(&cwd_for_store).await {
        Ok(s) => Some(Arc::new(s)),
        Err(e) => {
            eprintln!("argo gateway: store open failed ({e}); /sessions etc. will return 503");
            None
        }
    };
    let _sweeper = store
        .as_ref()
        .map(|s| sweepers::spawn_handoff_sweeper(Arc::clone(s)));
    let _duty_scheduler = store
        .as_ref()
        .map(|s| spawn_gateway_duty_scheduler(Arc::clone(s)));

    // PID file. If one already exists with a live PID, refuse to
    // start. If it exists but the process is gone, overwrite.
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

    metrics::init_boot_time();
    eprintln!("argo gateway: listening on {addr} (pid {pid})");
    if let Some(s) = store.as_ref() {
        // #718: print the cwd-bound workspace's name (matches the path
        // every request handler resolves to). Pre-fix this used
        // `first_workspace_name()` and on multi-workspace dbs surfaced
        // a stale workspace's name.
        if let Ok(id) = cwd_workspace(s).await {
            if let Ok(Some((_, name, _, _))) = s.workspace_info(id).await {
                eprintln!("argo gateway: workspace = '{name}'");
            }
        }
    }
    let argo_toml = cwd_for_store.join("argo.toml");
    let banner_cfg = load_layered(Some(&argo_toml), cli_quiet);
    print_banner(&banner_cfg);

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
                let mut buf = [0_u8; 4096];
                let n = stream.read(&mut buf).await.unwrap_or(0);
                let req = std::str::from_utf8(&buf[..n]).unwrap_or("");
                if streaming::is_streaming_runs_request(req) {
                    let _ = streaming::serve_streamed_run(req, store.as_ref(), &mut stream).await;
                    let _ = stream.shutdown().await;
                } else if metrics::is_metrics_request(req) {
                    let bytes = metrics::http_response(store.as_ref()).await;
                    let _ = stream.write_all(&bytes).await;
                    let _ = stream.shutdown().await;
                } else if routes_channels::is_email_webhook(req) {
                    let (status_line, body) = routes_channels::email_webhook_response(req).await;
                    let resp = format!("HTTP/1.1 {status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}", body.len(), body);
                    let _ = stream.write_all(resp.as_bytes()).await; let _ = stream.shutdown().await;
                } else if routes_voice::is_voice_webhook(req) {
                    let (status_line, body) = routes_voice::voice_webhook_response(req).await;
                    let resp = format!("HTTP/1.1 {status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}", body.len(), body);
                    let _ = stream.write_all(resp.as_bytes()).await; let _ = stream.shutdown().await;
                } else if let Some((status_line, body)) = routes_orchestrator::dispatch_route(req, store.as_ref()).await {
                    let resp = format!("HTTP/1.1 {status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}", body.len(), body);
                    let _ = stream.write_all(resp.as_bytes()).await;
                    let _ = stream.shutdown().await;
                } else if let Some((status_line, body)) = routes_skills::dispatch_route(req, store.as_ref()).await {
                    let resp = format!(
                        "HTTP/1.1 {status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                        body.len(),
                        body
                    );
                    let _ = stream.write_all(resp.as_bytes()).await;
                    let _ = stream.shutdown().await;
                } else {
                    let (status_line, body) = routes::dispatch_route(req, store.as_ref()).await;
                    let resp = format!(
                        "HTTP/1.1 {status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                        body.len(),
                        body
                    );
                    let _ = stream.write_all(resp.as_bytes()).await;
                    let _ = stream.shutdown().await;
                }
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

fn spawn_gateway_duty_scheduler(store: Arc<SessionStore>) -> AbortOnDrop {
    let interval_secs = std::env::var("ARGO_DUTY_SCHEDULER_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .filter(|secs| *secs > 0)
        .unwrap_or(DEFAULT_DUTY_SCHEDULER_SECS);
    let duties = Arc::new(DutyStore::new(Arc::clone(&store)));
    let orchestrator = Arc::new(OrchestratorStore::new(store));
    // V1 fail-safe: run the autonomous duty loop but deny every
    // fire until the live auto-approval gate is wired and verified.
    let gate = Arc::new(ClosedGate);
    let scheduler = Arc::new(DutyScheduler::new(duties, orchestrator, gate));
    eprintln!(
        "argo gateway: duty scheduler enabled (interval={}s, gate=closed)",
        interval_secs
    );
    AbortOnDrop::new(spawn_duty_scheduler(
        scheduler,
        Duration::from_secs(interval_secs),
    ))
}

pub(super) async fn stop() -> ArgoResult<()> {
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
