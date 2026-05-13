//! Gateway config sections: `[gateway]`, `[gateway.rate_limit]`,
//! `[gateway.rate_limit.tenant]`, `[gateway.behind_proxy]`. Extracted
//! from [`crate::sections`] to keep that module under the 500-LOC
//! ceiling. Re-exported through `sections` so the public API is
//! unchanged.
use crate::defaults;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

/// `[gateway]` (top-level shape; nested adapter configs are
/// deserialized into raw values for the gateway crate to interpret).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GatewayConfig {
    /// Master enable.
    pub enabled: bool,
    /// Adapters to spin up.
    pub adapters: Vec<String>,
    /// Bind host. Default `127.0.0.1`.
    pub host: String,
    /// Bind port. Default [`crate::defaults::DEFAULT_GATEWAY_PORT`].
    pub port: u16,
    /// Public-facing URL clients use to reach this gateway.
    /// Default [`crate::defaults::DEFAULT_GATEWAY_URL`].
    pub url: String,
    /// Orchestrator duty scheduler tick interval (seconds).
    /// Default [`crate::defaults::DEFAULT_DUTY_SCHEDULER_SECS`].
    pub duty_scheduler_secs: u64,
    /// Handoff walker sweep interval (seconds).
    /// Default [`crate::defaults::DEFAULT_HANDOFF_SWEEP_SECS`].
    pub handoff_sweep_secs: u64,
    /// `[gateway.rate_limit]` — per-tenant rate-limit defaults.
    pub rate_limit: GatewayRateLimitConfig,
    /// `[gateway.behind_proxy]` — reverse-proxy contract. Operators
    /// terminating TLS upstream set `expected_proxy = true` to allow
    /// the gateway to bind non-loopback addresses, and list the
    /// load-balancer IPs in `trusted_proxies` so `X-Forwarded-For` is
    /// only honored from those peers. See
    /// `docs/skycot/runbook/gateway-tls-deployment.md`.
    pub behind_proxy: GatewayBehindProxyConfig,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            adapters: Vec::new(),
            host: defaults::DEFAULT_GATEWAY_HOST.into(),
            port: defaults::DEFAULT_GATEWAY_PORT,
            url: defaults::DEFAULT_GATEWAY_URL.into(),
            duty_scheduler_secs: defaults::DEFAULT_DUTY_SCHEDULER_SECS,
            handoff_sweep_secs: defaults::DEFAULT_HANDOFF_SWEEP_SECS,
            rate_limit: GatewayRateLimitConfig::default(),
            behind_proxy: GatewayBehindProxyConfig::default(),
        }
    }
}

/// `[gateway.behind_proxy]` — operator contract for reverse-proxy
/// deployment. The gateway ships without native TLS; production
/// deployments MUST front it with a TLS-terminating proxy (caddy,
/// nginx, traefik, envoy, cloudflare-tunnel — see
/// `docs/skycot/runbook/gateway-tls-deployment.md` for end-to-end
/// recipes). This block is how the operator signs that contract:
/// flipping `expected_proxy = true` is the formal acknowledgment that
/// allows a non-loopback bind to boot, and `trusted_proxies` scopes
/// `X-Forwarded-For` honoring to specific peer IPs.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GatewayBehindProxyConfig {
    /// `true` ⇒ the gateway will allow a non-loopback bind (e.g.
    /// `0.0.0.0`) without `--insecure-bind`, on the operator's
    /// promise that a TLS-terminating proxy fronts it. The
    /// bind-safety guard refuses to boot non-loopback when this is
    /// `false`. Default
    /// [`crate::defaults::DEFAULT_GATEWAY_EXPECTED_PROXY`].
    pub expected_proxy: bool,
    /// Peer IPs (the TCP-layer source seen by `TcpStream::peer_addr`)
    /// permitted to set `X-Forwarded-For`. Loopback peers are always
    /// trusted regardless of this list — same-host caddy / nginx /
    /// envoy patterns work without listing `127.0.0.1` explicitly.
    /// Empty by default; remote load-balancer deployments MUST
    /// populate it with the LB's egress IP(s).
    pub trusted_proxies: Vec<IpAddr>,
}

impl Default for GatewayBehindProxyConfig {
    fn default() -> Self {
        Self {
            expected_proxy: defaults::DEFAULT_GATEWAY_EXPECTED_PROXY,
            trusted_proxies: Vec::new(),
        }
    }
}

/// `[gateway.rate_limit]` parent — currently only the `tenant`
/// sub-block is consumed by the gateway, but the parent is its own
/// struct so the existing per-route limiter (which the
/// `claude-rate-limit-per-route` brief left env-driven) can grow a
/// `[gateway.rate_limit.route]` block without another schema flip.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct GatewayRateLimitConfig {
    /// `[gateway.rate_limit.tenant]` — per-tenant defaults.
    pub tenant: GatewayRateLimitTenantConfig,
}

/// `[gateway.rate_limit.tenant]` — per-tenant rate-limit defaults
/// for the gateway's `RateLimitTenant` middleware (Stream
/// `mission-per-tenant-rate-limiting`, MISSION_BACKLOG Section 1
/// #21). Per-tenant overrides live in the `tenant_rate_limit` table
/// and take precedence over this config block.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GatewayRateLimitTenantConfig {
    /// Default per-minute budget per tenant. `0` disables the
    /// limiter entirely. Default
    /// [`crate::defaults::DEFAULT_GATEWAY_RATE_LIMIT_TENANT_RPM`].
    pub requests_per_minute: u32,
    /// Burst multiplier — the in-memory token bucket holds
    /// `requests_per_minute * burst_multiplier` tokens, so a tenant
    /// can spike up to that many requests in one second before the
    /// refill rate (rpm/60 per second) bounds them. Default
    /// [`crate::defaults::DEFAULT_GATEWAY_RATE_LIMIT_TENANT_BURST`].
    pub burst_multiplier: f64,
}

impl Default for GatewayRateLimitTenantConfig {
    fn default() -> Self {
        Self {
            requests_per_minute: defaults::DEFAULT_GATEWAY_RATE_LIMIT_TENANT_RPM,
            burst_multiplier: defaults::DEFAULT_GATEWAY_RATE_LIMIT_TENANT_BURST,
        }
    }
}
