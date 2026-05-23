//! Production-mode detection for the gateway bearer-token gate.
//!
//! CONFIG-REFERENCE: production mode is active when
//! `ARGO_GATEWAY_PROFILE=production`, when `ARGO_GATEWAY_BIND` points
//! at a non-loopback address, or when the compiled default gateway
//! host is non-loopback. The CLI boot path additionally checks the
//! explicit `--bind` value with [`is_production_mode_for_bind`].

use std::net::{IpAddr, SocketAddr};

const PROFILE_ENV: &str = "ARGO_GATEWAY_PROFILE";
const BIND_ENV: &str = "ARGO_GATEWAY_BIND";
pub(super) const BEARER_TOKEN_ENV: &str = "ARGO_GATEWAY_BEARER_TOKEN";

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
