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
use crate::client::McpClient;
use crate::error::McpResult;
pub use crate::pool_types::{
    PoolConfig, PoolKey, PoolStats, DEFAULT_POOL_IDLE_TTL_SECS, DEFAULT_POOL_MAX_CLIENTS,
};
use crate::rate_limit::BucketRegistry;
use argo_types::TenantId;
use lru::LruCache;
use std::future::Future;
use std::num::NonZeroUsize;
use std::ops::Deref;
use std::sync::{Arc, Mutex, Weak};
use std::time::{Duration, Instant};

struct PoolEntry {
    client: Arc<McpClient>,
    last_used: Instant,
}

/// Cache of warm [`McpClient`] subprocesses, keyed by
/// `(tenant_id, manifest_id)`. See module docs.
pub struct McpClientPool {
    inner: Mutex<LruCache<PoolKey, PoolEntry>>,
    idle_ttl: Duration,
    rate_limiter: BucketRegistry,
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
///
/// # Examples
///
/// ```no_run
/// use argo_mcp::{McpClient, McpClientPool, PoolConfig};
/// use argo_types::TenantId;
/// # async fn doc() -> argo_mcp::McpResult<()> {
/// let pool = McpClientPool::new(PoolConfig::default());
/// let guard = pool
///     .acquire(TenantId::new(), "example-connector", || async {
///         McpClient::connect_stdio("example-server", &[]).await
///     })
///     .await?;
/// // Deref to the underlying client to issue a call.
/// let _tools = guard.list_tools().await?;
/// # Ok(()) }
/// ```
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
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use argo_mcp::{McpClient, McpClientPool, PoolConfig};
    /// use argo_types::TenantId;
    /// # async fn doc() -> argo_mcp::McpResult<()> {
    /// let pool = McpClientPool::new(PoolConfig::default());
    /// let guard = pool
    ///     .acquire(TenantId::new(), "example-connector", || async {
    ///         McpClient::connect_stdio("example-server", &[]).await
    ///     })
    ///     .await?;
    /// let client = guard.client_arc();
    /// tokio::spawn(async move {
    ///     let _ = client.list_tools().await;
    /// });
    /// # Ok(()) }
    /// ```
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
    ///
    /// # Examples
    ///
    /// ```
    /// use argo_mcp::{McpClientPool, PoolConfig};
    /// let pool = McpClientPool::new(PoolConfig { max_clients: 4, idle_ttl_secs: 60 });
    /// assert_eq!(pool.stats().entries, 0);
    /// ```
    #[must_use]
    pub fn new(config: PoolConfig) -> Arc<Self> {
        let cap =
            NonZeroUsize::new(config.max_clients.max(1) as usize).unwrap_or(NonZeroUsize::MIN);
        Arc::new(Self {
            inner: Mutex::new(LruCache::new(cap)),
            idle_ttl: Duration::from_secs(config.idle_ttl_secs),
            rate_limiter: BucketRegistry::new(),
        })
    }

    /// Test-only constructor that accepts a sub-second idle TTL.
    #[doc(hidden)]
    #[must_use]
    pub fn new_with_idle_ttl(max_clients: u32, idle_ttl: Duration) -> Arc<Self> {
        let cap = NonZeroUsize::new(max_clients.max(1) as usize).unwrap_or(NonZeroUsize::MIN);
        Arc::new(Self {
            inner: Mutex::new(LruCache::new(cap)),
            idle_ttl,
            rate_limiter: BucketRegistry::new(),
        })
    }

    /// Acquire a guard for `(tenant_id, manifest_id)`. On cache miss,
    /// `spawn` is invoked to construct the client; the new client is
    /// installed in the cache (evicting the LRU entry if at capacity).
    ///
    /// Concurrent cold misses may each spawn; install re-check keeps one.
    ///
    /// # Errors
    /// Returns whatever `spawn` returns. The pool itself does not
    /// produce errors.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use argo_mcp::{McpClient, McpClientPool, PoolConfig};
    /// use argo_types::TenantId;
    /// # async fn doc() -> argo_mcp::McpResult<()> {
    /// let pool = McpClientPool::new(PoolConfig::default());
    /// // Cold cache: the closure is invoked and the resulting client
    /// // is installed. A second `acquire` with the same key reuses it.
    /// let guard = pool
    ///     .acquire(TenantId::new(), "example-connector", || async {
    ///         McpClient::connect_stdio("example-server", &[]).await
    ///     })
    ///     .await?;
    /// drop(guard);
    /// # Ok(()) }
    /// ```
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
        self.rate_limiter.check(key.tenant_id, &key.manifest_id)?;
        if let Some(guard) = self.try_hit(&key) {
            return Ok(guard);
        }
        let client = Arc::new(spawn().await?);
        Ok(self.install(key, client))
    }

    /// Access the pool's token-bucket registry for operator overrides.
    #[must_use]
    pub fn rate_limiter(&self) -> &BucketRegistry {
        &self.rate_limiter
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
    ///
    /// # Examples
    ///
    /// ```
    /// use argo_mcp::{McpClientPool, PoolConfig};
    /// # async fn doc() {
    /// let pool = McpClientPool::new(PoolConfig::default());
    /// // No-op on an empty pool; safe to call from a cron tick.
    /// pool.evict_idle().await;
    /// assert_eq!(pool.stats().entries, 0);
    /// # }
    /// ```
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
    ///
    /// # Examples
    ///
    /// ```
    /// use argo_mcp::{McpClientPool, PoolConfig};
    /// let stats = McpClientPool::new(PoolConfig::default()).stats();
    /// assert_eq!(stats.entries, 0);
    /// ```
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
