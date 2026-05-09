use argo_types::{ArgoError, ArgoResult};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

const DEFAULT_CAPACITY: f64 = 60.0;
const DEFAULT_REFILL_RATE_PER_SEC: f64 = 10.0;
const DEFAULT_MAX_WAIT_MS: u64 = 30_000;
const CONFIG_ENV: &str = "ARGO_MCP_RATE_LIMIT_PATH";

#[derive(Debug)]
struct TokenBucket {
    server_id: String,
    capacity: f64,
    refill_rate_per_sec: f64,
    state: Mutex<BucketState>,
}

#[derive(Debug, Clone, Copy)]
struct BucketState {
    tokens: f64,
    last_refill: Instant,
}

impl TokenBucket {
    fn new(server_id: impl Into<String>, config: BucketConfig) -> Self {
        Self {
            server_id: server_id.into(),
            capacity: config.capacity.max(0.0),
            refill_rate_per_sec: config.refill_rate_per_sec.max(0.0),
            state: Mutex::new(BucketState {
                tokens: config.capacity.max(0.0),
                last_refill: Instant::now(),
            }),
        }
    }

    fn try_acquire(&self, n: f64) -> bool {
        if n <= 0.0 {
            return true;
        }
        let Ok(mut state) = self.state.lock() else {
            return false;
        };
        self.refill(&mut state, Instant::now());
        if state.tokens + f64::EPSILON < n {
            return false;
        }
        state.tokens -= n;
        true
    }

    async fn acquire_with_timeout(&self, n: f64, max_wait: Duration) -> ArgoResult<()> {
        let started = Instant::now();
        loop {
            if self.try_acquire(n) {
                self.trace("acquired", started.elapsed(), None);
                return Ok(());
            }
            let elapsed = started.elapsed();
            if elapsed >= max_wait {
                let err = format!("rate limit exceeded for {}", self.server_id);
                self.trace("timeout", elapsed, Some(&err));
                return Err(ArgoError::Provider(err));
            }
            tokio::time::sleep(self.next_sleep(n, max_wait - elapsed)).await;
        }
    }

    fn refill(&self, state: &mut BucketState, now: Instant) {
        let elapsed = now.saturating_duration_since(state.last_refill);
        state.last_refill = now;
        if self.refill_rate_per_sec <= 0.0 || elapsed.is_zero() {
            return;
        }
        let added = elapsed.as_secs_f64() * self.refill_rate_per_sec;
        state.tokens = (state.tokens + added).min(self.capacity);
    }

    fn next_sleep(&self, n: f64, remaining: Duration) -> Duration {
        let needed = self
            .state
            .lock()
            .ok()
            .map(|state| (n - state.tokens).max(0.0))
            .unwrap_or(n);
        if self.refill_rate_per_sec <= 0.0 {
            return remaining.min(Duration::from_millis(10));
        }
        let ms = ((needed / self.refill_rate_per_sec) * 1000.0).ceil();
        remaining.min(Duration::from_millis(ms.max(1.0) as u64))
    }

