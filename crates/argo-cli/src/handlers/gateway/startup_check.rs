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
use std::time::{Duration, Instant};

/// Exit code for critical startup-check failure. Matches the BSD
/// `sysexits.h` `EX_CONFIG` convention so orchestrators (k8s,
/// systemd) treat the pod as misconfigured rather than crash-looping
/// indefinitely.
pub(super) const STARTUP_CHECK_EXIT: i32 = 78;

/// Bounded-retry budget for the store-reachability probe (see
/// [`open_store_for_probe_with_retry`]). Attempts, initial backoff, and
/// backoff ceiling. The worst-case wait before failing closed is
/// roughly `250 + 500 + 1000 + 1000 = 2750 ms` — long enough to absorb
/// a brand-new SQLCipher volume's first-init transient, short enough
/// that a genuinely-unreachable store still surfaces as `exit 78`
/// within a few seconds.
const STORE_PROBE_ATTEMPTS: u32 = 5;
const STORE_PROBE_BACKOFF_MS: u64 = 250;
const STORE_PROBE_BACKOFF_CAP_MS: u64 = 1000;

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
pub(super) async fn run_startup_checks(skip: bool, bind: &str, cwd: &Path) -> ArgoResult<()> {
    let dispatch = anomaly_dispatch_from_cwd(cwd);
    match run_startup_checks_inner(skip, bind, cwd).await {
        Ok(()) => Ok(()),
        Err(e) => {
            dispatch_gateway_boot_fail(&dispatch, cwd, &e).await;
            Err(e)
        }
    }
}

async fn run_startup_checks_inner(skip: bool, bind: &str, cwd: &Path) -> ArgoResult<()> {
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
    // sprint-ws1-1c-jwt-secret-fail-closed-boot-guard: refuse to bind
    // in production mode when the session-cookie HS256 secret is
    // unset / empty / too-short. Runs FIRST among the critical checks
    // (before any store/provider work) so a misconfig surfaces cheaply.
    jwt_secret_boot_guard(bind, cwd)?;
    // fix-mf1-audit-export-secret-fail-closed: refuse to bind in
    // production mode when either audit-signing secret is unset / empty.
    // Without these the signed audit-export download + screenshot URLs
    // fall back to a hardcoded, publicly-known dev secret — making the
    // links forgeable for ANY tenant. Same production gate + escape
    // hatches as the JWT-secret guard above.
    audit_signing_secret_boot_guard(bind, cwd)?;
    let started = Instant::now();
    let home = resolve_home(cwd)?;
    ensure_dir(&home)?;
    let store = open_store_for_probe_with_retry(&db_path(&home)).await?;
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
    warn_public_base_url_unset(bind);
    eprintln!(
        "argo gateway: startup_check ok in {} ms",
        started.elapsed().as_millis()
    );
    Ok(())
}

/// Bounded-retry open of the session store for the startup
/// reachability probe.
///
/// **Why this exists** (`fix-deploy-smoke-gateway-exit78-fresh-boot-race`):
/// on a brand-new encrypted (SQLCipher) `/data` volume the very first
/// `SessionStore::open` + `SELECT 1` occasionally fails while the fresh
/// cipher header settles. The deploy-smoke first-green run saw the
/// gateway `exit 78` on ~50% of FIRST fresh-volume `docker compose up`
/// cycles, then boot Healthy on the NEXT container start — i.e. the
/// exact same open succeeds once the file has been initialized. (The
/// store is the only boot check that touches the fresh volume: nothing
/// connects to postgres at boot — `DATABASE_URL` is scaffold-only — and
/// the container healthcheck `argo gateway probe live` is a pure TCP
/// GET that never opens the store.)
///
/// This replicates the successful retry-boot WITHIN a single boot: a
/// small bounded backoff ([`STORE_PROBE_ATTEMPTS`] attempts, doubling
/// from [`STORE_PROBE_BACKOFF_MS`], capped at
/// [`STORE_PROBE_BACKOFF_CAP_MS`]) absorbs the transient first-init.
/// Each attempt logs to stderr so the readiness wait is OBSERVABLE, and
/// a store that stays unreachable past the budget still fails closed
/// with the original error (→ `exit 78`).
///
/// This is a targeted in-process readiness gate on the one check that
/// races first-init — NOT a blind container `restart: on-failure`
/// (DEC-029: a green-on-retry that papers over an unknown failure is
/// not a fix; a bounded readiness wait that closes the actual first-init
/// race is). The config checks (JWT secret, provider keys, schema
/// version) stay single-shot fail-fast so a genuine misconfig is never
/// retried into a false pass.
async fn open_store_for_probe_with_retry(db: &Path) -> ArgoResult<SessionStore> {
    let mut backoff = Duration::from_millis(STORE_PROBE_BACKOFF_MS);
    let mut last_err: Option<ArgoError> = None;
    for attempt in 1..=STORE_PROBE_ATTEMPTS {
        match try_open_and_ping(db).await {
            Ok(store) => {
                if attempt > 1 {
                    eprintln!(
                        "argo gateway: startup_check store reachable on attempt \
                         {attempt}/{STORE_PROBE_ATTEMPTS} (fresh-volume first-init settled)"
                    );
                }
                return Ok(store);
            }
            Err(e) => {
                eprintln!(
                    "argo gateway: startup_check store probe attempt \
                     {attempt}/{STORE_PROBE_ATTEMPTS} failed: {e}"
                );
                last_err = Some(e);
                if attempt < STORE_PROBE_ATTEMPTS {
                    tokio::time::sleep(backoff).await;
                    backoff = (backoff * 2).min(Duration::from_millis(STORE_PROBE_BACKOFF_CAP_MS));
                }
            }
        }
    }
    Err(last_err.unwrap_or_else(|| {
        ArgoError::Provider("startup check: store unreachable (no attempts ran)".into())
    }))
}

