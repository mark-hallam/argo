//! Gateway lifecycle commands — `start` + `stop` + the supporting
//! `pid_path` / `open_store` helpers. Extracted from `mod.rs` to keep
//! that file under the 500-LOC ceiling.

use crate::home::{cwd_workspace, db_path, ensure_dir, resolve_home};
use argo_audit::{AuditKind, AuditSink, SYSTEM_AUDIT_TENANT_ID};
use argo_store::SessionStore;
use argo_telemetry::span_context::{enter_context, LogContext};
use argo_types::{ArgoError, ArgoResult, TenantId};
use serde_json::{json, Value};
use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

#[cfg(test)]
use super::boot_helpers::check_bind_loopback;
use super::boot_helpers::{http_status_label, parse_status_line, StartupPhases};
use super::boot_spawns::{
    finish_shutdown, spawn_alerts_loop, spawn_approval_reminder, spawn_audit_watchdog,
    spawn_cu_screenshot_sweep, spawn_gateway_duty_scheduler, spawn_siem_from_config,
    spawn_worker_supervisor,
};
use super::ip_allowlist::{self, AllowResult, IpAllowlist};
use super::lifecycle::{pid_is_alive, signal_term, PidGuard};
use super::rate_limit_ip::{self, rate_limited_response_bytes, CheckResult, IpRateLimiter};
use super::shutdown::{self, ShutdownController};
use super::start_banner::print_banner;
use super::start_banner_config::load_layered;
use super::startup_check::{run_startup_checks, validate_bind_safety, STARTUP_CHECK_EXIT};
use super::{
    approval_expiry_sweeper, metrics, proxy_guard, request_audit, routes, routes_channels,
    routes_openmetrics, routes_orchestrator, routes_skills, runs_streaming, security_headers,
    streaming, sweepers,
};
use argo_config::ArgoConfig;
use argo_forge::{spawn_forge_cron, ForgeCronConfig};

pub(super) struct AbortOnDrop {
    handle: JoinHandle<()>,
}

impl AbortOnDrop {
    pub(super) fn new(handle: JoinHandle<()>) -> Self {
        Self { handle }
    }
}

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

