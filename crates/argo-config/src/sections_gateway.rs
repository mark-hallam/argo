//! Gateway config sections: `[gateway]`, `[gateway.rate_limit]`,
//! `[gateway.rate_limit.tenant]`, `[gateway.behind_proxy]`. Extracted
//! from [`crate::sections`] to keep that module under the 500-LOC
//! ceiling. Re-exported through `sections` so the public API is
//! unchanged.
use crate::defaults;
use crate::sections_rate_limit::GatewayRateLimitConfig;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

/// `[gateway]` (top-level shape; nested adapter configs are
/// deserialized into raw values for the gateway crate to interpret).
///
/// # Examples
///
/// ```rust
/// use argo_config::GatewayConfig;
///
/// let cfg = GatewayConfig::default();
/// assert_eq!(cfg.host, "127.0.0.1");
/// ```
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
    /// Integration-test escape hatch for routes that normally require
    /// bearer-auth. Default `false`.
    pub auth_bypass: bool,
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
    /// `[gateway.request_audit]` per-request audit capture.
    pub request_audit: GatewayRequestAuditConfig,
    /// `[gateway.security_headers]` browser hardening knobs.
    pub security_headers: GatewaySecurityHeadersConfig,
    /// `[gateway.cors]` — Cross-Origin Resource Sharing allowlist for
    /// `/api/v1/*` browser callers.
    pub cors: GatewayCorsConfig,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            adapters: Vec::new(),
            host: defaults::DEFAULT_GATEWAY_HOST.into(),
            port: defaults::DEFAULT_GATEWAY_PORT,
            url: defaults::DEFAULT_GATEWAY_URL.into(),
            auth_bypass: false,
            duty_scheduler_secs: defaults::DEFAULT_DUTY_SCHEDULER_SECS,
            handoff_sweep_secs: defaults::DEFAULT_HANDOFF_SWEEP_SECS,
            rate_limit: GatewayRateLimitConfig::default(),
            behind_proxy: GatewayBehindProxyConfig::default(),
            request_audit: GatewayRequestAuditConfig::default(),
            security_headers: GatewaySecurityHeadersConfig::default(),
            cors: GatewayCorsConfig::default(),
        }
    }
}

/// `[gateway.cors]` — browser CORS allowlist for `/api/v1/*` routes
/// (mission-gateway-cors-preflight).
///
/// The gateway echoes the request `Origin` back in
/// `Access-Control-Allow-Origin` only when it appears in
/// `allowed_origins`. Origins not in the list receive responses
/// without any `Access-Control-Allow-*` header, which browsers
/// refuse — the fail-closed default that prevents drive-by
/// cross-origin reads.
///
/// Dev default is `["http://localhost:3000", "http://localhost:3001"]`
/// so the SPA dev server (port 3000) and Storybook (port 3001) work
/// against a freshly checked-out repo without configuration.
/// Production deployments override the list explicitly, e.g.
/// `allowed_origins = ["https://app.skycot.com"]`.
///
/// # Examples
///
/// ```rust
/// use argo_config::GatewayCorsConfig;
///
/// let cfg = GatewayCorsConfig::default();
/// assert!(cfg.allowed_origins.iter().any(|o| o == "http://localhost:3000"));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GatewayCorsConfig {
    /// Origins permitted to read `/api/v1/*` responses from a browser
    /// context. Match is exact-string against the request `Origin`
    /// header. Empty list disables CORS entirely (no browser callers
    /// can reach the API).
    pub allowed_origins: Vec<String>,
}

impl Default for GatewayCorsConfig {
    fn default() -> Self {
        Self {
            allowed_origins: vec![
                "http://localhost:3000".into(),
                "http://localhost:3001".into(),
            ],
        }
    }
}

/// `[gateway.security_headers]` — browser header rollout knobs.
///
/// # Examples
///
/// ```rust
/// use argo_config::GatewaySecurityHeadersConfig;
///
/// let cfg = GatewaySecurityHeadersConfig::default();
/// assert!(!cfg.csp_report_only);
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct GatewaySecurityHeadersConfig {
    /// When true, emit `Content-Security-Policy-Report-Only` instead
    /// of enforcing CSP. Default `false`.
    pub csp_report_only: bool,
}

/// `[gateway.request_audit]` — per-request audit table capture.
///
/// # Examples
///
/// ```rust
/// use argo_config::GatewayRequestAuditConfig;
///
/// let cfg = GatewayRequestAuditConfig::default();
/// assert!(cfg.body_hash_for_methods.contains(&"POST".to_string()));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GatewayRequestAuditConfig {
    /// `true` records one `gateway_request_audit` row per request.
    pub enabled: bool,
    /// `true` honors the leftmost `X-Forwarded-For` IP for the audit
    /// row. Defaults to false so direct deployments do not trust
    /// caller-supplied headers.
    pub behind_proxy: bool,
    /// HTTP methods whose non-empty request body should be hashed.
    pub body_hash_for_methods: Vec<String>,
    /// Maximum stored User-Agent length.
    pub user_agent_max_len: u32,
}

impl Default for GatewayRequestAuditConfig {
    fn default() -> Self {
        Self {
            enabled: defaults::DEFAULT_GATEWAY_REQUEST_AUDIT_ENABLED,
            behind_proxy: defaults::DEFAULT_GATEWAY_REQUEST_AUDIT_BEHIND_PROXY,
            body_hash_for_methods: vec!["POST".into(), "PUT".into(), "PATCH".into()],
            user_agent_max_len: defaults::DEFAULT_GATEWAY_REQUEST_AUDIT_UA_MAX,
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
///
/// # Examples
///
/// ```rust
/// use argo_config::GatewayBehindProxyConfig;
///
/// let cfg = GatewayBehindProxyConfig::default();
/// assert!(!cfg.expected_proxy);
/// ```
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