/// One store-reachability attempt: open the store and issue a trivial
/// `SELECT 1`. On a `SELECT 1` failure the just-opened handle is closed
/// before returning the error so a retry starts from a clean pool.
async fn try_open_and_ping(db: &Path) -> ArgoResult<SessionStore> {
    let store = SessionStore::open(db)
        .await
        .map_err(|e| ArgoError::Provider(format!("startup check: store unreachable: {e}")))?;
    match sqlx::query("SELECT 1").fetch_one(store.pool()).await {
        Ok(_) => Ok(store),
        Err(e) => {
            store.close().await;
            Err(ArgoError::Provider(format!("startup check: SELECT 1: {e}")))
        }
    }
}

/// Warn (do NOT refuse) when the gateway is in production mode but
/// `SKYCOT_PUBLIC_BASE_URL` is unset. Magic-link / invite emails then
/// mint links against the hardcoded `https://app.skycot.com` default,
/// which is wrong for any deploy on a different host
/// (launch-email-delivery-honesty-and-public-base-url). Warn-only
/// because a same-host deploy where the default IS correct must still
/// boot; the loud line is the operator's cue to set the var.
fn warn_public_base_url_unset(bind: &str) {
    if !super::routes_auth_production_mode::is_production_mode_for_bind(bind) {
        return;
    }
    let set = std::env::var("SKYCOT_PUBLIC_BASE_URL").is_ok_and(|v| !v.trim().is_empty());
    if !set {
        eprintln!(
            "argo gateway: WARNING — SKYCOT_PUBLIC_BASE_URL is unset in production; \
             magic-link + invite emails will mint links against the default \
             https://app.skycot.com. Set SKYCOT_PUBLIC_BASE_URL to this deploy's \
             public origin (e.g. https://app.yourdomain.com) or links will point \
             at the wrong host."
        );
    }
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

/// Env var holding the session-cookie HS256 secret. Mirrors
/// `middleware_session_verify::SECRET_ENV` (kept as a local const
/// rather than reaching across the module boundary — that file is
/// outside this brief's scope).
const SESSION_JWT_SECRET_ENV: &str = "SKYCOT_AUTH_JWT_SECRET";

/// Minimum byte length for the HS256 session secret. 32 bytes = 256
/// bits, the HS256 key-size floor.
const MIN_JWT_SECRET_BYTES: usize = 32;

/// Fail-closed boot-guard for the session-cookie HS256 secret.
///
/// `middleware_session_verify::decode_session_cookie` treats an empty
/// `SKYCOT_AUTH_JWT_SECRET` as `NoCookie` — silently ignoring every
/// session cookie — on the comment-asserted promise that "production
/// callers detect the missing secret upstream". **This is that upstream
/// guard.** Without it, booting a production gateway with the secret
/// unset / empty silently disables session-cookie auth for every
/// browser user: a fail-open misconfiguration with no error and no log.
///
/// In production mode the secret MUST be set, non-empty, and at least
/// [`MIN_JWT_SECRET_BYTES`] bytes (the HS256 256-bit floor). Dev / demo
/// / bypass runs are left untouched so the documented dev-without-secret
/// recovery path (`middleware_session_verify.rs:161-167`) still works.
fn jwt_secret_boot_guard(bind: &str, cwd: &Path) -> ArgoResult<()> {
    if !jwt_guard_active(bind, cwd) {
        return Ok(());
    }
    let secret = std::env::var(SESSION_JWT_SECRET_ENV).unwrap_or_default();
    if secret.is_empty() {
        return Err(ArgoError::Provider(
            "SKYCOT_AUTH_JWT_SECRET must be set in production — session-cookie \
             auth is silently disabled without it. Set it to a random secret of \
             at least 32 bytes, or set ARGO_GATEWAY_AUTH_BYPASS=1 for dev/demo runs."
                .into(),
        ));
    }
    if secret.len() < MIN_JWT_SECRET_BYTES {
        return Err(ArgoError::Provider(format!(
            "SKYCOT_AUTH_JWT_SECRET is too short ({} bytes) — HS256 requires a \
             secret of at least {MIN_JWT_SECRET_BYTES} bytes (256-bit) in production.",
            secret.len()
        )));
    }
    Ok(())
}

/// Env vars holding the two audit-signing HMAC secrets. These sign the
/// public download URLs for `GET /api/v1/audit/export/<id>` (export
/// bundles) and `GET /api/v1/audit/<id>/screenshots/<idx>` (computer-use
/// screenshots). Mirror the local-const pattern used for
/// `SESSION_JWT_SECRET_ENV` (the canonical names also live in
/// `routes_audit_export_signing.rs` / `routes_audit_screenshot.rs`).
const AUDIT_EXPORT_SECRET_ENV: &str = "ARGO_AUDIT_EXPORT_SECRET";
const AUDIT_SCREENSHOT_SECRET_ENV: &str = "ARGO_AUDIT_SCREENSHOT_SECRET";

/// Fail-closed boot-guard for the two audit-signing HMAC secrets
/// (`fix-mf1-audit-export-secret-fail-closed`).
///
/// `routes_audit_export_signing::secret_bytes` and
/// `routes_audit_screenshot::secret_bytes` previously fell back to a
/// hardcoded, publicly-known dev secret
/// (`b"argo-dev-export-secret-do-not-use-in-prod"` /
/// `b"argo-dev-screenshot-secret-do-not-use-in-prod"`) whenever the env
/// var was unset. In production that makes the signed audit-download +
/// screenshot URLs **forgeable for any tenant** — anyone who reads the
/// (open-source) fallback constant can mint a valid `sig` for any bundle
/// id. **This is the upstream guard.** In production mode both secrets
/// MUST be set and non-empty. Dev / demo / bypass runs are left
/// untouched (same escape hatches as the JWT-secret guard), so the
/// loopback dev-without-secret path still works.
fn audit_signing_secret_boot_guard(bind: &str, cwd: &Path) -> ArgoResult<()> {
    if !jwt_guard_active(bind, cwd) {
        return Ok(());
    }
    for env in [AUDIT_EXPORT_SECRET_ENV, AUDIT_SCREENSHOT_SECRET_ENV] {
        let secret = std::env::var(env).unwrap_or_default();
        if secret.trim().is_empty() {
            return Err(ArgoError::Provider(format!(
                "{env} must be set in production — the signed audit-download \
                 + screenshot URLs are forgeable without it (they fall back to \
                 a publicly-known dev secret). Set it to a random secret (e.g. \
                 openssl rand -hex 32), or set ARGO_GATEWAY_AUTH_BYPASS=1 for \
                 dev/demo runs."
            )));
        }
    }
    Ok(())
}

/// Whether the JWT-secret boot-guard should enforce. True only when the
/// gateway is in production mode AND not in a dev/demo/bypass escape
/// hatch (those paths do not rely on session-cookie auth, so a missing
/// secret is not a fail-open there).
///
/// Production mode reuses the canonical **bind-aware** signal the two
/// sibling boot gates already honor —
/// [`super::routes_auth_production_mode::is_production_mode_for_bind`]
/// (`ARGO_GATEWAY_PROFILE=production`, a non-loopback `ARGO_GATEWAY_BIND`,
/// or — the load-bearing disjunct — the explicit non-loopback `--bind`
/// CLI value the socket actually binds to). The bind-blind
/// `is_production_mode()` checks the *compiled default* host, not the
/// runtime `--bind`, so a standard `argo gateway start --bind 0.0.0.0`
/// deploy without an `ARGO_GATEWAY_PROFILE` export would evade it — the
/// exact fail-open hole the bearer-token (`boot.rs`) and
/// enterprise-encryption gates already close by taking `bind`.
///
/// The escape hatches mirror `boot_serve::init_boot`'s effective config
/// merge:
///   - `gateway.auth_bypass` from `argo.toml`, merged with env
///     `ARGO_GATEWAY_AUTH_BYPASS` via [`ArgoConfig::from_env`].
///   - `ARGO_DEMO_MODE=1` (the demo signup-bypass escape hatch).
fn jwt_guard_active(bind: &str, cwd: &Path) -> bool {
    if !super::routes_auth_production_mode::is_production_mode_for_bind(bind) {
        return false;
    }
    let mut cfg = ArgoConfig::load(&cwd.join("argo.toml")).unwrap_or_default();
    if let Ok(env_cfg) = ArgoConfig::from_env() {
        cfg.merge(env_cfg);
    }
    let demo_mode = std::env::var("ARGO_DEMO_MODE").is_ok_and(|v| v == "1");
    !(cfg.gateway.auth_bypass || demo_mode)
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