    fn trace(&self, decision: &'static str, waited: Duration, error: Option<&str>) {
        match error {
            Some(error) => tracing::info!(
                target: "argo",
                server_id = %self.server_id,
                decision,
                wait_ms = waited.as_millis() as u64,
                error,
                "mcp.rate_limit"
            ),
            None => tracing::info!(
                target: "argo",
                server_id = %self.server_id,
                decision,
                wait_ms = waited.as_millis() as u64,
                "mcp.rate_limit"
            ),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
struct BucketConfig {
    #[serde(default = "default_capacity")]
    capacity: f64,
    #[serde(default = "default_refill_rate")]
    refill_rate_per_sec: f64,
}

impl Default for BucketConfig {
    fn default() -> Self {
        Self {
            capacity: DEFAULT_CAPACITY,
            refill_rate_per_sec: DEFAULT_REFILL_RATE_PER_SEC,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct McpRateLimitConfig {
    #[serde(default = "default_capacity")]
    default_capacity: f64,
    #[serde(default = "default_refill_rate")]
    default_refill_rate_per_sec: f64,
    #[serde(default = "default_max_wait_ms")]
    max_wait_ms: u64,
    #[serde(default)]
    rate_limit: HashMap<String, BucketConfig>,
}

impl Default for McpRateLimitConfig {
    fn default() -> Self {
        Self {
            default_capacity: DEFAULT_CAPACITY,
            default_refill_rate_per_sec: DEFAULT_REFILL_RATE_PER_SEC,
            max_wait_ms: DEFAULT_MAX_WAIT_MS,
            rate_limit: HashMap::new(),
        }
    }
}

impl McpRateLimitConfig {
    fn load(path: &Path) -> ArgoResult<Self> {
        let body = std::fs::read_to_string(path).map_err(|e| {
            ArgoError::Provider(format!(
                "read MCP rate limit config {}: {e}",
                path.display()
            ))
        })?;
        toml::from_str(&body).map_err(|e| {
            ArgoError::Provider(format!(
                "parse MCP rate limit config {}: {e}",
                path.display()
            ))
        })
    }

    fn load_default() -> ArgoResult<Self> {
        let path = std::env::var(CONFIG_ENV)
            .ok()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("manifests/connectors/mcp_rate_limit.toml"));
        if path.exists() {
            Self::load(&path)
        } else {
            Ok(Self::default())
        }
    }

    fn default_bucket(&self) -> BucketConfig {
        BucketConfig {
            capacity: self.default_capacity,
            refill_rate_per_sec: self.default_refill_rate_per_sec,
        }
    }

    fn max_wait(&self) -> Duration {
        Duration::from_millis(self.max_wait_ms.max(1))
    }
}

#[derive(Debug)]
struct RateLimiter {
    buckets: RwLock<HashMap<String, Arc<TokenBucket>>>,
    configured: HashMap<String, BucketConfig>,
    default_bucket: BucketConfig,
    max_wait: Duration,
}

impl RateLimiter {
    fn from_config(config: &McpRateLimitConfig) -> Self {
        Self {
            buckets: RwLock::new(HashMap::new()),
            configured: config.rate_limit.clone(),
            default_bucket: config.default_bucket(),
            max_wait: config.max_wait(),
        }
    }

    async fn acquire_with_timeout(
        &self,
        server_id: &str,
        n: f64,
        max_wait: Duration,
    ) -> ArgoResult<()> {
        self.bucket(server_id)
            .await
            .acquire_with_timeout(n, max_wait)
            .await
    }

    async fn acquire(&self, server_id: &str, n: f64) -> ArgoResult<()> {
        self.acquire_with_timeout(server_id, n, self.max_wait).await
    }

    async fn bucket(&self, server_id: &str) -> Arc<TokenBucket> {
        if let Some(bucket) = self.buckets.read().await.get(server_id).cloned() {
            return bucket;
        }
        let mut buckets = self.buckets.write().await;
        buckets
            .entry(server_id.to_string())
            .or_insert_with(|| {
                let config = self
                    .configured
                    .get(server_id)
                    .copied()
                    .unwrap_or(self.default_bucket);
                Arc::new(TokenBucket::new(server_id, config))
            })
            .clone()
    }
}

fn default_limiter() -> &'static RateLimiter {
    static LIMITER: OnceLock<RateLimiter> = OnceLock::new();
    LIMITER.get_or_init(|| {
        let config = McpRateLimitConfig::load_default().unwrap_or_default();
        RateLimiter::from_config(&config)
    })
}

pub(crate) async fn acquire_for_dispatch(server_id: &str) -> ArgoResult<()> {
    default_limiter().acquire(server_id, 1.0).await
}

fn default_capacity() -> f64 {
    DEFAULT_CAPACITY
}

fn default_refill_rate() -> f64 {
    DEFAULT_REFILL_RATE_PER_SEC
}

fn default_max_wait_ms() -> u64 {
    DEFAULT_MAX_WAIT_MS
}

#[cfg(test)]
#[path = "mcp_rate_limit_tests.rs"]
mod tests;
