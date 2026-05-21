//! Linear phases of [`super::boot::start`] split into named helpers
//! so the entry-point stays under the 80-LOC budget enforced by
//! `scripts/check-fn-loc.sh`. Each helper owns one concern (boot
//! initialization, background task spawn, PID file, banner emission,
//! routing gates, accept loop) and is called in order from `start`.
//! No behavior change versus the inline form.

use super::boot::{open_store, set_learning_registry, AbortOnDrop};
use super::boot_helpers::StartupPhases;
use super::boot_router::{self, ConnectionCtx};
use super::boot_spawns::{
    finish_shutdown, spawn_alerts_loop, spawn_approval_reminder, spawn_audit_watchdog,
    spawn_cu_screenshot_sweep, spawn_gateway_duty_scheduler, spawn_siem_from_config,
    spawn_worker_supervisor,
};
use super::ip_allowlist::IpAllowlist;
use super::lifecycle::{pid_is_alive, PidGuard};
use super::rate_limit_ip::IpRateLimiter;
use super::routes_openmetrics;
use super::shutdown::{self, ShutdownController};
use super::start_banner::print_banner;
use super::start_banner_config::load_layered;
use super::startup_check::{run_startup_checks, validate_bind_safety, STARTUP_CHECK_EXIT};
use super::{approval_expiry_sweeper, metrics, service_token_binding_sweeper, sweepers};
use argo_config::ArgoConfig;
use argo_forge::{spawn_forge_cron, ForgeCronConfig};
use argo_store::SessionStore;
use argo_trust_engine::{spawn_trust_engine_cron, TrustEngineConfig};
use argo_types::{ArgoError, ArgoResult, TenantId};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

use crate::handlers::mcp_health::HealthCheckGuard;
use crate::home::cwd_workspace;
use argo_orchestrator::WorkerSupervisor;

/// Bundle returned from [`init_boot`] so the caller in `start` can
/// continue without a 6-tuple.
pub(super) struct BootInit {
    pub cfg: ArgoConfig,
    pub listener: TcpListener,
    pub store: Option<Arc<SessionStore>>,
    pub gateway_cfg: argo_config::GatewayConfig,
    pub addr: String,
    pub cwd_for_store: PathBuf,
    pub phases: StartupPhases,
}

/// Bind-safety guard, startup checks, config load, telemetry init,
/// optional insecure-bind pre-audit, TCP listener bind, and store
/// open. Mirrors the inline flow that used to live at the head of
/// `start`. Returns the resolved state the caller threads through
/// the remaining phases.
pub(super) async fn init_boot(
    bind: &str,
    port: u16,
    skip_startup_check: bool,
    bind_non_loopback: bool,
) -> ArgoResult<BootInit> {
    let mut phases = StartupPhases::new();
    let cwd_for_check =
        std::env::current_dir().map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
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
    let prebound = preaudit_insecure_bind(bind, port, bind_non_loopback, &cwd_for_check).await;
    let addr = format!("{bind}:{port}");
    let listener = TcpListener::bind(&addr)
        .await
        .map_err(|e| ArgoError::Provider(format!("bind {addr}: {e}")))?;
    phases.mark("tcp_bind");
    let cwd_for_store =
        std::env::current_dir().map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let store = open_serving_store(prebound, &cwd_for_store).await;
    phases.mark("store_open_migrate");
    if let Some(s) = store.as_ref() {
        super::hipaa_startup_check::emit_startup_check_best_effort(&cwd_for_store, s).await;
    }
    Ok(BootInit {
        cfg,
        listener,
        store,
        gateway_cfg,
        addr,
        cwd_for_store,
        phases,
    })
}

async fn preaudit_insecure_bind(
    bind: &str,
    port: u16,
    bind_non_loopback: bool,
    cwd: &std::path::Path,
) -> Option<Arc<SessionStore>> {
    if !bind_non_loopback || boot_router::is_loopback_bind(bind) {
        return None;
    }
    match open_store(cwd).await {
        Ok(s) => {
            let store = Arc::new(s);
            boot_router::emit_insecure_bind_audit(&store, bind, port).await;
            Some(store)
        }
        Err(e) => {
            eprintln!(
                "argo gateway: bind-non-loopback audit unavailable before bind ({e}); continuing"
            );
            None
        }
    }
}

async fn open_serving_store(
    prebound: Option<Arc<SessionStore>>,
    cwd: &std::path::Path,
) -> Option<Arc<SessionStore>> {
    if let Some(s) = prebound {
        return Some(s);
    }
    match open_store(cwd).await {
        Ok(s) => Some(Arc::new(s)),
        Err(e) => {
            eprintln!("argo gateway: store open failed ({e}); /sessions etc. will return 503");
            None
        }
    }
}

