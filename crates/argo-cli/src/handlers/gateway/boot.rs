//! Gateway lifecycle commands — `start` + `stop` + the supporting
//! `pid_path` / `open_store` helpers. Extracted from `mod.rs` to keep
//! that file under the 500-LOC ceiling.

use crate::home::{cwd_workspace, db_path, ensure_dir, resolve_home};
use argo_orchestrator::{
    spawn_duty_scheduler, ClosedGate, DutyScheduler, DutyStore, OrchestratorStore,
};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

use super::lifecycle::{pid_is_alive, signal_term, PidGuard};
use super::rate_limit_ip::{self, rate_limited_response_bytes, CheckResult, IpRateLimiter};
use super::shutdown::{self, ShutdownController};
use super::start_banner::print_banner;
use super::start_banner_config::load_layered;
use super::startup_check::{run_startup_checks, STARTUP_CHECK_EXIT};
use super::{
    approval_expiry_sweeper, metrics, routes, routes_channels, routes_orchestrator, routes_skills,
    runs_streaming, security_headers, streaming, sweepers,
};
use crate::handlers::alerts;
use crate::handlers::cu_screenshots::CuScreenshotStore;
use argo_forge::{spawn_forge_cron, ForgeCronConfig};
use argo_store::attached_blob::{BlobStore, MemoryAttachedBlob};

