//! MCP client pool — keep MCP server subprocesses warm across turns.
//!
//! Today every call to [`McpClient::connect_stdio`] spawns a fresh
//! subprocess and runs the JSON-RPC `initialize` handshake. Heavy
//! connectors (OAuth-refreshing or session-warming ones — Gmail, GCal,
//! Slack) pay 200-1500 ms cold-start per turn; a turn that touches three
//! distinct MCP tools pays the cost three times. [`McpClientPool`]
//! caches `Arc<McpClient>` instances keyed by
//! `(tenant_id, manifest_id)` and hands them out via
//! [`PoolGuard`]; the subprocess stays warm until the entry is evicted
//! by LRU pressure or by the idle-TTL reaper.
//!
//! ## Eviction
//!
//! - **LRU**: when the cache is at [`PoolConfig::max_clients`] and a
//!   new key arrives, the least-recently-used entry is popped and its
//!   subprocess shut down.
//! - **Idle TTL**: [`McpClientPool::evict_idle`] (driven by an external
//!   cron tick — see `mission-mcp-pool-evict-cron`) removes entries
//!   whose `last_used` is older than [`PoolConfig::idle_ttl_secs`].
//!
//! ## Per-tenant isolation
//!
//! The pool key is `(TenantId, manifest_id)` — never just
//! `manifest_id`. Two tenants asking for the same manifest get
//! distinct subprocesses with distinct credentials. Cross-tenant
//! collision would leak credentials across tenants and is the only
//! correctness invariant of this module.
//!
//! ## Outstanding guards survive eviction
//!
//! Entries hold `Arc<McpClient>` and so does the guard. When the pool
//! evicts an entry that still has an outstanding guard, the
//! subprocess stays alive until the last `Arc` drops; the dropped
//! guard simply isn't re-poolable because its slot is gone. This is
//! the simplest correct semantic given that [`McpClient`] is itself
//! `Sync` and supports concurrent `call_tool` invocations.
use crate::client::McpClient;
use crate::error::McpResult;
use argo_types::TenantId;
use lru::LruCache;
use std::future::Future;
use std::num::NonZeroUsize;
use std::ops::Deref;
use std::sync::{Arc, Mutex, Weak};
use std::time::{Duration, Instant};

/// Default LRU capacity. Bounded so an idle worker can't accumulate
/// hundreds of FDs against rarely-used MCP servers.
pub const DEFAULT_POOL_MAX_CLIENTS: u32 = 32;
/// Default idle TTL — long enough to span turn-to-turn pauses, short
/// enough that abandoned sessions free their subprocesses.
pub const DEFAULT_POOL_IDLE_TTL_SECS: u64 = 600;

/// Operator-tunable pool knobs. Defaults match
/// [`DEFAULT_POOL_MAX_CLIENTS`] / [`DEFAULT_POOL_IDLE_TTL_SECS`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PoolConfig {
    /// Hard cap on cached clients. New acquisitions past this evict
    /// the LRU entry. `0` is treated as `1` so the cache always holds
    /// at least one slot.
    pub max_clients: u32,
    /// Seconds an entry can sit untouched before [`McpClientPool::evict_idle`]
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
/// correctness invariant — see module docs.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct PoolKey {
    /// Tenant scope.
    pub tenant_id: TenantId,
    /// Caller-chosen identifier (typically the MCP manifest id, but
    /// the pool treats it as opaque).
    pub manifest_id: String,
}

/// Snapshot of pool state for operator tooling.
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

struct PoolEntry {
    client: Arc<McpClient>,
    last_used: Instant,
}

/// Cache of warm [`McpClient`] subprocesses, keyed by
/// `(tenant_id, manifest_id)`. See module docs.
pub struct McpClientPool {
    inner: Mutex<LruCache<PoolKey, PoolEntry>>,
    idle_ttl: Duration,
}

impl std::fmt::Debug for McpClientPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let entries = self
            .inner
            .lock()
            .map(|c| c.len())
            .unwrap_or_else(|p| p.into_inner().len());
        f.debug_struct("McpClientPool")
            .field("entries", &entries)
            .field("idle_ttl_secs", &self.idle_ttl.as_secs())
            .finish()
    }
}

/// Borrow handle for a pooled [`McpClient`]. Deref'ing exposes the
/// underlying client; on drop the entry's `last_used` is bumped so
/// the LRU and idle-TTL reaper see fresh activity.
pub struct PoolGuard {
    client: Arc<McpClient>,
    pool: Weak<McpClientPool>,
    key: PoolKey,
}

impl std::fmt::Debug for PoolGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PoolGuard")
            .field("key", &self.key)
            .field("client", &self.client)
            .finish()
    }
}

impl Deref for PoolGuard {
    type Target = McpClient;
    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

impl PoolGuard {
    /// Borrow the underlying client as an `Arc`. Useful when callers
    /// want to hand the client to a task that outlives the guard.
    #[must_use]
    pub fn client_arc(&self) -> Arc<McpClient> {
        self.client.clone()
    }
}

impl Drop for PoolGuard {
    fn drop(&mut self) {
        let Some(pool) = self.pool.upgrade() else {
            return;
        };
        let mut cache = pool
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(entry) = cache.peek_mut(&self.key) {
            entry.last_used = Instant::now();
        }
    }
}

impl McpClientPool {
    /// Construct an empty pool.
    #[must_use]
    pub fn new(config: PoolConfig) -> Arc<Self> {
        let cap = NonZeroUsize::new(config.max_clients.max(1) as usize).unwrap_or(NonZeroUsize::MIN);
        Arc::new(Self {
            inner: Mutex::new(LruCache::new(cap)),
            idle_ttl: Duration::from_secs(config.idle_ttl_secs),
        })
    }