/// Bag of background-task guards that `start` keeps alive until the
/// accept loop exits. Each spawn function's return type determines the
/// field type — the `Option<Option<...>>` flatten via `and_then`
/// matches the inline form's `let _x = bg_store.map(...)` shape.
pub(super) struct BgGuards {
    _sweeper: Option<JoinHandle<()>>,
    _approval_expiry: Option<JoinHandle<()>>,
    _service_token_binding_sweeper: Option<JoinHandle<()>>,
    _approval_reminder: Option<AbortOnDrop>,
    _duty_scheduler: Option<AbortOnDrop>,
    _alerts_loop: Option<AbortOnDrop>,
    _cu_screenshot_sweep: Option<AbortOnDrop>,
    _audit_watchdog: Option<AbortOnDrop>,
    _forge_cron: Option<AbortOnDrop>,
    _trust_engine_cron: Option<AbortOnDrop>,
    _skill_learning_registry: Option<Arc<argo_skill::WorkerLearningRegistry>>,
    _siem_dispatcher: Option<AbortOnDrop>,
    _mcp_health: Option<HealthCheckGuard>,
    _worker_supervisor: Arc<WorkerSupervisor>,
}

pub(super) fn spawn_bg_tasks(
    bg_store: Option<&Arc<SessionStore>>,
    cfg: &ArgoConfig,
    cwd_for_store: &std::path::Path,
) -> BgGuards {
    BgGuards {
        _sweeper: bg_store.and_then(|s| sweepers::spawn_handoff_sweeper(Arc::clone(s))),
        _approval_expiry: bg_store
            .and_then(|s| approval_expiry_sweeper::spawn_approval_expiry_sweeper(Arc::clone(s))),
        _service_token_binding_sweeper: bg_store.and_then(|s| {
            service_token_binding_sweeper::spawn_service_token_binding_sweeper(Arc::clone(s))
        }),
        _approval_reminder: bg_store.map(|s| spawn_approval_reminder(Arc::clone(s))),
        _duty_scheduler: bg_store.map(|s| spawn_gateway_duty_scheduler(Arc::clone(s))),
        _alerts_loop: bg_store.map(|s| spawn_alerts_loop(Arc::clone(s))),
        _cu_screenshot_sweep: bg_store.map(|s| spawn_cu_screenshot_sweep(Arc::clone(s))),
        _audit_watchdog: bg_store.map(|s| spawn_audit_watchdog(Arc::clone(s))),
        _forge_cron: bg_store
            .map(|s| AbortOnDrop::new(spawn_forge_cron(Arc::clone(s), ForgeCronConfig::from_env()))),
        _trust_engine_cron: bg_store.map(|s| {
            AbortOnDrop::new(spawn_trust_engine_cron(
                Arc::clone(s),
                TrustEngineConfig::from_env(),
            ))
        }),
        _skill_learning_registry: bg_store.and_then(|s| spawn_skill_learning(s, cfg)),
        _siem_dispatcher: bg_store.and_then(|s| {
            let cfg = ArgoConfig::load(&cwd_for_store.join("argo.toml")).unwrap_or_default();
            spawn_siem_from_config(Arc::clone(s), &cfg.siem)
        }),
        _mcp_health: crate::handlers::mcp_health::spawn_from_env(),
        _worker_supervisor: spawn_worker_supervisor(),
    }
}

/// Boot-time entry point for the per-worker skill-learning cron. When
/// `[skill_learning].enabled = true` (default), constructs a
/// [`argo_skill::WorkerLearningRegistry`], installs it as the
/// process-global so per-request worker-hire handlers can enroll
/// post-boot workers, then asynchronously enumerates existing
/// (non-archived) workers and enrolls each one through the registry.
/// Returns the registry for `BgGuards` to retain — dropping it on
/// shutdown aborts every per-worker tokio task it spawned.
///
/// Pre-`mission-audit-skill-learning-dynamic-enroll`, this function
/// spawned crons inline through `argo_skill::spawn_per_worker_learning_cron`
/// and held the handles in a single `AbortOnDrop`. Post-brief, the
/// registry is the single source of truth for "which workers have a
/// learning handle" and is shared between the boot enumerator and
/// the hire HTTP path.
fn spawn_skill_learning(
    s: &Arc<SessionStore>,
    cfg: &ArgoConfig,
) -> Option<Arc<argo_skill::WorkerLearningRegistry>> {
    let learning_cfg = cfg.skill_learning.clone();
    if !learning_cfg.enabled {
        eprintln!("argo gateway: skill learning cron disabled by config");
        return None;
    }
    let tenant_filter = match learning_cfg
        .tenant_filter
        .as_deref()
        .map(uuid::Uuid::parse_str)
        .transpose()
    {
        Ok(v) => v.map(TenantId),
        Err(e) => {
            eprintln!("argo gateway: invalid [skill_learning].tenant_filter ({e}); skipping cron");
            return None;
        }
    };
    let skills = Arc::new(argo_skill::SkillStore::new(Arc::clone(s)));
    let interval = Duration::from_secs(learning_cfg.interval_secs.max(1));
    let registry = Arc::new(argo_skill::WorkerLearningRegistry::new(
        skills,
        argo_skill::PerWorkerLearningConfig::default(),
        interval,
    ));
    set_learning_registry(Arc::clone(&registry));
    tokio::spawn(enroll_boot_workers(
        Arc::clone(s),
        Arc::clone(&registry),
        tenant_filter,
        interval,
    ));
    Some(registry)
}

