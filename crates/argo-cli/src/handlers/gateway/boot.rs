//! Gateway lifecycle commands — `start` + `stop` + the supporting
//! `pid_path` / `open_store` helpers. Extracted from `mod.rs` to keep
//! that file under the 500-LOC ceiling.

use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::SessionStore;
use argo_telemetry::span_context::{enter_context, LogContext};
use argo_types::{ArgoError, ArgoResult};
use std::path::PathBuf;
use tokio::task::JoinHandle;

#[cfg(test)]
use super::boot_helpers::check_bind_loopback;
use super::boot_router::ConnectionCtx;
use super::lifecycle::{pid_is_alive, signal_term};
use super::shutdown::ShutdownController;

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

pub(super) async fn open_store(cwd: &std::path::Path) -> ArgoResult<SessionStore> {
    let home = resolve_home(cwd)?;
    ensure_dir(&home)?;
    let db = db_path(&home);
    let store = open_store_at(&db).await?;
    store.migrate().await?;
    Ok(store)
}

/// Resolve the workspace SQLCipher key via
/// `argo_config::cred::db_key::get_or_create_db_key` and open the
/// session store with `PRAGMA key` applied. This is the
/// encrypted-by-default flip (`mission-sqlcipher-4-default-flip`):
/// every gateway boot path now goes through it instead of the legacy
/// plaintext `SessionStore::open`. Tenancy-aware deployments still
/// resolve per-tenant keys through `MultiTenantStore`; the gateway's
/// single-deployment default pool uses the workspace-default tenant
/// `Uuid::nil()` so the keyring entry is stable across boots.
async fn open_store_at(path: &std::path::Path) -> ArgoResult<SessionStore> {
    use argo_types::TenantId;
    use secrecy::ExposeSecret;
    let tenant = TenantId(uuid::Uuid::nil());
    let secret = argo_config::cred::db_key::get_or_create_db_key(tenant)
        .await
        .map_err(|e| {
            ArgoError::Provider(format!(
                "gateway store: derive SQLCipher key for workspace-default tenant: {e}. \
                 Set SKYCOT_DB_KEY_BASE64 to a base64-encoded 32-byte secret, or run \
                 with an OS keyring available."
            ))
        })?;
    let hex_key = argo_store::encryption::base64_to_hex(secret.expose_secret())?;
    SessionStore::open_with_hex_key(
        path,
        argo_store::wal_tuning::WalTuningProfile::default(),
        &hex_key,
    )
    .await
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
    if super::routes_auth_production_mode::is_production_mode_for_bind(bind)
        && super::routes_auth::production_bearer_missing_for_boot()
    {
        let msg = super::routes_auth::production_bearer_missing_message();
        tracing::error!("{msg}");
        return Err(ArgoError::Schema(msg));
    }
    enforce_enterprise_encryption_at_boot(bind)?;
    let _ = super::bootstrap_token::ensure_bootstrap_token()?;
    // Boot-time context: tenant_id resolves per-request inside the
    // route handlers — this entry covers everything that emits before
    // a request is parsed (banner, startup checks, bg spawns) with
    // the anon tenant marker.
    let _boot_ctx = enter_context(LogContext::default());
    let mut boot =
        super::boot_serve::init_boot(bind, port, skip_startup_check, bind_non_loopback).await?;
    let _bg_guards =
        super::boot_serve::spawn_bg_tasks(boot.store.as_ref(), &boot.cfg, &boot.cwd_for_store);
    boot.phases.mark("spawn_bg_tasks");
    let (_pid_guard, pid) = super::boot_serve::setup_pid_file()?;
    boot.phases.mark("pidfile");
    super::boot_serve::print_serve_banner(
        boot.store.as_ref(),
        &boot.cwd_for_store,
        cli_quiet,
        &boot.addr,
        pid,
        &mut boot.phases,
    )
    .await;
    if profile_startup {
        boot.phases.emit();
        eprintln!("argo gateway: --profile-startup set, exiting before serve loop");
        return Ok(());
    }
    let gates = super::boot_serve::setup_routing_gates(&boot.cfg, &boot.gateway_cfg)?;
    let shutdown_ctl = ShutdownController::new();
    let ctx = ConnectionCtx {
        behind_proxy: &boot.gateway_cfg.behind_proxy,
        request_audit_cfg: &boot.gateway_cfg.request_audit,
        security_headers_cfg: &boot.gateway_cfg.security_headers,
        ip_limiter: &gates.ip_limiter,
        ip_allowlist: &gates.ip_allowlist,
        store: boot.store.as_ref(),
        shutdown_ctl: &shutdown_ctl,
    };
    super::boot_serve::run_accept_loop(
        boot.listener,
        ctx,
        max_requests,
        &shutdown_ctl,
        gates.maintenance_cfg.as_ref(),
        boot.store.as_ref(),
    )
    .await
}

/// Production-mode at-rest-encryption gate
/// (`mission-audit-sqlcipher-enterprise-default`). Loads the effective
/// `argo.toml` + env overrides and routes the resulting
/// `security.encryption.key_source` through
/// [`argo_store::store_open::enforce_enterprise_encryption`]. Mirrors
/// the bearer-token guard above: dev mode is unchanged, production
/// mode fails closed unless the binary is built with SQLCipher and a
/// real key source is configured.
///
/// "Real key source" means either:
/// - `[security.encryption] key_source` is set to something other
///   than `"none"` (the argo.toml-driven path), OR
/// - `SKYCOT_DB_KEY_BASE64` is set (the env-var path the gateway
///   currently uses by default — see `open_store_at` above).
///
/// Either route resolves a real per-deployment key, so either is a
/// valid satisfaction signal. The brief originally framed the check
/// as `config.security.key_source != "none"`; treating the env-var
/// route as equivalent is a sensible deviation that matches the
/// gateway's actual code path.
fn enforce_enterprise_encryption_at_boot(bind: &str) -> ArgoResult<()> {
    let production = super::routes_auth_production_mode::is_production_mode_for_bind(bind);
    if !production {
        return Ok(());
    }
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let mut cfg = argo_config::ArgoConfig::load(&cwd.join("argo.toml")).unwrap_or_default();
    if let Ok(env_cfg) = argo_config::ArgoConfig::from_env() {
        cfg.merge(env_cfg);
    }
    let key_source = effective_key_source(&cfg.security.encryption.key_source);
    if let Err(e) = argo_store::store_open::enforce_enterprise_encryption(production, &key_source) {
        tracing::error!("{e}");
        return Err(e);
    }
    Ok(())
}

/// Substitute a synthetic key-source token when `SKYCOT_DB_KEY_BASE64`
/// is set so the production-mode gate honors the env-var path. The
/// helper is kept module-local — the store-crate gate stays a pure
/// `(production, key_source)` predicate.
fn effective_key_source(configured: &str) -> String {
    let trimmed = configured.trim();
    let configured_is_none = trimmed.is_empty() || trimmed.eq_ignore_ascii_case("none");
    if configured_is_none {
        if let Ok(raw) = std::env::var(argo_store::store_open::DB_KEY_ENV) {
            if !raw.trim().is_empty() {
                return format!("env:{}", argo_store::store_open::DB_KEY_ENV);
            }
        }
    }
    configured.to_string()
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