const DEFAULT_DUTY_SCHEDULER_SECS: u64 = 60;
/// Sweep cadence for the `cu_screenshot` retention task. Hourly is
/// the brief's target — short enough to keep the index lean,
/// long enough that the sweep cost is negligible against the
/// `(retention_until)` index.
const CU_SCREENSHOT_SWEEP_SECS: u64 = 3_600;
/// Default retention horizon for CU screenshots: 14 days. Operator
/// override via `ARGO_CU_SCREENSHOT_RETENTION_DAYS`.
const DEFAULT_CU_SCREENSHOT_RETENTION_DAYS: u64 = 14;

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
    profile_startup: bool,
    skip_startup_check: bool,
    insecure_bind: bool,
) -> ArgoResult<()> {
    let mut phases = StartupPhases::new();
    let cwd_for_check =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    if let Err(e) = run_startup_checks(skip_startup_check, &cwd_for_check).await {
        eprintln!("argo gateway: STARTUP CHECK FAILED ({e}); refusing to bind");
        std::process::exit(STARTUP_CHECK_EXIT);
    }
    phases.mark("startup_check");
    check_bind_loopback(bind, insecure_bind)?;
    let addr = format!("{bind}:{port}");
    let listener = TcpListener::bind(&addr)
        .await
        .map_err(|e| ArgoError::Provider(format!("bind {addr}: {e}")))?;
    phases.mark("tcp_bind");

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
    phases.mark("store_open_migrate");

    let bg_store = store.as_ref();
    let _sweeper = bg_store.map(|s| sweepers::spawn_handoff_sweeper(Arc::clone(s)));
    let _approval_expiry =
        bg_store.map(|s| approval_expiry_sweeper::spawn_approval_expiry_sweeper(Arc::clone(s)));
    let _duty_scheduler = bg_store.map(|s| spawn_gateway_duty_scheduler(Arc::clone(s)));
    let _alerts_loop = bg_store.map(|s| spawn_alerts_loop(Arc::clone(s)));
    let _cu_screenshot_sweep = bg_store.map(|s| spawn_cu_screenshot_sweep(Arc::clone(s)));
    let _audit_watchdog = bg_store.map(|s| spawn_audit_watchdog(Arc::clone(s)));
    let _forge_cron = bg_store
        .map(|s| AbortOnDrop::new(spawn_forge_cron(Arc::clone(s), ForgeCronConfig::from_env())));
    let _mcp_health = crate::handlers::mcp_health::spawn_from_env();
    phases.mark("spawn_bg_tasks");

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
    phases.mark("pidfile");

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
    phases.mark("workspace_lookup");
    let argo_toml = cwd_for_store.join("argo.toml");
    let banner_cfg = load_layered(Some(&argo_toml), cli_quiet);
    print_banner(&banner_cfg);
    phases.mark("banner");

    if profile_startup {
        phases.emit();
        eprintln!("argo gateway: --profile-startup set, exiting before serve loop");
        return Ok(());
    }

    let ip_limiter = Arc::new(IpRateLimiter::from_env());
    let mut served = 0_u64;
    let shutdown_ctl = ShutdownController::new();
    let mut shutdown_rx = shutdown::install_signal_handler(shutdown_ctl.clone());
    let mut watchdog: Option<JoinHandle<()>> = None;
    loop {
        tokio::select! {
            biased;
            recv = &mut shutdown_rx => {
                watchdog = recv.ok();
                eprintln!("argo gateway: stopping accept loop; SIGTERM observed");
                break;
            }
            accept = listener.accept() => {
                let (mut stream, peer) = accept
                    .map_err(|e| ArgoError::Provider(format!("accept: {e}")))?;
                let mut buf = [0_u8; 4096];
                let n = stream.read(&mut buf).await.unwrap_or(0);
                let req = std::str::from_utf8(&buf[..n]).unwrap_or("");
                let src_ip = rate_limit_ip::extract_ip(req, peer, ip_limiter.trust_xff());
                let https = security_headers::request_is_https(req);
                let sec = security_headers::header_lines("application/json", https);
                if let CheckResult::Deny { retry_after_secs } = ip_limiter.check(src_ip) {
                    tracing::warn!(
                        target: "gateway::rate_limit_ip",
                        ip = %src_ip,
                        retry_after_secs,
                        "per-IP rate limit triggered"
                    );
                    let bytes = rate_limited_response_bytes(retry_after_secs);
                    let _ = stream.write_all(&bytes).await;
                    let _ = stream.shutdown().await;
                } else if super::rate_limit_tenant::write_if_throttled(req, &mut stream, &sec).await {
                } else if runs_streaming::is_streaming_runs_request(req) {
                    let _ = runs_streaming::serve_streamed_run(req, store.as_ref(), &mut stream).await;
                    let _ = stream.shutdown().await;
                } else if streaming::is_streaming_list_request(req) {
                    let _ = streaming::serve_streamed_list(req, store.as_ref(), &mut stream).await;
                    let _ = stream.shutdown().await;
                } else if metrics::is_metrics_request(req) {
                    let bytes = metrics::http_response(store.as_ref()).await;
                    let _ = stream.write_all(&bytes).await;
                    let _ = stream.shutdown().await;
                } else if super::ws_upgrade::is_ws_request(req) {
                    let _ = super::ws_upgrade::serve(req, store.as_ref(), &mut stream).await;
                    let _ = stream.shutdown().await;
                } else if routes_channels::is_email_webhook(req) {
                    let (status_line, body) = routes_channels::email_webhook_response(req).await;
                    let resp = format!("HTTP/1.1 {status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n{sec}\r\n{}", body.len(), body);
                    let _ = stream.write_all(resp.as_bytes()).await; let _ = stream.shutdown().await;
                } else if let Some((status_line, body)) = routes_orchestrator::dispatch_route(req, store.as_ref()).await {
                    let resp = format!("HTTP/1.1 {status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n{sec}\r\n{}", body.len(), body);
                    let _ = stream.write_all(resp.as_bytes()).await;
                    let _ = stream.shutdown().await;
                } else if let Some((status_line, body)) = routes_skills::dispatch_route(req, store.as_ref()).await {
                    let resp = format!(
                        "HTTP/1.1 {status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n{sec}\r\n{}",
                        body.len(),
                        body
                    );
                    let _ = stream.write_all(resp.as_bytes()).await;
                    let _ = stream.shutdown().await;
                } else {
                    let (status_line, body) = routes::dispatch_route(req, store.as_ref(), &shutdown_ctl).await;
                    let resp = format!(
                        "HTTP/1.1 {status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n{sec}\r\n{}",
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
                    break;
                }
            }
        }
    }
    finish_shutdown(&shutdown_ctl, store.as_ref(), watchdog).await;
    Ok(())
}

/// Drain in-flight requests up to `ARGO_SHUTDOWN_DEADLINE_SECS` (env,
/// default 30s), then close the session store so SQLite checkpoints
/// the WAL and releases the lock file. The force-exit watchdog (when
/// present) was armed inside the signal handler and started ticking
/// from SIGTERM time; we abort it after a clean drain+close so the
/// process exits 0 instead of 124.
async fn finish_shutdown(
    ctl: &ShutdownController,
    store: Option<&Arc<SessionStore>>,
    watchdog: Option<JoinHandle<()>>,
) {
    let deadline = Duration::from_secs(shutdown::deadline_secs_from_env());
    shutdown::drain_in_flight(ctl, deadline).await;
    if let Some(s) = store {
        s.close().await;
        eprintln!("argo gateway: session store closed");
    }
    if let Some(w) = watchdog {
        w.abort();
    }
}

fn spawn_audit_watchdog(store: Arc<SessionStore>) -> AbortOnDrop {
    let cb: Arc<dyn Fn(argo_audit::ChainBreak) + Send + Sync> = Arc::new(|b| {
        tracing::error!(
            target: "gateway::audit_watchdog",
            seq = b.seq,
            kind = %b.kind,
            reason = %b.reason,
            "audit chain break detected"
        );
    });
    eprintln!("argo gateway: audit chain watchdog enabled");
    AbortOnDrop::new(argo_audit::spawn_watchdog(
        store,
        argo_audit::WatchdogConfig::default(),
        cb,
    ))
}

fn spawn_alerts_loop(store: Arc<SessionStore>) -> AbortOnDrop {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let path = cwd.join(".skycot").join("alerts.toml");
    let cfg = match alerts::load_alerts_config(&path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("argo gateway: alerts config load failed ({e}); skipping evaluator");
            Default::default()
        }
    };
    if cfg.rules.is_empty() {
        eprintln!("argo gateway: alerts evaluator inactive (no rules)");
    } else {
        eprintln!(
            "argo gateway: alerts evaluator enabled ({} rule(s))",
            cfg.rules.len()
        );
    }
    AbortOnDrop::new(tokio::spawn(alerts::run_alerts_loop(store, cfg)))
}