async fn enroll_boot_workers(
    store: Arc<SessionStore>,
    registry: Arc<argo_skill::WorkerLearningRegistry>,
    tenant_filter: Option<TenantId>,
    interval: Duration,
) {
    let rows = match tenant_filter {
        Some(t) => {
            sqlx::query_as::<_, (String, String)>(
                "SELECT id, tenant_id FROM worker WHERE tenant_id = ? AND archived_at IS NULL",
            )
            .bind(t.to_string())
            .fetch_all(store.pool())
            .await
        }
        None => {
            sqlx::query_as::<_, (String, String)>(
                "SELECT id, tenant_id FROM worker WHERE archived_at IS NULL",
            )
            .fetch_all(store.pool())
            .await
        }
    };
    let rows = match rows {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(
                target: "gateway::skill_learning",
                error = %e,
                "list workers failed; boot enrollment skipped",
            );
            return;
        }
    };
    let mut enrolled = 0_usize;
    for (worker_raw, tenant_raw) in rows {
        let (Ok(worker), Ok(tenant)) = (
            worker_raw.parse::<argo_store::WorkerId>(),
            uuid::Uuid::parse_str(&tenant_raw),
        ) else {
            continue;
        };
        if registry.enroll(worker, TenantId(tenant)) {
            enrolled += 1;
        }
    }
    eprintln!(
        "argo gateway: skill learning cron enabled (workers={enrolled}, interval={}s)",
        interval.as_secs()
    );
}

/// PID file management: refuse to start if a live process already
/// holds the path; overwrite stale entries; return a `PidGuard` that
/// removes the file on drop.
pub(super) fn setup_pid_file() -> ArgoResult<(PidGuard, u32)> {
    let pidfile = super::boot::pid_path()?;
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
    Ok((PidGuard(pidfile), pid))
}

/// Emit pre-serve metrics + workspace/banner stdout lines. Marks the
/// `workspace_lookup` and `banner` startup phases.
pub(super) async fn print_serve_banner(
    store: Option<&Arc<SessionStore>>,
    cwd_for_store: &std::path::Path,
    cli_quiet: bool,
    addr: &str,
    pid: u32,
    phases: &mut StartupPhases,
) {
    super::quota_middleware::init_tier_cache();
    metrics::init_boot_time();
    routes_openmetrics::ensure_metrics_registered();
    routes_openmetrics::brief_store_migration_chain_version()
        .set(i64::from(argo_store::EXPECTED_SCHEMA_VERSION));
    eprintln!("argo gateway: listening on {addr} (pid {pid})");
    if let Some(s) = store {
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
}

/// IP rate-limiter, source-CIDR allowlist, and maintenance-mode
/// config resolved before the accept loop starts.
pub(super) struct RoutingGates {
    pub ip_limiter: Arc<IpRateLimiter>,
    pub ip_allowlist: Arc<IpAllowlist>,
    pub maintenance_cfg: Option<super::maintenance_middleware::MaintenanceCfg>,
}

pub(super) fn setup_routing_gates(
    cfg: &ArgoConfig,
    gateway_cfg: &argo_config::GatewayConfig,
) -> ArgoResult<RoutingGates> {
    let ip_limiter = Arc::new(IpRateLimiter::from_config(&gateway_cfg.rate_limit.ip));
    let ip_allowlist = match IpAllowlist::from_config(&cfg.security.ip_allowlist) {
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
    let maintenance_cfg = super::maintenance_middleware::from_env();
    if maintenance_cfg.is_some() {
        eprintln!("argo gateway: MAINTENANCE MODE — non-/health requests return 503");
    }
    Ok(RoutingGates {
        ip_limiter,
        ip_allowlist,
        maintenance_cfg,
    })
}

/// Tokio accept loop with shutdown bias + per-connection
/// maintenance-mode short-circuit + `--max-requests` budget.
pub(super) async fn run_accept_loop(
    listener: TcpListener,
    ctx: ConnectionCtx<'_>,
    max_requests: Option<u64>,
    shutdown_ctl: &ShutdownController,
    maintenance_cfg: Option<&super::maintenance_middleware::MaintenanceCfg>,
    store: Option<&Arc<SessionStore>>,
) -> ArgoResult<()> {
    let mut shutdown_rx = shutdown::install_signal_handler(shutdown_ctl.clone());
    let mut watchdog: Option<JoinHandle<()>> = None;
    let mut served = 0_u64;
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
                if !super::maintenance_middleware::try_serve(&mut stream, maintenance_cfg).await {
                    boot_router::handle_connection(&mut stream, peer, &ctx).await;
                }
                served += 1;
                if max_requests.is_some_and(|cap| served >= cap) {
                    eprintln!("argo gateway: served {served} request(s); exiting per --max-requests");
                    break;
                }
            }
        }
    }
    finish_shutdown(shutdown_ctl, store, watchdog).await;
    Ok(())
}
