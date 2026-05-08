//! Centralised sqlx (SQLite) connection-pool tuning knobs.
//!
//! Today's effective defaults at the `SessionStore::open` site are:
//! - `max_connections = 8`
//! - `min_connections = 0` (sqlx default)
//! - `acquire_timeout = 30s` (sqlx default)
//! - `idle_timeout = None` (sqlx default — no idle reaping)
//!
//! [`PoolConfig::from_env`] preserves those numbers as the structural
//! defaults so a build with no env vars set is byte-equivalent to the
//! previous behaviour. Operators tuning a swarm can override any
//! field via env without recompiling:
//!
//! | Env var                              | Field             | Default |
//! |--------------------------------------|-------------------|---------|
//! | `ARGO_STORE_POOL_MAX`                | max_connections   | 8       |
//! | `ARGO_STORE_POOL_MIN`                | min_connections   | 0       |
//! | `ARGO_STORE_POOL_ACQUIRE_TIMEOUT_MS` | acquire_timeout   | 30000   |
//! | `ARGO_STORE_POOL_IDLE_TIMEOUT_S`     | idle_timeout      | unset   |
//!
//! Recommended swarm-scale tuning — bump max to ~16, set idle timeout
//! to 60s so reapers free file handles between iterations:
//!
//! ```sh
//! export ARGO_STORE_POOL_MAX=16
//! export ARGO_STORE_POOL_IDLE_TIMEOUT_S=60
//! ```
//!
//! Invalid env values fall back to the default with an `eprintln!`
//! warning. They never panic — `from_env` is infallible.

use std::time::Duration;

/// Parsed sqlx pool tuning. Built from env via [`PoolConfig::from_env`]
/// at `SessionStore::open` time.
///
/// ```
/// use argo_store::pool_config::PoolConfig;
/// let cfg = PoolConfig::from_env();
/// assert!(cfg.max_connections >= cfg.min_connections);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoolConfig {
    /// Hard ceiling on simultaneously-checked-out connections.
    pub max_connections: u32,
    /// Lower bound — sqlx keeps at least this many connections live.
    pub min_connections: u32,
    /// How long a `pool.acquire()` call waits before erroring out.
    pub acquire_timeout: Duration,
    /// Optional idle-reap window. `None` keeps connections forever
    /// (sqlx default).
    pub idle_timeout: Option<Duration>,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_connections: 8,
            min_connections: 0,
            acquire_timeout: Duration::from_secs(30),
            idle_timeout: None,
        }
    }
}

impl PoolConfig {
    /// Parse `ARGO_STORE_POOL_*` env vars; invalid values warn and fall
    /// back to [`PoolConfig::default`].
    #[must_use]
    pub fn from_env() -> Self {
        let mut cfg = Self::default();
        if let Some(v) = parse_env_u32("ARGO_STORE_POOL_MAX") {
            cfg.max_connections = v;
        }
        if let Some(v) = parse_env_u32("ARGO_STORE_POOL_MIN") {
            cfg.min_connections = v;
        }
        if let Some(ms) = parse_env_u64("ARGO_STORE_POOL_ACQUIRE_TIMEOUT_MS") {
            cfg.acquire_timeout = Duration::from_millis(ms);
        }
        if let Some(secs) = parse_env_u64("ARGO_STORE_POOL_IDLE_TIMEOUT_S") {
            cfg.idle_timeout = Some(Duration::from_secs(secs));
        }
        cfg
    }
}

fn parse_env_u32(key: &str) -> Option<u32> {
    let raw = std::env::var(key).ok()?;
    match raw.parse::<u32>() {
        Ok(v) => Some(v),
        Err(_) => {
            eprintln!(
                "warning: {key}={raw} is not a valid u32; falling back to default"
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
            eprintln!(
                "warning: {key}={raw} is not a valid u64; falling back to default"
            );
            None
        }
    }
}

#[cfg(test)]
#[path = "pool_config_tests.rs"]
mod tests;