fn spawn_cu_screenshot_sweep(store: Arc<SessionStore>) -> AbortOnDrop {
    let retention_days = std::env::var("ARGO_CU_SCREENSHOT_RETENTION_DAYS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .filter(|d| *d > 0)
        .unwrap_or(DEFAULT_CU_SCREENSHOT_RETENTION_DAYS);
    let retention = Duration::from_secs(retention_days.saturating_mul(86_400));
    // The gateway boots without an injected `BlobStore` yet; the
    // memory-backed store keeps the API available end-to-end and
    // can be replaced by a SQLite-backed `BlobStore` in a follow-up
    // brief without changing this wiring. Sweep operates on the
    // index, which is durable, so the choice of blob backing only
    // affects bytes-level retention (which the trait deliberately
    // doesn't expose).
    let blobs: Arc<dyn BlobStore> = Arc::new(MemoryAttachedBlob::new());
    let pool = store.pool().clone();
    let cu_store = CuScreenshotStore::new(blobs, pool, retention);
    eprintln!(
        "argo gateway: cu_screenshot sweep enabled (interval={}s, retention={} days)",
        CU_SCREENSHOT_SWEEP_SECS, retention_days
    );
    AbortOnDrop::new(tokio::spawn(async move {
        if let Err(e) = cu_store.ensure_schema().await {
            tracing::warn!(target: "gateway::cu_screenshot_sweep", error = %e, "ensure_schema failed; skipping");
            return;
        }
        let mut ticker = tokio::time::interval(Duration::from_secs(CU_SCREENSHOT_SWEEP_SECS));
        // First tick fires immediately; skip it so boot doesn't
        // race the migrate() call.
        ticker.tick().await;
        loop {
            ticker.tick().await;
            match cu_store.sweep_expired(SystemTime::now(), retention).await {
                Ok(n) if n > 0 => tracing::info!(
                    target: "gateway::cu_screenshot_sweep",
                    deleted = n,
                    "cu_screenshot sweep removed expired index rows"
                ),
                Ok(_) => {}
                Err(e) => tracing::warn!(
                    target: "gateway::cu_screenshot_sweep",
                    error = %e,
                    "cu_screenshot sweep failed; will retry on next tick"
                ),
            }
        }
    }))
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

/// Per-phase wall-clock recorder used by `--profile-startup`. Each
/// `mark()` call records the elapsed time since the previous boundary
/// under a stable `&'static str` label. `emit()` writes a stable,
/// machine-grep-able summary to stderr (one line per phase + a `total`
/// line). The struct itself stays well under 30 LOC so boot.rs keeps
/// headroom under the 500-LOC ceiling.
struct StartupPhases {
    t0: Instant,
    last: Instant,
    items: Vec<(&'static str, Duration)>,
}

impl StartupPhases {
    fn new() -> Self {
        let now = Instant::now();
        Self {
            t0: now,
            last: now,
            items: Vec::with_capacity(8),
        }
    }

    fn mark(&mut self, label: &'static str) {
        let now = Instant::now();
        self.items.push((label, now.duration_since(self.last)));
        self.last = now;
    }

    fn emit(&self) {
        eprintln!("argo gateway: startup phases (ms)");
        for (label, d) in &self.items {
            eprintln!("  phase  {:<20} {:>8.2}", label, d.as_secs_f64() * 1000.0);
        }
        eprintln!(
            "  phase  {:<20} {:>8.2}",
            "total",
            self.last.duration_since(self.t0).as_secs_f64() * 1000.0
        );
    }
}

/// Refuse to bind anything outside the loopback range unless the
/// operator passes `--insecure-bind`. A literal IP that parses to a
/// loopback address (`127.0.0.0/8`, `::1`) is always accepted. Any
/// other literal IP — and any non-IP value (e.g. a hostname like
/// `localhost` or `0`-shorthand) — is treated as non-loopback and
/// requires the explicit opt-in. The opt-in path prints a single
/// yellow stderr warning so reverse-proxy operators see the
/// reminder on every boot.
pub(super) fn check_bind_loopback(bind: &str, insecure_bind: bool) -> ArgoResult<()> {
    let is_loopback = bind
        .parse::<IpAddr>()
        .map(|ip| ip.is_loopback())
        .unwrap_or(false);
    if is_loopback {
        return Ok(());
    }
    if insecure_bind {
        eprintln!(
            "\x1b[33margo gateway: binding non-loopback {bind} per --insecure-bind; \
             ensure a TLS-terminating proxy is in front\x1b[0m"
        );
        return Ok(());
    }
    Err(ArgoError::Provider(format!(
        "refusing to bind non-loopback {bind} without --insecure-bind; \
         see docs/skycot/SECURITY.md"
    )))
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

#[cfg(test)]
#[path = "boot_tests.rs"]
mod tests;