    /// Test-only constructor that accepts a sub-second idle TTL.
    /// Production code goes through [`Self::new`] which derives the
    /// TTL from `config.idle_ttl_secs`.
    #[doc(hidden)]
    #[must_use]
    pub fn new_with_idle_ttl(max_clients: u32, idle_ttl: Duration) -> Arc<Self> {
        let cap = NonZeroUsize::new(max_clients.max(1) as usize).unwrap_or(NonZeroUsize::MIN);
        Arc::new(Self {
            inner: Mutex::new(LruCache::new(cap)),
            idle_ttl,
        })
    }

    /// Acquire a guard for `(tenant_id, manifest_id)`. On cache miss,
    /// `spawn` is invoked to construct the client; the new client is
    /// installed in the cache (evicting the LRU entry if at capacity).
    ///
    /// Two acquisitions of the same key racing on a cold pool will
    /// each invoke `spawn`; the second installation supersedes the
    /// first and the loser's subprocess is best-effort shut down.
    /// This is acceptable for v1 (per the brief — the mutex is not
    /// the bottleneck).
    ///
    /// # Errors
    /// Returns whatever `spawn` returns. The pool itself does not
    /// produce errors.
    pub async fn acquire<F, Fut>(
        self: &Arc<Self>,
        tenant_id: TenantId,
        manifest_id: impl Into<String>,
        spawn: F,
    ) -> McpResult<PoolGuard>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = McpResult<McpClient>>,
    {
        let key = PoolKey {
            tenant_id,
            manifest_id: manifest_id.into(),
        };
        if let Some(guard) = self.try_hit(&key) {
            return Ok(guard);
        }
        let client = Arc::new(spawn().await?);
        Ok(self.install(key, client))
    }

    fn try_hit(self: &Arc<Self>, key: &PoolKey) -> Option<PoolGuard> {
        let mut cache = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let entry = cache.get_mut(key)?;
        entry.last_used = Instant::now();
        let client = entry.client.clone();
        Some(PoolGuard {
            client,
            pool: Arc::downgrade(self),
            key: key.clone(),
        })
    }

    fn install(self: &Arc<Self>, key: PoolKey, client: Arc<McpClient>) -> PoolGuard {
        let mut cache = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        // Race re-check: another acquire may have installed the same
        // key while we were spawning. If so, prefer the existing
        // entry and let the just-spawned client drop (its subprocess
        // is reaped via `kill_on_drop`).
        if let Some(existing) = cache.get_mut(&key) {
            existing.last_used = Instant::now();
            let existing_client = existing.client.clone();
            return PoolGuard {
                client: existing_client,
                pool: Arc::downgrade(self),
                key,
            };
        }
        let evicted: Option<PoolEntry> = if cache.len() >= cache.cap().get() {
            cache.pop_lru().map(|(_, entry)| entry)
        } else {
            None
        };
        cache.put(
            key.clone(),
            PoolEntry {
                client: client.clone(),
                last_used: Instant::now(),
            },
        );
        drop(cache);
        if let Some(old) = evicted {
            spawn_shutdown(old.client);
        }
        PoolGuard {
            client,
            pool: Arc::downgrade(self),
            key,
        }
    }

    /// Drop every entry whose `last_used` is older than
    /// [`PoolConfig::idle_ttl_secs`]; shut each evicted client down
    /// in-band so callers can `await` the reap. Safe to call on any
    /// schedule; an empty pool returns immediately.
    pub async fn evict_idle(&self) {
        let ttl = self.idle_ttl;
        let now = Instant::now();
        let evicted: Vec<PoolEntry> = {
            let mut cache = self
                .inner
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let stale_keys: Vec<PoolKey> = cache
                .iter()
                .filter_map(|(k, entry)| {
                    if now.duration_since(entry.last_used) >= ttl {
                        Some(k.clone())
                    } else {
                        None
                    }
                })
                .collect();
            stale_keys
                .into_iter()
                .filter_map(|k| cache.pop(&k))
                .collect()
        };
        for entry in evicted {
            let _ = entry.client.shutdown().await;
        }
    }

    /// Snapshot of the pool's current state. Cheap; takes the
    /// internal mutex briefly.
    #[must_use]
    pub fn stats(&self) -> PoolStats {
        let cache = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let now = Instant::now();
        let mut idles: Vec<u64> = cache
            .iter()
            .map(|(_, entry)| now.saturating_duration_since(entry.last_used).as_secs())
            .collect();
        idles.sort_unstable();
        let entries = idles.len();
        let idle_seconds_max = idles.last().copied().unwrap_or(0);
        let idle_seconds_p50 = if entries == 0 { 0 } else { idles[entries / 2] };
        PoolStats {
            entries,
            idle_seconds_p50,
            idle_seconds_max,
        }
    }
}

fn spawn_shutdown(client: Arc<McpClient>) {
    // `tokio::spawn` requires a runtime; eviction may run from a
    // non-tokio context (eg. operator tooling). Fall back to a
    // detached thread that builds a tiny runtime — sufficient for a
    // best-effort `kill()` since the child is also `kill_on_drop`.
    if tokio::runtime::Handle::try_current().is_ok() {
        tokio::spawn(async move {
            let _ = client.shutdown().await;
        });
    } else {
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build();
            if let Ok(rt) = rt {
                rt.block_on(async move {
                    let _ = client.shutdown().await;
                });
            }
        });
    }
}
