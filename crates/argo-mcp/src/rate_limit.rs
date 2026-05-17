//! Per-(tenant, connector) MCP token buckets.
//!
//! Buckets are lazy and bounded by an LRU so a large tenant set does
//! not pre-allocate connector quota state. `McpClientPool` owns one
//! registry and checks it before spawning or reusing a warm client.

use crate::error::{McpError, McpResult};
use argo_types::TenantId;
use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Maximum active `(tenant, connector)` buckets kept in memory.
pub const DEFAULT_BUCKET_REGISTRY_CAPACITY: usize = 10_000;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct BucketKey {
    tenant_id: TenantId,
    connector: String,
}

#[derive(Debug, Clone, Copy)]
struct BucketLimit {
    sustained_per_minute: u32,
    burst: u32,
}

impl BucketLimit {
    fn new(sustained_per_minute: u32) -> Self {
        let sustained_per_minute = sustained_per_minute.max(1);
        Self {
            sustained_per_minute,
            burst: sustained_per_minute,
        }
    }

    fn default_for(connector: &str) -> Self {
        match connector {
            "gmail" | "slack" => Self::new(30),
            "hubspot" | "salesforce" => Self::new(60),
            "stripe" => Self::new(10),
            _ => Self::new(60),
        }
    }

    fn refill_per_second(self) -> f64 {
        f64::from(self.sustained_per_minute) / 60.0
    }

    fn retry_after_ms(self) -> u64 {
        ((1000.0 / self.refill_per_second()).ceil() as u64).max(1)
    }
}

#[derive(Debug)]
struct TokenBucket {
    limit: BucketLimit,
    tokens: f64,
    last_refill: Instant,
}

impl TokenBucket {
    fn new(limit: BucketLimit, now: Instant) -> Self {
        Self {
            limit,
            tokens: f64::from(limit.burst),
            last_refill: now,
        }
    }

    fn update_limit(&mut self, limit: BucketLimit) {
        self.limit = limit;
        self.tokens = f64::from(limit.burst);
        self.last_refill = Instant::now();
    }

    fn refill(&mut self, now: Instant) {
        let elapsed = now.saturating_duration_since(self.last_refill);
        self.last_refill = now;
        let added = elapsed.as_secs_f64() * self.limit.refill_per_second();
        self.tokens = (self.tokens + added).min(f64::from(self.limit.burst));
    }

    fn try_acquire(&mut self, now: Instant) -> Result<(), BucketLimit> {
        self.refill(now);
        if self.tokens + f64::EPSILON >= 1.0 {
            self.tokens -= 1.0;
            Ok(())
        } else {
            Err(self.limit)
        }
    }
}

#[derive(Debug)]
struct RegistryInner {
    buckets: Mutex<LruCache<BucketKey, TokenBucket>>,
    refill_task_started: AtomicBool,
}

/// Bounded token-bucket registry for MCP connector quota.
///
/// # Examples
///
/// ```
/// use argo_mcp::rate_limit::BucketRegistry;
/// let registry = BucketRegistry::new();
/// assert_eq!(registry.installed_bucket_count(), 0);
/// ```
#[derive(Debug, Clone)]
pub struct BucketRegistry {
    inner: Arc<RegistryInner>,
}

impl Default for BucketRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl BucketRegistry {
    /// Construct a registry with the default 10k active-pair cap.
    ///
    /// # Examples
    ///
    /// ```
    /// use argo_mcp::rate_limit::BucketRegistry;
    /// let registry = BucketRegistry::new();
    /// assert_eq!(registry.installed_bucket_count(), 0);
    /// ```
    #[must_use]
    pub fn new() -> Self {
        let registry = Self::with_capacity(DEFAULT_BUCKET_REGISTRY_CAPACITY);
        registry.spawn_refill_task();
        registry
    }

    pub(crate) fn with_capacity(capacity: usize) -> Self {
        let cap = NonZeroUsize::new(capacity.max(1)).unwrap_or(NonZeroUsize::MIN);
        Self {
            inner: Arc::new(RegistryInner {
                buckets: Mutex::new(LruCache::new(cap)),
                refill_task_started: AtomicBool::new(false),
            }),
        }
    }

