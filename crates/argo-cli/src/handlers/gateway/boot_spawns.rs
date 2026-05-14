//! Background-task spawn helpers used by `boot::start`. Each helper
//! returns an [`AbortOnDrop`] (defined in `boot.rs`) so the start
//! routine can hold the handle for the lifetime of the gateway
//! process — Drop aborts the underlying tokio task on graceful
//! shutdown. Extracted from `boot.rs` (claude-gateway-boot-split) to
//! free headroom under the 500-LOC ceiling for the queued mission
//! wire-ups that touch gateway startup.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use argo_channels::{AnomalyAlert, AnomalyDispatch, AnomalyKind, Severity};
use argo_config::SiemConfig;
use argo_orchestrator::{
    spawn_duty_scheduler, ClosedGate, DutyScheduler, DutyStore, OrchestratorStore, WorkerSupervisor,
};
use argo_siem::{
    spawn_siem_dispatcher, DatadogLogsSink, ElasticSink, SiemDispatcherConfig, SiemSink,
    SplunkHecSink,
};
use argo_store::attached_blob::{BlobStore, MemoryAttachedBlob};
use argo_store::SessionStore;
use tokio::task::JoinHandle;

use super::approval_reminder;
use super::boot::AbortOnDrop;
use super::shutdown::{self, ShutdownController};
use crate::handlers::alerts;
use crate::handlers::cu_screenshots::CuScreenshotStore;
use std::collections::BTreeMap;

const DEFAULT_DUTY_SCHEDULER_SECS: u64 = 60;
/// Sweep cadence for the `cu_screenshot` retention task. Hourly is
/// the brief's target — short enough to keep the index lean,
/// long enough that the sweep cost is negligible against the
/// `(retention_until)` index.
const CU_SCREENSHOT_SWEEP_SECS: u64 = 3_600;
/// Default retention horizon for CU screenshots: 14 days. Operator
/// override via `ARGO_CU_SCREENSHOT_RETENTION_DAYS`.
const DEFAULT_CU_SCREENSHOT_RETENTION_DAYS: u64 = 14;

pub(super) fn spawn_audit_watchdog(store: Arc<SessionStore>) -> AbortOnDrop {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let dispatch = super::startup_check::anomaly_dispatch_from_cwd(&cwd);
    spawn_audit_watchdog_with_dispatch(store, dispatch)
}

pub(super) fn spawn_audit_watchdog_with_dispatch(
    store: Arc<SessionStore>,
    dispatch: Arc<dyn AnomalyDispatch>,
) -> AbortOnDrop {
    let cb: Arc<dyn Fn(argo_audit::ChainBreak) + Send + Sync> = Arc::new(move |b| {
        tracing::error!(
            target: "gateway::audit_watchdog",
            seq = b.seq,
            kind = %b.kind,
            reason = %b.reason,
            "audit chain break detected"
        );
        let dispatch = Arc::clone(&dispatch);
        tokio::spawn(async move {
            let mut context = BTreeMap::new();
            context.insert("seq".into(), b.seq.to_string());
            context.insert("kind".into(), b.kind);
            context.insert("reason".into(), b.reason);
            let alert = AnomalyAlert {
                kind: AnomalyKind::ChainBreak,
                severity: Severity::Critical,
                message: "audit chain break detected".into(),
                context,
            };
            if let Err(e) = dispatch.dispatch(alert).await {
                tracing::warn!(target: "gateway::audit_watchdog", error = %e, "chain-break anomaly dispatch failed");
            }
        });
    });
    eprintln!("argo gateway: audit chain watchdog enabled");
    AbortOnDrop::new(argo_audit::spawn_watchdog(
        store,
        argo_audit::WatchdogConfig::default(),
        cb,
    ))
}

