//! Production-mode detection for the gateway bearer-token gate.
//!
//! CONFIG-REFERENCE: production mode is active when
//! `ARGO_GATEWAY_PROFILE=production`, when `ARGO_GATEWAY_BIND` points
//! at a non-loopback address, or when the compiled default gateway
//! host is non-loopback. The CLI boot path additionally checks the
//! explicit `--bind` value with [`is_production_mode_for_bind`].

use std::net::{IpAddr, SocketAddr};
use std::sync::OnceLock;

const PROFILE_ENV: &str = "ARGO_GATEWAY_PROFILE";
const BIND_ENV: &str = "ARGO_GATEWAY_BIND";
pub(super) const BEARER_TOKEN_ENV: &str = "ARGO_GATEWAY_BEARER_TOKEN";

/// The bind-AWARE production decision, captured ONCE at gateway boot from
/// the runtime `--bind` value (via [`record_bind_production`]). Per-request
/// gates that cannot see `bind` (CSRF middleware, cookie emit) read it
/// through [`bind_aware_production_mode`]. Write-once; never mutated after
/// boot — the same pattern the bearer-token / CORS boot gates use.
static BIND_PRODUCTION: OnceLock<bool> = OnceLock::new();

/// Capture the bind-aware production decision at boot. Called once from the
/// gateway boot path where the runtime `--bind` is in scope. Idempotent
/// (first writer wins; subsequent calls are no-ops).
pub(super) fn record_bind_production(bind: &str) {
    let _ = BIND_PRODUCTION.set(is_production_mode_for_bind(bind));
}

/// The bind-AWARE production decision for per-request gates. Prefers the
/// boot-captured value; **fails CLOSED to production (`true`)** when boot
/// has not recorded it yet — so a path reading it before boot can never
/// fail-open (e.g. silently mint a non-`Secure` dev CSRF cookie). Replaces
/// the bind-BLIND [`is_production_mode`] at the CSRF gates, which the
/// bearer-token / encryption / CORS gates already abandoned as fail-open
/// for a `--bind 0.0.0.0`-behind-a-proxy deploy with no `ARGO_GATEWAY_PROFILE`.
pub(super) fn bind_aware_production_mode() -> bool {
    *BIND_PRODUCTION.get().unwrap_or(&true)
}

pub(super) fn is_production_mode() -> bool {
    profile_is_production()
        || bind_env_is_non_loopback()
        || bind_host_is_non_loopback(argo_config::defaults::DEFAULT_GATEWAY_HOST)
}

pub(super) fn is_production_mode_for_bind(bind: &str) -> bool {
    profile_is_production() || bind_env_is_non_loopback() || bind_host_is_non_loopback(bind)
}

pub(super) fn bearer_token_configured() -> bool {
    // tenant-unscoped: gateway process-level secret (no per-request tenant context for this slot)
    argo_config::cred::get_unscoped(BEARER_TOKEN_ENV).is_ok_and(|t| !t.trim().is_empty())
}

/// True when the operator has EXPLICITLY opened the perimeter for local
/// dev / demo / tests via `ARGO_GATEWAY_AUTH_BYPASS=1` (env →
/// `gateway.auth_bypass`).
///
/// This is the ONLY switch that opens the perimeter (WS5: fail-closed by
/// identity). The default — and therefore every real deployment that does
/// not set the flag — fails CLOSED: anonymous requests to protected routes
/// are rejected, the RBAC role defaults to Guest, and the JWT-secret boot
/// guard is armed. It replaces the old `!is_production_mode()` heuristic as
/// the fail-open signal, so a loopback-behind-a-proxy deploy can no longer
/// silently serve an open perimeter just because the heuristic didn't trip.
pub(super) fn perimeter_open() -> bool {
    argo_config::load().unwrap_or_default().gateway.auth_bypass
}

fn profile_is_production() -> bool {
    std::env::var(PROFILE_ENV).is_ok_and(|p| p.eq_ignore_ascii_case("production"))
}

fn bind_env_is_non_loopback() -> bool {
    std::env::var(BIND_ENV).is_ok_and(|bind| bind_host_is_non_loopback(&bind))
}

fn bind_host_is_non_loopback(bind: &str) -> bool {
    parse_bind_ip(bind).is_some_and(|ip| !ip.is_loopback())
}

fn parse_bind_ip(raw: &str) -> Option<IpAddr> {
    if let Ok(addr) = raw.parse::<SocketAddr>() {
        return Some(addr.ip());
    }
    if let Ok(ip) = raw.parse::<IpAddr>() {
        return Some(ip);
    }
    raw.rsplit_once(':')
        .and_then(|(host, _port)| host.parse::<IpAddr>().ok())
}
