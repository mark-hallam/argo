//! Centralised reqwest (provider HTTP) connection-pool tuning knobs.
//!
//! Today's effective defaults at every `Client::builder()` site
//! (`anthropic`, `openai_responses`, `openai_compat`) are:
//! - `pool_max_idle_per_host = usize::MAX` (reqwest default — no cap)
//! - `pool_idle_timeout = 90s` (reqwest default)
//! - `connect_timeout` and `request_timeout` come from the provider
//!   manifest's `transport.connect_timeout_ms` /
//!   `transport.read_timeout_ms` and that wins over this struct.
//!
//! [`ProviderPoolConfig::from_env`] preserves reqwest's pool defaults
//! when no env vars are set, so a build with empty env is
//! byte-equivalent to the prior behaviour. Operators tuning a swarm
//! can override any field via env without recompiling:
//!
//! | Env var                              | Field                    | Default     |
//! |--------------------------------------|--------------------------|-------------|
//! | `ARGO_HTTP_POOL_MAX_IDLE_PER_HOST`   | pool_max_idle_per_host   | usize::MAX  |
//! | `ARGO_HTTP_POOL_IDLE_TIMEOUT_S`      | pool_idle_timeout        | 90          |
//! | `ARGO_HTTP_CONNECT_TIMEOUT_S`        | connect_timeout          | 10          |
//! | `ARGO_HTTP_REQUEST_TIMEOUT_S`        | request_timeout          | 120         |
//!
//! Recommended swarm-scale tuning — cap idle sockets per provider
//! host, shorten the idle reap window so fan-out doesn't keep dead
//! TLS sessions alive:
//!
//! ```sh
//! export ARGO_HTTP_POOL_MAX_IDLE_PER_HOST=8
//! export ARGO_HTTP_POOL_IDLE_TIMEOUT_S=30
//! ```
//!
//! Invalid env values fall back to the default with a `tracing::warn!`.
//! They never panic — `from_env` is infallible.
//!
//! Note: `connect_timeout` and `request_timeout` are exposed for
//! callers that build a client without a manifest. Manifest-driven
//! transports (the v1 default) keep using the per-provider manifest
//! values; [`ProviderPoolConfig::apply_pool`] only stamps the pool
//! sizing knobs onto a `ClientBuilder`, leaving timeouts untouched.

use reqwest::ClientBuilder;
use std::time::Duration;

/// Parsed reqwest pool tuning. Built from env via
/// [`ProviderPoolConfig::from_env`] when each transport constructs its
/// HTTP client.
///
/// ```
/// use argo_provider::pool_config::ProviderPoolConfig;
/// let cfg = ProviderPoolConfig::from_env();
/// assert!(cfg.pool_idle_timeout.as_secs() > 0);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderPoolConfig {
    /// Per-host idle-socket cap. reqwest default is `usize::MAX`.
    pub pool_max_idle_per_host: usize,
    /// How long an idle socket lives before reqwest drops it.
    pub pool_idle_timeout: Duration,
    /// Default TCP-connect timeout for clients built without a
    /// manifest. Manifest-driven transports override this.
    pub connect_timeout: Duration,
    /// Default end-to-end request timeout for clients built without a
    /// manifest. Manifest-driven transports override this.
    pub request_timeout: Duration,
}

impl Default for ProviderPoolConfig {
    fn default() -> Self {
        Self {
            pool_max_idle_per_host: usize::MAX,
            pool_idle_timeout: Duration::from_secs(90),
            connect_timeout: Duration::from_secs(10),
            request_timeout: Duration::from_secs(120),
        }
    }
}

impl ProviderPoolConfig {
    /// Parse `ARGO_HTTP_*` env vars; invalid values warn and fall
    /// back to [`ProviderPoolConfig::default`].
    #[must_use]
    pub fn from_env() -> Self {
        let mut cfg = Self::default();
        if let Some(v) = parse_env_usize("ARGO_HTTP_POOL_MAX_IDLE_PER_HOST") {
            cfg.pool_max_idle_per_host = v;
        }
        if let Some(secs) = parse_env_u64("ARGO_HTTP_POOL_IDLE_TIMEOUT_S") {
            cfg.pool_idle_timeout = Duration::from_secs(secs);
        }
        if let Some(secs) = parse_env_u64("ARGO_HTTP_CONNECT_TIMEOUT_S") {
            cfg.connect_timeout = Duration::from_secs(secs);
        }
        if let Some(secs) = parse_env_u64("ARGO_HTTP_REQUEST_TIMEOUT_S") {
            cfg.request_timeout = Duration::from_secs(secs);
        }
        cfg
    }

    /// Stamp the pool-sizing knobs onto an existing [`ClientBuilder`].
    /// Timeouts are intentionally left alone so manifest-driven
    /// transports keep their per-provider values.
    pub fn apply_pool(&self, builder: ClientBuilder) -> ClientBuilder {
        builder
            .pool_max_idle_per_host(self.pool_max_idle_per_host)
            .pool_idle_timeout(Some(self.pool_idle_timeout))
    }
}

fn parse_env_usize(key: &str) -> Option<usize> {
    let raw = std::env::var(key).ok()?;
    match raw.parse::<usize>() {
        Ok(v) => Some(v),
        Err(_) => {
            tracing::warn!(
                env = key,
                value = %raw,
                "invalid usize; falling back to default"
            );
            None
        }
    }
}

fn parse_env_u64(key: &str) -> Option<u64> {
    let raw = std::env::var(key).ok()?;
    match raw.parse::<u64>() {
        Ok(v) => Some(v),
        Err(_) => {
            tracing::warn!(
                env = key,
                value = %raw,
                "invalid u64; falling back to default"
            );
            None
        }
    }
}

#[cfg(test)]
#[path = "pool_config_tests.rs"]
mod tests;