pub(super) fn spawn_alerts_loop(store: Arc<SessionStore>) -> AbortOnDrop {
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

pub(super) fn spawn_cu_screenshot_sweep(store: Arc<SessionStore>) -> AbortOnDrop {
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

pub(super) fn spawn_gateway_duty_scheduler(store: Arc<SessionStore>) -> AbortOnDrop {
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

/// Build the right [`SiemSink`] for the configured backend and spawn
/// the dispatcher. Returns `None` when `enabled = false` (default),
/// when the backend label is unrecognised, or when the sink fails to
/// construct (logged at boot, not fatal — the gateway still serves
/// without SIEM shipping).
pub(super) fn spawn_siem_from_config(
    store: Arc<SessionStore>,
    cfg: &SiemConfig,
) -> Option<AbortOnDrop> {
    if !cfg.enabled {
        return None;
    }
    let dispatcher_cfg = SiemDispatcherConfig {
        interval: if cfg.interval_secs == 0 {
            Duration::from_secs(argo_siem::dispatcher::DEFAULT_INTERVAL_SECS)
        } else {
            Duration::from_secs(cfg.interval_secs)
        },
    };
    let sink: Arc<dyn SiemSink> = match cfg.backend.as_str() {
        "splunk" => match SplunkHecSink::new(
            cfg.endpoint.clone(),
            cfg.token.clone(),
            if cfg.sourcetype.is_empty() {
                None
            } else {
                Some(cfg.sourcetype.clone())
            },
        ) {
            Ok(s) => Arc::new(s),
            Err(e) => {
                eprintln!("argo gateway: siem splunk sink build failed ({e}); dispatcher disabled");
                return None;
            }
        },
        "elastic" => match ElasticSink::new(
            cfg.endpoint.clone(),
            cfg.token.clone(),
            if cfg.index.is_empty() {
                None
            } else {
                Some(cfg.index.clone())
            },
        ) {
            Ok(s) => Arc::new(s),
            Err(e) => {
                eprintln!(
                    "argo gateway: siem elastic sink build failed ({e}); dispatcher disabled"
                );
                return None;
            }
        },
        "datadog" => match DatadogLogsSink::new(
            cfg.endpoint.clone(),
            cfg.token.clone(),
            None,
            if cfg.service.is_empty() {
                None
            } else {
                Some(cfg.service.clone())
            },
        ) {
            Ok(s) => Arc::new(s),
            Err(e) => {
                eprintln!(
                    "argo gateway: siem datadog sink build failed ({e}); dispatcher disabled"
                );
                return None;
            }
        },
        other => {
            eprintln!(
                "argo gateway: siem backend '{other}' not recognised \
                 (expected splunk|elastic|datadog); dispatcher disabled"
            );
            return None;
        }
    };
    eprintln!(
        "argo gateway: siem dispatcher enabled — backend={} endpoint={}",
        sink.backend(),
        sink.endpoint(),
    );
    Some(AbortOnDrop::new(spawn_siem_dispatcher(
        store,
        sink,
        dispatcher_cfg,
    )))
}

/// Construct the `WorkerSupervisor` for the gateway process. Held
/// as an `Arc` so the future worker-dispatch wiring can call
/// `spawn_worker` on submissions; gateway boot itself does not
/// spawn workers. The cage + OOM-restart wire is the
/// `mission-worker-supervisor-cage-and-oom` deliverable.
pub(super) fn spawn_worker_supervisor() -> Arc<WorkerSupervisor> {
    let supervisor = Arc::new(WorkerSupervisor::with_defaults());
    eprintln!("argo gateway: worker supervisor constructed (cage + oom-restart wired)");
    supervisor
}

pub(super) fn spawn_approval_reminder(store: Arc<SessionStore>) -> AbortOnDrop {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    eprintln!(
        "argo gateway: approval reminder enabled (interval={}s, window={}m)",
        approval_reminder::interval_secs_from_env(),
        approval_reminder::window_from_env().as_secs() / 60
    );
    AbortOnDrop::new(approval_reminder::spawn_approval_reminder_loop(store, cwd))
}

/// Drain in-flight requests up to `ARGO_SHUTDOWN_DEADLINE_SECS` (env,
/// default 30s), then close the session store so SQLite checkpoints
/// the WAL and releases the lock file. The force-exit watchdog (when
/// present) was armed inside the signal handler and started ticking
/// from SIGTERM time; we abort it after a clean drain+close so the
/// process exits 0 instead of 124.
pub(super) async fn finish_shutdown(
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
