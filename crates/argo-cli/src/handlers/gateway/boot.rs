//! Gateway lifecycle commands — `start` + `stop` + the supporting
//! `pid_path` / `open_store` helpers. Extracted from `mod.rs` to keep
//! that file under the 500-LOC ceiling.

use crate::home::{cwd_workspace, db_path, ensure_dir, resolve_home};
use argo_store::SessionStore;
use argo_telemetry::span_context::{enter_context, LogContext};
use argo_types::{ArgoError, ArgoResult, TenantId};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

#[cfg(test)]
use super::boot_helpers::check_bind_loopback;
use super::boot_helpers::StartupPhases;
use super::boot_router::{self, ConnectionCtx};
use super::boot_spawns::{
    finish_shutdown, spawn_alerts_loop, spawn_approval_reminder, spawn_audit_watchdog,
    spawn_cu_screenshot_sweep, spawn_gateway_duty_scheduler, spawn_siem_from_config,
    spawn_worker_supervisor,
};
use super::ip_allowlist::IpAllowlist;
use super::lifecycle::{pid_is_alive, signal_term, PidGuard};
use super::rate_limit_ip::IpRateLimiter;
use super::routes_openmetrics;
use super::shutdown::{self, ShutdownController};
use super::start_banner::print_banner;
use super::start_banner_config::load_layered;
use super::startup_check::{run_startup_checks, validate_bind_safety, STARTUP_CHECK_EXIT};
use super::{approval_expiry_sweeper, metrics, sweepers};
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
    super::boot_telemetry_admin::apply_cardinality_caps(&cfg.telemetry.prometheus.cardinality_cap);
    let gateway_cfg = cfg.gateway.clone();
    let behind_proxy = gateway_cfg.behind_proxy.clone();
    let request_audit_cfg = gateway_cfg.request_audit.clone();
    let prebound_store = if bind_non_loopback && !boot_router::is_loopback_bind(bind) {
        match open_store(&cwd_for_check).await {
            Ok(s) => {
                let store = Arc::new(s);
                boot_router::emit_insecure_bind_audit(&store, bind, port).await;
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

    // claude-pricing-tier-enforcement-runtime — warm the tier LRU
    // cache (60s TTL) before the listener accepts so the first
    // tier-aware quota lookup doesn't pay an extra OnceLock init.
    super::quota_middleware::init_tier_cache();
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
    // At-rest encryption advisory. v1 stock builds ship the pure-Rust
    // `sqlx` SQLite driver, which is not linked against SQLCipher —
    // `PRAGMA key` is silently dropped and pages are written
    // plaintext. Warn on every boot until the operator either ships a
    // SQLCipher-linked build or sets
    // `[security.encryption] acknowledge_plaintext = true`. See
    // `docs/skycot/security/at-rest-encryption.md`.
    let enc = &cfg.security.encryption;
    if enc.key_source.eq_ignore_ascii_case("none") && !enc.acknowledge_plaintext {
        eprintln!(
            "argo gateway: ⚠️  at-rest encryption OFF (plaintext pages). \
             v1 stock build does not link SQLCipher. Set \
             `[security.encryption] acknowledge_plaintext = true` to \
             suppress; see docs/skycot/security/at-rest-encryption.md."
        );
    }
    let maintenance_cfg = super::maintenance_middleware::from_env();
    if maintenance_cfg.is_some() {
        eprintln!("argo gateway: MAINTENANCE MODE — non-/health requests return 503");
    }
    let mut served = 0_u64;
    let shutdown_ctl = ShutdownController::new();
    let mut shutdown_rx = shutdown::install_signal_handler(shutdown_ctl.clone());
    let mut watchdog: Option<JoinHandle<()>> = None;
    let ctx = ConnectionCtx {
        behind_proxy: &behind_proxy,
        request_audit_cfg: &request_audit_cfg,
        security_headers_cfg: &gateway_cfg.security_headers,
        ip_limiter: &ip_limiter,
        ip_allowlist: &ip_allowlist,
        store: store.as_ref(),
        shutdown_ctl: &shutdown_ctl,
    };
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
                if !super::maintenance_middleware::try_serve(&mut stream, maintenance_cfg.as_ref()).await {
                    boot_router::handle_connection(&mut stream, peer, &ctx).await;
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
