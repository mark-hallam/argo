//! Sequential pre-bind startup health check
//! (`claude-startup-health-check`).
//!
//! The gateway used to bind its TCP listener before opening the
//! session store. That meant pod orchestrators received a healthy
//! TCP `accept()` while the database / migrations / provider-key
//! configuration were still uninspected — the first inbound request
//! would discover the misconfiguration, by which time traffic was
//! already routed at the failing pod.
//!
//! [`run_startup_checks`] runs four synchronous checks before
//! `TcpListener::bind` and refuses the bind on any critical failure,
//! exiting with code [`STARTUP_CHECK_EXIT`] (BSD `sysexits.h`
//! `EX_CONFIG = 78`). Failures are surfaced as `argo gateway` stderr
//! plus the orchestrator's standard misconfig handling.
//!
//! Sibling tests live in `health_check_tests.rs`.

use crate::home::{db_path, ensure_dir, resolve_home};
use argo_channels::{AnomalyAlert, AnomalyDispatch, AnomalyKind, MultiChannelDispatch, Severity};
use argo_config::ArgoConfig;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

/// Exit code for critical startup-check failure. Matches the BSD
/// `sysexits.h` `EX_CONFIG` convention so orchestrators (k8s,
/// systemd) treat the pod as misconfigured rather than crash-looping
/// indefinitely.
pub(super) const STARTUP_CHECK_EXIT: i32 = 78;

/// Sequential, sync (per-check) startup health gate. Runs four
/// critical-path checks before the gateway binds its TCP listener:
///
///   1. **Store reachable** — open the SQLite store and run a
///      trivial `SELECT 1` to confirm the file is accessible and
///      the pool can issue queries.
///   2. **Schema version** — compare on-disk `schema_version` to
///      [`argo_store::EXPECTED_SCHEMA_VERSION`]. A fresh database
///      (`schema_version = 0`) is treated as a legitimate new
///      install and passes through; the existing `migrate()` step
///      inside `boot::open_store` then brings it up. Any other
///      mismatch is critical and refuses the bind with a
///      directional error message.
///   3. **Provider keys** — at least one of `ANTHROPIC_API_KEY` or
///      `OPENAI_API_KEY` must be set. Without a model provider the
///      gateway has nothing to dispatch to.
///   4. **Channels (warn-only)** — list the configured adapters
///      from `[gateway].adapters` and log them. We deliberately do
///      not refuse on channel failure: misconfigured channels are
///      common during initial setup and should not block the
///      gateway's HTTP surface.
///
/// Returns `Err` on any critical failure. Caller maps that to
/// `exit(78)` before the listener binds.
pub(super) async fn run_startup_checks(skip: bool, cwd: &Path) -> ArgoResult<()> {
    let dispatch = anomaly_dispatch_from_cwd(cwd);
    match run_startup_checks_inner(skip, cwd).await {
        Ok(()) => Ok(()),
        Err(e) => {
            dispatch_gateway_boot_fail(&dispatch, cwd, &e).await;
            Err(e)
        }
    }
}

async fn run_startup_checks_inner(skip: bool, cwd: &Path) -> ArgoResult<()> {
    // demo-ops-add-magic-link-bypass-for-recording: refuse to bind
    // when ARGO_DEMO_MODE=1 is set in an environment that still has
    // bearer-auth enforcement on. Honored even with --skip-startup-check
    // so an operator cannot accidentally ship a prod gateway with the
    // demo signup-bypass route exposed.
    demo_mode_compatibility_check()?;
    if skip {
        eprintln!("argo gateway: --skip-startup-check set; bypassing critical health checks");
        return Ok(());
    }
    let started = Instant::now();
    let home = resolve_home(cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home))
        .await
        .map_err(|e| ArgoError::Provider(format!("startup check: store unreachable: {e}")))?;
    sqlx::query("SELECT 1")
        .fetch_one(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("startup check: SELECT 1: {e}")))?;
    let actual = store.schema_version().await?;
    if actual == 0 {
        eprintln!(
            "argo gateway: startup_check: schema_version=0 (fresh install — gateway will migrate)"
        );
    } else {
        store
            .verify_schema_version()
            .await
            .map_err(|e| ArgoError::Provider(format!("startup check: {e}")))?;
    }
    store.close().await;
    if !provider_key_present() {
        return Err(ArgoError::Provider(
            "startup check: no provider API key set \
             (need ANTHROPIC_API_KEY or OPENAI_API_KEY)"
                .into(),
        ));
    }
    warn_unprobed_channels(cwd);
    eprintln!(
        "argo gateway: startup_check ok in {} ms",
        started.elapsed().as_millis()
    );
    Ok(())
}

