//! Reverse-proxy contract enforcement
//! (`mission-gateway-tls-termination-or-proxy-doc`).
//!
//! Skycot's gateway ships **without native TLS**. Production
//! deployments MUST front it with a TLS-terminating reverse proxy
//! (caddy / nginx / traefik / envoy / cloudflare-tunnel — see
//! `docs/skycot/runbook/gateway-tls-deployment.md` for end-to-end
//! recipes). This module is how that contract is enforced at boot
//! and honored at request time:
//!
//! - [`validate_bind_address`] is the **load-bearing** bind-safety
//!   guard. It refuses to boot when the operator binds a non-loopback
//!   address without setting `[gateway.behind_proxy].expected_proxy =
//!   true` (or passing the explicit `--insecure-bind` escape hatch).
//!   Today's `argo gateway start --bind 0.0.0.0:7400` failure mode —
//!   plaintext HTTP served to the world — exits non-zero with a
//!   runbook link instead.
//! - [`extract_real_ip`] picks the request source IP for downstream
//!   logging / per-IP rate-limiting. When `expected_proxy = true` AND
//!   the TCP peer is in `trusted_proxies` (or is loopback — the
//!   same-host caddy / nginx pattern), `X-Forwarded-For` is honored;
//!   otherwise the header is dropped with a `warn` log to surface
//!   spoofing attempts.
//!
//! The brief's `[gateway.behind_proxy]` block is the operator-facing
//! contract:
//!
//! ```toml
//! [gateway.behind_proxy]
//! expected_proxy   = true            # I have a TLS proxy in front
//! trusted_proxies  = ["10.0.0.5"]    # only this peer can set XFF
//! ```
//!
//! See `proxy_guard_tests.rs` for the unit-test surface.

use argo_config::GatewayBehindProxyConfig;
use argo_types::{ArgoError, ArgoResult};
use std::net::{IpAddr, SocketAddr};

/// Path the load-bearing error message points operators at. Kept as
/// a `const` so the smoke test can assert on the exact string.
pub(super) const RUNBOOK_PATH: &str = "docs/skycot/runbook/gateway-tls-deployment.md";

/// Validate the gateway's bind address against the
/// `[gateway.behind_proxy]` contract. Returns `Ok` when ANY of:
///
/// - `bind` parses to a loopback address (`127.0.0.0/8` or `::1`) —
///   local dev / on-host probes are always safe.
/// - `cfg.expected_proxy = true` — the operator has signed the
///   reverse-proxy contract.
/// - `insecure_bind = true` — the explicit CLI escape hatch.
///
/// Otherwise returns `Err` with a load-bearing message pointing at
/// the runbook ([`RUNBOOK_PATH`]). The error is what the BSD
/// `EX_CONFIG = 78` exit code surfaces to orchestrators.
pub(super) fn validate_bind_address(
    bind: &str,
    cfg: &GatewayBehindProxyConfig,
    insecure_bind: bool,
) -> ArgoResult<()> {
    let is_loopback = bind
        .parse::<IpAddr>()
        .map(|ip| ip.is_loopback())
        .unwrap_or(false);
    if is_loopback {
        return Ok(());
    }
    if cfg.expected_proxy {
        eprintln!(
            "argo gateway: non-loopback bind {bind} permitted; \
             [gateway.behind_proxy].expected_proxy=true \
             ({} trusted_proxies + loopback)",
            cfg.trusted_proxies.len()
        );
        return Ok(());
    }
    if insecure_bind {
        eprintln!(
            "\x1b[33margo gateway: binding non-loopback {bind} per --insecure-bind; \
             ensure a TLS-terminating proxy is in front (see {RUNBOOK_PATH})\x1b[0m"
        );
        return Ok(());
    }
    Err(ArgoError::Provider(format!(
        "refusing to bind non-loopback {bind} without TLS or reverse-proxy contract. \
         Set [gateway.behind_proxy].expected_proxy=true (with `trusted_proxies` listing \
         the load-balancer egress IPs) once a TLS-terminating proxy is in front, or pass \
         --insecure-bind for dev/recovery. See {RUNBOOK_PATH}"
    )))
}

/// Pick the request source IP based on the `[gateway.behind_proxy]`
/// contract. Used by the gateway accept loop for per-IP rate
/// limiting + audit-log attribution.
///
/// - `expected_proxy = false` ⇒ always returns the TCP peer IP.
///   `X-Forwarded-For` is untrusted in this mode.
/// - `expected_proxy = true` AND peer IP is loopback OR listed in
///   `trusted_proxies` ⇒ honors the first hop of `X-Forwarded-For`,
///   falling back to the peer IP when the header is missing.
/// - `expected_proxy = true` AND peer IP is NOT trusted ⇒ drops the
///   header with a `warn` log (spoofing attempt or misconfigured LB)
///   and returns the peer IP.
///
/// The loopback default-trust matches the same-host caddy / nginx /
/// envoy pattern (operator runs the proxy on the same box as the
/// gateway and proxies over `127.0.0.1:7400`); it relieves operators
/// from listing `127.0.0.1` in `trusted_proxies` for this common
/// case.
pub(super) fn extract_real_ip(
    req: &str,
    peer: SocketAddr,
    cfg: &GatewayBehindProxyConfig,
) -> IpAddr {
    if !cfg.expected_proxy {
        return peer.ip();
    }
    let peer_ip = peer.ip();
    let trusted = peer_ip.is_loopback() || cfg.trusted_proxies.contains(&peer_ip);
    if !trusted {
        tracing::warn!(
            target: "gateway::proxy_guard",
            peer = %peer_ip,
            "X-Forwarded-For dropped: peer not in [gateway.behind_proxy].trusted_proxies \
             and not loopback"
        );
        return peer_ip;
    }
    super::rate_limit_ip::parse_first_xff_hop(req).unwrap_or(peer_ip)
}

#[cfg(test)]
#[path = "proxy_guard_tests.rs"]
mod tests;
