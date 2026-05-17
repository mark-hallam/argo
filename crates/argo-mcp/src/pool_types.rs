//! Pure data types for the MCP client pool.
//!
//! Extracted from `pool.rs` under the
//! `codex-split-files-450-500-wave` brief. Keeping the docstring
//! examples on the data shapes (constants, config knobs, the
//! cache key, and the operator-facing stats snapshot) here lets
//! `pool.rs` focus on the cache + guard logic.
use argo_types::TenantId;

/// Default LRU capacity. Bounded so an idle worker can't accumulate
/// hundreds of FDs against rarely-used MCP servers.
///
/// # Examples
///
/// ```
/// use argo_mcp::DEFAULT_POOL_MAX_CLIENTS;
/// assert_eq!(DEFAULT_POOL_MAX_CLIENTS, 32);
/// ```
pub const DEFAULT_POOL_MAX_CLIENTS: u32 = 32;
/// Default idle TTL — long enough to span turn-to-turn pauses, short
/// enough that abandoned sessions free their subprocesses.
///
/// # Examples
///
/// ```
/// use argo_mcp::DEFAULT_POOL_IDLE_TTL_SECS;
/// assert_eq!(DEFAULT_POOL_IDLE_TTL_SECS, 600);
/// ```
pub const DEFAULT_POOL_IDLE_TTL_SECS: u64 = 600;

/// Operator-tunable pool knobs. Defaults match
/// [`DEFAULT_POOL_MAX_CLIENTS`] / [`DEFAULT_POOL_IDLE_TTL_SECS`].
///
/// # Examples
///
/// ```
/// use argo_mcp::{PoolConfig, DEFAULT_POOL_MAX_CLIENTS, DEFAULT_POOL_IDLE_TTL_SECS};
/// let cfg = PoolConfig::default();
/// assert_eq!(cfg.max_clients, DEFAULT_POOL_MAX_CLIENTS);
/// assert_eq!(cfg.idle_ttl_secs, DEFAULT_POOL_IDLE_TTL_SECS);
///
/// let tight = PoolConfig { max_clients: 4, idle_ttl_secs: 60 };
/// assert_eq!(tight.max_clients, 4);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PoolConfig {
    /// Hard cap on cached clients. New acquisitions past this evict
    /// the LRU entry. `0` is treated as `1` so the cache always holds
    /// at least one slot.
    pub max_clients: u32,
    /// Seconds an entry can sit untouched before `McpClientPool::evict_idle`
    /// reaps it.
    pub idle_ttl_secs: u64,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_clients: DEFAULT_POOL_MAX_CLIENTS,
            idle_ttl_secs: DEFAULT_POOL_IDLE_TTL_SECS,
        }
    }
}

/// Pool key: `(tenant, manifest)`. Per-tenant isolation is the only
/// correctness invariant — see [`crate::pool`] module docs.
///
/// # Examples
///
/// ```
/// use argo_mcp::PoolKey;
/// use argo_types::TenantId;
/// let tenant = TenantId::new();
/// let key = PoolKey { tenant_id: tenant, manifest_id: "example-connector".into() };
/// assert_eq!(key.manifest_id, "example-connector");
/// // Same tenant + manifest hash-equal so the cache can dedupe lookups.
/// let twin = PoolKey { tenant_id: tenant, manifest_id: "example-connector".into() };
/// assert_eq!(key, twin);
/// ```
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct PoolKey {
    /// Tenant scope.
    pub tenant_id: TenantId,
    /// Caller-chosen identifier (typically the MCP manifest id, but
    /// the pool treats it as opaque).
    pub manifest_id: String,
}

/// Snapshot of pool state for operator tooling.
///
/// # Examples
///
/// ```
/// use argo_mcp::PoolStats;
/// let stats = PoolStats { entries: 0, idle_seconds_p50: 0, idle_seconds_max: 0 };
/// assert_eq!(stats.entries, 0);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoolStats {
    /// Number of cached entries.
    pub entries: usize,
    /// 50th-percentile idle time in seconds across all entries (0 if
    /// the pool is empty).
    pub idle_seconds_p50: u64,
    /// Maximum idle time in seconds (0 if the pool is empty).
    pub idle_seconds_max: u64,
}