pub(super) fn anomaly_dispatch_from_cwd(cwd: &Path) -> Arc<dyn AnomalyDispatch> {
    let cfg = ArgoConfig::load(&cwd.join("argo.toml")).unwrap_or_default();
    MultiChannelDispatch::from_config(&cfg.runtime.anomaly_alerting, BTreeMap::new())
}

async fn dispatch_gateway_boot_fail(
    dispatch: &Arc<dyn AnomalyDispatch>,
    cwd: &Path,
    err: &ArgoError,
) {
    let mut context = BTreeMap::new();
    context.insert("cwd".into(), cwd.display().to_string());
    context.insert("error".into(), err.to_string());
    let alert = AnomalyAlert {
        kind: AnomalyKind::GatewayBootFail,
        severity: Severity::Critical,
        message: format!("gateway startup check failed: {err}"),
        context,
    };
    if let Err(e) = dispatch.dispatch(alert).await {
        tracing::warn!(target: "gateway::startup_check", error = %e, "gateway boot-fail anomaly dispatch failed");
    }
}

pub(super) fn provider_key_present() -> bool {
    let any = |k: &str| std::env::var(k).is_ok_and(|v| !v.is_empty());
    any("ANTHROPIC_API_KEY") || any("OPENAI_API_KEY")
}

/// Refuse to bind when `ARGO_DEMO_MODE=1` is set in an environment
/// where bearer-auth would otherwise be enforced. Demo-mode bypasses
/// the magic-link verify step entirely, so it MUST co-exist with
/// `ARGO_GATEWAY_AUTH_BYPASS=1` (the dev-only escape hatch that
/// disables the bearer gate). The pair is intentional: if you have
/// bearer-auth on, demo-mode is not safe to enable.
fn demo_mode_compatibility_check() -> ArgoResult<()> {
    let demo = std::env::var("ARGO_DEMO_MODE").is_ok_and(|v| v == "1");
    if !demo {
        return Ok(());
    }
    let bypass = std::env::var("ARGO_GATEWAY_AUTH_BYPASS").is_ok_and(|v| v == "1");
    if !bypass {
        return Err(ArgoError::Provider(
            "ARGO_DEMO_MODE=1 cannot be used in production; \
             set ARGO_GATEWAY_AUTH_BYPASS=1 or unset ARGO_DEMO_MODE"
                .into(),
        ));
    }
    eprintln!(
        "argo gateway: ARGO_DEMO_MODE=1 active — POST /api/v1/demo/mint-principal is mounted; \
         every mint emits a warn log + audit row with demo_mode_bypass=true"
    );
    Ok(())
}

/// Load-bearing bind-safety guard
/// (`mission-gateway-tls-termination-or-proxy-doc`). Runs BEFORE the
/// async startup checks and BEFORE the TCP `bind()`, and is honored
/// even when `--skip-startup-check` is set, so the
/// `argo gateway start --bind 0.0.0.0:7400` failure mode (plaintext
/// HTTP served to the world) cannot be skipped by accident.
///
/// Delegates to [`super::proxy_guard::validate_bind_address`], which
/// loads the `[gateway.behind_proxy]` contract from `argo.toml`. See
/// the module docs there for the decision matrix.
pub(super) fn validate_bind_safety(bind: &str, insecure_bind: bool, cwd: &Path) -> ArgoResult<()> {
    let cfg = ArgoConfig::load(&cwd.join("argo.toml")).unwrap_or_default();
    super::proxy_guard::validate_bind_address(bind, &cfg.gateway.behind_proxy, insecure_bind)
}

fn warn_unprobed_channels(cwd: &Path) {
    let cfg = ArgoConfig::load(&cwd.join("argo.toml")).unwrap_or_default();
    if !cfg.gateway.adapters.is_empty() {
        let unique = cfg
            .gateway
            .adapters
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        eprintln!(
            "argo gateway: channels probe warn-only ({} adapter(s): {})",
            unique.len(),
            unique.join(", ")
        );
    }
}

#[cfg(test)]
#[path = "health_check_tests.rs"]
mod tests;
