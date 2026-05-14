//! Gateway rate-limit config sections.

use crate::defaults;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// `[gateway.rate_limit]` parent.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct GatewayRateLimitConfig {
    /// `[gateway.rate_limit.tenant]` — per-tenant defaults.
    pub tenant: GatewayRateLimitTenantConfig,
    /// `[gateway.rate_limit.ip]` — per-IP defaults.
    pub ip: GatewayRateLimitIpConfig,
    /// `[gateway.rate_limit.route]` — per-route defaults.
    pub route: GatewayRateLimitRouteConfig,
}

/// `[gateway.rate_limit.ip]` — per-source-IP token-bucket defaults.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GatewayRateLimitIpConfig {
    /// Master enable. Default `true`.
    pub enabled: bool,
    /// Bucket capacity per IP. Default 120.
    pub capacity: u32,
    /// Refill rate in tokens per second. Default 2.0.
    pub refill_per_sec: f64,
    /// Whether to trust the first `X-Forwarded-For` hop. Default
    /// `false`; production proxy deployments should prefer
    /// `[gateway.behind_proxy]` with trusted peer IPs.
    pub trust_xff: bool,
}

impl Default for GatewayRateLimitIpConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            capacity: 120,
            refill_per_sec: 2.0,
            trust_xff: false,
        }
    }
}

/// `[gateway.rate_limit.route]` — per-route token-bucket defaults.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GatewayRateLimitRouteConfig {
    /// Default requests per minute for routes without an override.
    pub default_rpm: u32,
    /// Stable route-key overrides.
    pub overrides: HashMap<String, u32>,
}

impl Default for GatewayRateLimitRouteConfig {
    fn default() -> Self {
        let mut overrides = HashMap::new();
        overrides.insert("forge.run".into(), 10);
        overrides.insert("skill.execute".into(), 30);
        Self {
            default_rpm: 100,
            overrides,
        }
    }
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