    /// Install or replace a connector override for one tenant.
    ///
    /// The override is in calls per minute. It takes effect
    /// immediately for an existing bucket and is used as the burst
    /// size as well as the sustained one-minute rate.
    ///
    /// # Examples
    ///
    /// ```
    /// use argo_mcp::rate_limit::BucketRegistry;
    /// use argo_types::TenantId;
    /// let registry = BucketRegistry::new();
    /// registry.install_override(TenantId::new(), "gmail", 100);
    /// assert_eq!(registry.installed_bucket_count(), 1);
    /// ```
    pub fn install_override(
        &self,
        tenant_id: TenantId,
        connector: impl AsRef<str>,
        sustained_per_minute: u32,
    ) {
        let key = BucketKey {
            tenant_id,
            connector: normalize_connector(connector.as_ref()),
        };
        let limit = BucketLimit::new(sustained_per_minute);
        let mut buckets = self
            .inner
            .buckets
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(bucket) = buckets.get_mut(&key) {
            bucket.update_limit(limit);
        } else {
            buckets.put(key, TokenBucket::new(limit, Instant::now()));
        }
    }

    /// Install connector overrides loaded from an external store.
    ///
    /// # Examples
    ///
    /// ```
    /// use argo_mcp::rate_limit::BucketRegistry;
    /// use argo_types::TenantId;
    /// let tenant = TenantId::new();
    /// let registry = BucketRegistry::new();
    /// registry.install_overrides([(tenant, "slack", 30)]);
    /// assert_eq!(registry.installed_bucket_count(), 1);
    /// ```
    pub fn install_overrides<I, S>(&self, overrides: I)
    where
        I: IntoIterator<Item = (TenantId, S, u32)>,
        S: AsRef<str>,
    {
        for (tenant_id, connector, sustained_per_minute) in overrides {
            self.install_override(tenant_id, connector, sustained_per_minute);
        }
    }

    /// Number of buckets currently installed in the registry.
    ///
    /// # Examples
    ///
    /// ```
    /// use argo_mcp::rate_limit::BucketRegistry;
    /// let registry = BucketRegistry::new();
    /// assert_eq!(registry.installed_bucket_count(), 0);
    /// ```
    #[must_use]
    pub fn installed_bucket_count(&self) -> usize {
        self.inner
            .buckets
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .len()
    }

    pub(crate) fn check(&self, tenant_id: TenantId, connector: &str) -> McpResult<()> {
        let connector = normalize_connector(connector);
        let key = BucketKey {
            tenant_id,
            connector,
        };
        let now = Instant::now();
        let mut buckets = self
            .inner
            .buckets
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let bucket = buckets.get_or_insert_mut(key.clone(), || {
            TokenBucket::new(BucketLimit::default_for(&key.connector), now)
        });
        bucket
            .try_acquire(now)
            .map_err(|limit| McpError::RateLimited {
                retry_after_ms: limit.retry_after_ms(),
                limit: limit.sustained_per_minute,
            })
    }

    pub(crate) fn spawn_refill_task(&self) {
        if self.inner.refill_task_started.swap(true, Ordering::AcqRel) {
            return;
        }
        let inner = Arc::clone(&self.inner);
        let Ok(handle) = tokio::runtime::Handle::try_current() else {
            self.inner
                .refill_task_started
                .store(false, Ordering::Release);
            return;
        };
        handle.spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(1));
            loop {
                interval.tick().await;
                refill_all(&inner, Instant::now());
            }
        });
    }

    #[cfg(test)]
    pub(crate) fn refill_once_for_test(&self) {
        refill_all(&self.inner, Instant::now() + Duration::from_secs(1));
    }

    #[cfg(test)]
    pub(crate) fn len_for_test(&self) -> usize {
        self.installed_bucket_count()
    }

    #[cfg(test)]
    pub(crate) fn contains_for_test(&self, tenant_id: TenantId, connector: &str) -> bool {
        let key = BucketKey {
            tenant_id,
            connector: normalize_connector(connector),
        };
        self.inner
            .buckets
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .iter()
            .any(|(candidate, _)| candidate == &key)
    }
}

fn refill_all(inner: &RegistryInner, now: Instant) {
    let mut buckets = inner
        .buckets
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    for (_, bucket) in buckets.iter_mut() {
        bucket.refill(now);
    }
}

fn normalize_connector(connector: &str) -> String {
    let connector = connector.trim();
    if connector.is_empty() {
        "*".to_string()
    } else {
        connector.to_ascii_lowercase()
    }
}