pub(super) fn pid_path() -> ArgoResult<PathBuf> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
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
    bind_non_loopback: bool,
) -> ArgoResult<()> {
    // Boot-time context: tenant_id resolves per-request inside the
    // route handlers — this entry covers everything that emits before
    // a request is parsed (banner, startup checks, bg spawns) with
    // the anon tenant marker.
    let _boot_ctx = enter_context(LogContext::default());
    let mut phases = StartupPhases::new();
    let cwd_for_check =
        std::env::current_dir().map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    // Load-bearing bind-safety guard runs FIRST, so plaintext
    // non-loopback bind without TLS/proxy config is refused even
    // under `--skip-startup-check`.
    validate_bind_safety(bind, bind_non_loopback, &cwd_for_check)?;
    if let Err(e) = run_startup_checks(skip_startup_check, &cwd_for_check).await {
        eprintln!("argo gateway: STARTUP CHECK FAILED ({e}); refusing to bind");
        std::process::exit(STARTUP_CHECK_EXIT);
    }
    phases.mark("startup_check");
    let mut cfg = ArgoConfig::load(&cwd_for_check.join("argo.toml")).unwrap_or_default();
    if let Ok(env_cfg) = ArgoConfig::from_env() {
        cfg.merge(env_cfg);
    }
    let _error_reporting = super::boot_telemetry::init_gateway_error_reporting(&cfg);
    let gateway_cfg = cfg.gateway.clone();
    let behind_proxy = gateway_cfg.behind_proxy.clone();
    let request_audit_cfg = gateway_cfg.request_audit.clone();
    let prebound_store = if bind_non_loopback && !is_loopback_bind(bind) {
        match open_store(&cwd_for_check).await {
            Ok(s) => {
                let store = Arc::new(s);
                emit_insecure_bind_audit(&store, bind, port).await;
                Some(store)
            }
            Err(e) => {
                eprintln!(
                    "argo gateway: bind-non-loopback audit unavailable before bind ({e}); \
                     continuing"
                );
                None
            }
        }
    } else {
        None
    };
    let addr = format!("{bind}:{port}");
    let listener = TcpListener::bind(&addr)
        .await
        .map_err(|e| ArgoError::Provider(format!("bind {addr}: {e}")))?;
    phases.mark("tcp_bind");

    // Open the session store so the routes that touch it can serve.
    // Failure to open is logged but doesn't kill the gateway —
    // /health and /version still answer.
    let cwd_for_store =
        std::env::current_dir().map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let store = match prebound_store {
        Some(s) => Some(s),
        None => match open_store(&cwd_for_store).await {
            Ok(s) => Some(Arc::new(s)),
            Err(e) => {
                eprintln!("argo gateway: store open failed ({e}); /sessions etc. will return 503");
                None
            }
        },
    };
    phases.mark("store_open_migrate");
    if let Some(s) = store.as_ref() {
        super::hipaa_startup_check::emit_startup_check_best_effort(&cwd_for_store, s).await;
    }

    let bg_store = store.as_ref();
    let _sweeper = bg_store.map(|s| sweepers::spawn_handoff_sweeper(Arc::clone(s)));
    let _approval_expiry =
        bg_store.map(|s| approval_expiry_sweeper::spawn_approval_expiry_sweeper(Arc::clone(s)));
    let _approval_reminder = bg_store.map(|s| spawn_approval_reminder(Arc::clone(s)));
    let _duty_scheduler = bg_store.map(|s| spawn_gateway_duty_scheduler(Arc::clone(s)));
    let _alerts_loop = bg_store.map(|s| spawn_alerts_loop(Arc::clone(s)));
    let _cu_screenshot_sweep = bg_store.map(|s| spawn_cu_screenshot_sweep(Arc::clone(s)));
    let _audit_watchdog = bg_store.map(|s| spawn_audit_watchdog(Arc::clone(s)));
    let _forge_cron = bg_store
        .map(|s| AbortOnDrop::new(spawn_forge_cron(Arc::clone(s), ForgeCronConfig::from_env())));
    #[rustfmt::skip]
    let _skill_learning_crons = bg_store.and_then(|s| {
        let learning_cfg = cfg.skill_learning.clone();
        if !learning_cfg.enabled { eprintln!("argo gateway: skill learning cron disabled by config"); return None; }
        let store = Arc::clone(s);
        Some(AbortOnDrop::new(tokio::spawn(async move {
            let tenant_filter = match learning_cfg.tenant_filter.as_deref().map(uuid::Uuid::parse_str).transpose() {
                Ok(v) => v.map(TenantId),
                Err(e) => { eprintln!("argo gateway: invalid [skill_learning].tenant_filter ({e}); skipping cron"); return; }
            };
            let rows = match tenant_filter {
                Some(t) => sqlx::query_as::<_, (String, String)>(
                    "SELECT id, tenant_id FROM worker WHERE tenant_id = ? AND archived_at IS NULL",
                ).bind(t.to_string()).fetch_all(store.pool()).await,
                None => sqlx::query_as::<_, (String, String)>(
                    "SELECT id, tenant_id FROM worker WHERE archived_at IS NULL",
                ).fetch_all(store.pool()).await,
            };
            let rows = match rows { Ok(rows) => rows, Err(e) => {
                tracing::warn!(target: "gateway::skill_learning", error = %e, "list workers failed; cron disabled"); return;
            }};
            let skills = Arc::new(argo_skill::SkillStore::new(Arc::clone(&store)));
            let interval = Duration::from_secs(learning_cfg.interval_secs.max(1));
            let mut handles = Vec::new();
            for (worker_raw, tenant_raw) in rows {
                let (Ok(worker), Ok(tenant)) =
                    (worker_raw.parse::<argo_store::WorkerId>(), uuid::Uuid::parse_str(&tenant_raw))
                else { continue };
                let tick_cfg = argo_skill::PerWorkerLearningConfig { tenant_id: Some(TenantId(tenant)), ..Default::default() };
                handles.push(AbortOnDrop::new(argo_skill::spawn_per_worker_learning_cron(
                    worker, Arc::clone(&skills), tick_cfg, interval,
                )));
            }
            eprintln!("argo gateway: skill learning cron enabled (workers={}, interval={}s)", handles.len(), interval.as_secs());
            std::future::pending::<()>().await;
            drop(handles);
        })))
    });
    let _siem_dispatcher = bg_store.and_then(|s| {
        let cfg = ArgoConfig::load(&cwd_for_store.join("argo.toml")).unwrap_or_default();
        spawn_siem_from_config(Arc::clone(s), &cfg.siem)
    });
    let _mcp_health = crate::handlers::mcp_health::spawn_from_env();
    let _worker_supervisor = spawn_worker_supervisor();
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
        .map_err(|e| ArgoError::Io(format!("write {}: {e}", pidfile.display())))?;
    let _guard = PidGuard(pidfile.clone());
    phases.mark("pidfile");

    metrics::init_boot_time();
    // Auto-wire the brief-specified prometheus metrics + stamp the
    // boot-time `store_migration_chain_version` gauge before the first
    // `/openmetrics` scrape can hit. Idempotent.
    routes_openmetrics::ensure_metrics_registered();
    routes_openmetrics::brief_store_migration_chain_version()
        .set(i64::from(argo_store::EXPECTED_SCHEMA_VERSION));
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

    let ip_limiter = Arc::new(IpRateLimiter::from_config(&gateway_cfg.rate_limit.ip));
    // Source-CIDR allowlist (`mission-ip-allowlist-deployment-mode`).
    // Layered ahead of the bearer-auth path so a leaked token alone
    // isn't sufficient for access. Disabled by default (the load path
    // returns an Arc<IpAllowlist> with `enabled = false` which short-
    // circuits the check). A malformed CIDR is an operator typo —
    // refuse to start rather than silently widen the gate.
    let allowlist_cfg = cfg.security.ip_allowlist;
    let ip_allowlist = match IpAllowlist::from_config(&allowlist_cfg) {
        Ok(g) => Arc::new(g),
        Err(e) => {
            return Err(ArgoError::Provider(format!(
                "[security.ip_allowlist] config invalid: {e}"
            )));
        }
    };
    if ip_allowlist.enabled() {
        eprintln!(
            "argo gateway: ip allowlist ENABLED ({} CIDR(s))",
            ip_allowlist.cidr_count()
        );
    }
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
                let req_t0 = Instant::now();
                let req_method = req
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .to_string();
                let req_route = super::rate_limit::classify_request(req).into_owned();
                let mut req_status: u16 = 200;
                let src_ip = if behind_proxy.expected_proxy {
                    proxy_guard::extract_real_ip(req, peer, &behind_proxy)
                } else {
                    rate_limit_ip::extract_ip(req, peer, ip_limiter.trust_xff())
                };
                let mut audit = request_audit::capture(req, peer.ip(), src_ip, &request_audit_cfg);
                let https = security_headers::request_is_https(req);
                let sec = security_headers::header_lines(
                    "application/json",
                    https,
                    &gateway_cfg.security_headers,
                );
                // `mission-ip-allowlist-deployment-mode`: CIDR gate
                // runs BEFORE the bearer-auth check so a leaked token
                // alone is insufficient for access. Deny → 403 with
                // empty body (no info-leak about what was gated).
                if let AllowResult::Deny = ip_allowlist.check(req, peer) {
                    let logged_ip = ip_allowlist.source_ip(req, peer);
                    ip_allowlist::log_deny(logged_ip, req);
                    let bytes = ip_allowlist::deny_response_bytes();
                    let _ = stream.write_all(&bytes).await;
                    let _ = stream.shutdown().await;
                } else if let CheckResult::Deny { retry_after_secs } = ip_limiter.check(src_ip) {
                    tracing::warn!(
                        target: "gateway::rate_limit_ip",
                        ip = %src_ip,
                        retry_after_secs,
                        "per-IP rate limit triggered"
                    );
                    routes_openmetrics::brief_rate_limit_throttle_total()
                        .with_label_values(&["ip", ""])
                        .inc();
                    let bytes = rate_limited_response_bytes(retry_after_secs);
                    let _ = stream.write_all(&bytes).await;
                    let body_len = request_audit::response_body_len_from_bytes(&bytes);
                    request_audit::spawn_record(audit.take(), store.as_ref(), "429 Too Many Requests", body_len);
                    let _ = stream.shutdown().await;
                    req_status = 429;
                } else if super::rate_limit_tenant::write_if_throttled(req, &mut stream, &sec).await {
                    routes_openmetrics::brief_rate_limit_throttle_total()
                        .with_label_values(&["tenant", &super::rate_limit::extract_tenant(req)])
                        .inc();
                    request_audit::spawn_record(audit.take(), store.as_ref(), "429 Too Many Requests", 0);
                    req_status = 429;
                } else if runs_streaming::is_streaming_runs_request(req) {
                    let _ = runs_streaming::serve_streamed_run(req, store.as_ref(), &mut stream).await;
                    request_audit::spawn_record(audit.take(), store.as_ref(), "200 OK", 0);
                    let _ = stream.shutdown().await;
                } else if streaming::is_streaming_list_request(req) {
                    let _ = streaming::serve_streamed_list(req, store.as_ref(), &mut stream).await;
                    request_audit::spawn_record(audit.take(), store.as_ref(), "200 OK", 0);
                    let _ = stream.shutdown().await;
                } else if metrics::is_metrics_request(req) {
                    let bytes = metrics::http_response(store.as_ref()).await;
                    let _ = stream.write_all(&bytes).await;
                    let body_len = request_audit::response_body_len_from_bytes(&bytes);
                    request_audit::spawn_record(audit.take(), store.as_ref(), "200 OK", body_len);
                    let _ = stream.shutdown().await;
                } else if routes_openmetrics::is_openmetrics_request(req) {
                    let bytes = routes_openmetrics::http_response();
                    let _ = stream.write_all(&bytes).await;
                    let _ = stream.shutdown().await;
                } else if super::ws_upgrade::is_ws_request(req) {
                    let _ = super::ws_upgrade::serve(req, store.as_ref(), &mut stream).await;
                    request_audit::spawn_record(audit.take(), store.as_ref(), "101 Switching Protocols", 0);
                    let _ = stream.shutdown().await;
                } else if super::sso_dispatch::is_sso_request(req) {
                    super::sso_dispatch::serve(req, store.as_ref(), &mut stream).await;
                } else if routes_channels::is_email_webhook(req) {
                    let (status_line, body) = routes_channels::email_webhook_response(req).await;
                    req_status = parse_status_line(status_line);
                    let resp = format!("HTTP/1.1 {status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n{sec}\r\n{}", body.len(), body);
                    let _ = stream.write_all(resp.as_bytes()).await; let _ = stream.shutdown().await;
                    request_audit::spawn_record(audit.take(), store.as_ref(), status_line, body.len());
                } else if let Some((status_line, body)) = routes_orchestrator::dispatch_route(req, store.as_ref()).await {
                    req_status = parse_status_line(status_line);
                    let resp = format!("HTTP/1.1 {status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n{sec}\r\n{}", body.len(), body);
                    let _ = stream.write_all(resp.as_bytes()).await;
                    request_audit::spawn_record(audit.take(), store.as_ref(), status_line, body.len());
                    let _ = stream.shutdown().await;
                } else if let Some((status_line, body)) = routes_skills::dispatch_route(req, store.as_ref()).await {
                    req_status = parse_status_line(status_line);
                    let resp = format!(
                        "HTTP/1.1 {status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n{sec}\r\n{}",
                        body.len(),
                        body
                    );
                    let _ = stream.write_all(resp.as_bytes()).await;
                    request_audit::spawn_record(audit.take(), store.as_ref(), status_line, body.len());
                    let _ = stream.shutdown().await;
                } else {
                    let (status_line, body) = routes::dispatch_route(req, store.as_ref(), &shutdown_ctl).await;
                    req_status = parse_status_line(status_line);
                    let resp = format!(
                        "HTTP/1.1 {status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n{sec}\r\n{}",
                        body.len(),
                        body
                    );
                    let _ = stream.write_all(resp.as_bytes()).await;
                    request_audit::spawn_record(audit.take(), store.as_ref(), status_line, body.len());
                    let _ = stream.shutdown().await;
                }
                let req_status_str = http_status_label(req_status);
                routes_openmetrics::brief_request_total()
                    .with_label_values(&[&req_method, &req_route, req_status_str])
                    .inc();
                routes_openmetrics::brief_request_duration()
                    .with_label_values(&[&req_method, &req_route])
                    .observe(req_t0.elapsed().as_secs_f64());
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

fn is_loopback_bind(bind: &str) -> bool {
    bind.parse::<IpAddr>()
        .map(|ip| ip.is_loopback())
        .unwrap_or(false)
}

fn insecure_bind_audit_payload(bind: &str, port: u16, pid: u32) -> Value {
    json!({
        "event_type": "gateway_insecure_bind",
        "severity": "warn",
        "actor": "system",
        "bind": bind,
        "port": port,
        "pid": pid,
        "tls_enabled": false,
        "bind_source": "cli"
    })
}

async fn emit_insecure_bind_audit(store: &Arc<SessionStore>, bind: &str, port: u16) {
    let sink = AuditSink::new_with_tenant(Arc::clone(store), SYSTEM_AUDIT_TENANT_ID.to_string());
    let payload = insecure_bind_audit_payload(bind, port, std::process::id());
    if let Err(e) = sink
        .write(None, AuditKind::GatewayInsecureBind, payload, None)
        .await
    {
        tracing::warn!(target: "gateway::bind", error = %e, "gateway insecure bind audit write failed");
        return;
    }
    if let Err(e) = sink.flush_pending().await {
        tracing::warn!(target: "gateway::bind", error = %e, "gateway insecure bind audit flush failed");
    }
}

pub(super) async fn stop() -> ArgoResult<()> {
    let pidfile = pid_path()?;
    let raw = std::fs::read_to_string(&pidfile)
        .map_err(|e| ArgoError::Provider(format!("no PID file at {}: {e}", pidfile.display())))?;
    let pid: i32 = raw
        .trim()
        .parse()
        .map_err(|e| ArgoError::Schema(format!("invalid PID in {}: {e}", pidfile.display())))?;
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

#[cfg(test)]
#[path = "boot_bind_tests.rs"]
mod bind_tests;
