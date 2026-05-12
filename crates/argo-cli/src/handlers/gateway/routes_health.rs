//! Health-probe handlers (`mission-health-check-endpoint`). Five
//! paths share three handlers here:
//!
//!   - `GET /health` and `GET /readyz` — `health_response`; full
//!     readiness probe. 200 + `{"status":"ok","schema_version":N,
//!     "uptime_seconds":S}` on pass; 503 + `{"status":"degraded",
//!     "reason":"<tag>"}` on first failing check.
//!   - `GET /livez` and `GET /health/live` — `live_response`;
//!     k8s-convention liveness, always 200, no readiness checks.
//!   - `GET /health/ready` — `ready_response`; legacy SQLite-ping
//!     only, kept for pre-existing callers.
//!
//! Sequential readiness checks (short-circuit on first failure):
//! SQLite `SELECT 1` < `SQLITE_PING_BUDGET_MS`; on-disk
//! `schema_version` matches `EXPECTED_SCHEMA_VERSION` (or 0 = fresh
//! install); at least one provider manifest under `manifests/
//! providers/` (or dir absent); auth keystore reachable (bypass
//! env var set, no credstore file, or credstore parses cleanly).
//!
//! All endpoints are unauthenticated — k8s/systemd need them
//! callable without credentials. Bypass is enforced upstream by
//! `routes_auth::is_public_get`.
use argo_store::{EXPECTED_SCHEMA_VERSION, SessionStore};
use std::path::Path;
use std::sync::{Arc, OnceLock};
use std::time::Instant;

/// SQLite `SELECT 1` round-trip ceiling for the readiness check.
/// Tunable as a constant so a slow disk doesn't false-positive a
/// degraded probe in dev.
const SQLITE_PING_BUDGET_MS: u128 = 100;

/// Captured once at gateway start (see `mod.rs::run`). `OnceLock`
/// so `mark_start` is idempotent across re-entry and lazy
/// initialisation is harmless: the first probe call lands well
/// before any operator-visible uptime threshold matters.
static GATEWAY_START: OnceLock<Instant> = OnceLock::new();

/// Record the gateway boot instant. Called from the `GatewayCmd::Start`
/// arm in `mod.rs::run`. Idempotent — subsequent calls are no-ops.
pub(super) fn mark_start() {
    let _ = GATEWAY_START.set(Instant::now());
}

fn uptime_seconds() -> u64 {
    GATEWAY_START.get_or_init(Instant::now).elapsed().as_secs()
}

/// `GET /health` and `GET /readyz` — full readiness probe.
///
/// Returns `200 OK` + `{"status":"ok","schema_version":N,"uptime_seconds":S}`
/// when every check passes, `503 Service Unavailable` +
/// `{"status":"degraded","reason":"<machine_tag>"}` on the first
/// failing check (short-circuit).
pub(super) async fn health_response(
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let cwd = std::env::current_dir().ok();
    health_response_with_cwd(store, cwd.as_deref()).await
}

/// Testable inner: same as [`health_response`] but the workspace
/// root is injected. Production calls pass `current_dir()`; unit
/// tests pass a tempdir so the providers / credstore checks are
/// hermetic.
pub(super) async fn health_response_with_cwd(
    store: Option<&Arc<SessionStore>>,
    cwd: Option<&Path>,
) -> (&'static str, String) {
    let schema_version = match check_store_and_schema(store).await {
        Ok(v) => v,
        Err(reason) => return degraded(reason),
    };
    if let Some(cwd) = cwd {
        if let Err(reason) = check_providers(cwd) {
            return degraded(reason);
        }
        if let Err(reason) = check_auth_keystore(cwd) {
            return degraded(reason);
        }
    }
    (
        "200 OK",
        format!(
            "{{\"status\":\"ok\",\"schema_version\":{schema_version},\
              \"uptime_seconds\":{}}}",
            uptime_seconds()
        ),
    )
}

/// `GET /livez` and `GET /health/live` — k8s liveness convention.
/// Always 200; no readiness checks. Useful to distinguish "alive
/// but unready" from "dead".
pub(super) fn live_response() -> (&'static str, String) {
    let now = chrono::Utc::now();
    (
        "200 OK",
        format!(
            "{{\"status\":\"alive\",\"epoch_micros\":{}}}",
            now.timestamp_micros()
        ),
    )
}

/// `GET /health/ready` — legacy SQLite-ping-only readiness. Kept
/// for callers that already adopted it pre-`mission-health-check-endpoint`;
/// new orchestrators should prefer `/readyz`.
pub(super) async fn ready_response(
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return (
            "503 Service Unavailable",
            "{\"status\":\"not_ready\",\"reason\":\"store_unavailable\"}".to_string(),
        );
    };
    match sqlx::query_scalar::<_, i64>("SELECT 1").fetch_one(store.pool()).await {
        Ok(_) => {
            let now = chrono::Utc::now();
            (
                "200 OK",
                format!(
                    "{{\"status\":\"ready\",\"epoch_micros\":{}}}",
                    now.timestamp_micros()
                ),
            )
        }
        Err(e) => (
            "503 Service Unavailable",
            format!("{{\"status\":\"not_ready\",\"reason\":\"db_unreachable: {e}\"}}"),
        ),
    }
}

fn degraded(reason: &str) -> (&'static str, String) {
    (
        "503 Service Unavailable",
        format!("{{\"status\":\"degraded\",\"reason\":\"{reason}\"}}"),
    )
}

async fn check_store_and_schema(
    store: Option<&Arc<SessionStore>>,
) -> Result<u32, &'static str> {
    let store = store.ok_or("store_unavailable")?;
    let started = Instant::now();
    sqlx::query_scalar::<_, i64>("SELECT 1")
        .fetch_one(store.pool())
        .await
        .map_err(|_| "db_unreachable")?;
    if started.elapsed().as_millis() > SQLITE_PING_BUDGET_MS {
        return Err("db_ping_slow");
    }
    let actual = store
        .schema_version()
        .await
        .map_err(|_| "schema_version_unreadable")?;
    // schema_version=0 is a fresh install (migrations not yet run on
    // disk); treat as ready since gateway boot runs migrations.
    if actual != 0 && actual != EXPECTED_SCHEMA_VERSION {
        return Err("schema_version_mismatch");
    }
    Ok(actual)
}

fn check_providers(cwd: &Path) -> Result<(), &'static str> {
    let dir = cwd.join("manifests").join("providers");
    if !dir.exists() {
        // Fresh install: no providers dir yet. Permissive.
        return Ok(());
    }
    let any_toml = std::fs::read_dir(&dir)
        .map(|rd| {
            rd.filter_map(Result::ok)
                .any(|e| e.path().extension().is_some_and(|x| x == "toml"))
        })
        .unwrap_or(false);
    if any_toml {
        Ok(())
    } else {
        Err("no_providers_configured")
    }
}

fn check_auth_keystore(cwd: &Path) -> Result<(), &'static str> {
    if std::env::var("ARGO_GATEWAY_AUTH_BYPASS").as_deref() == Ok("1") {
        return Ok(());
    }
    let path = argo_config::cred::credentials_path(cwd);
    if !path.exists() {
        // No credstore = dev default. The bearer-token gate treats
        // this as "auth disabled" and lets requests through, so we
        // mirror that here.
        return Ok(());
    }
    argo_config::cred::load(cwd)
        .map(|_| ())
        .map_err(|_| "auth_keystore_unreachable")
}

#[cfg(test)]
#[path = "routes_health_tests.rs"]
mod tests;
