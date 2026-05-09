//! Per-route, per-tenant token-bucket rate limiter
//! (claude-rate-limit-per-route).
//!
//! Layered at the top of [`super::routes::dispatch_route`]. Each
//! `(route, tenant)` pair gets a token bucket with capacity = the
//! route's RPM (so a cold-start tenant can burst its full minute).
//! Refill is `tokens += elapsed_secs * (rpm / 60.0)`. An empty
//! bucket → `429 Too Many Requests` with body
//! `{"error":"rate_limited","retry_after_ms":<n>}`.
//!
//! This is **independent + additive** to the per-IP limiter in
//! [`super::rate_limit_ip`] (#1051): IP layer rejects floods from a
//! single source; this layer rejects per-tenant abuse of a hot route
//! across many IPs. It is also separate from
//! `claude-tenant-quota-enforcement-stricter`: quota = monthly-ish
//! business limit, returning `error: quota_exceeded`; this = per-minute
//! load shedding, returning `error: rate_limited`.
//!
//! # Defaults
//!
//! - All routes: 100 rpm/tenant.
//! - `forge.run` (POST /sessions/<id>/runs): 10 rpm/tenant.
//! - `skill.execute` (POST /skills/<name>/refine): 30 rpm/tenant.
//!
//! # Env knobs
//!
//! - `ARGO_RATE_LIMIT_DEFAULT_RPM=<n>` overrides the default rpm.
//! - `ARGO_RATE_LIMIT_<ROUTE>_RPM=<n>` overrides a specific route.
//!   Route key is uppercased and non-alphanumerics become `_`. So
//!   `forge.run` → `ARGO_RATE_LIMIT_FORGE_RUN_RPM`.
//!
//! # Sensible deviation from brief
//!
//! The brief asks for a `Retry-After` header on the 429 response. The
//! gateway frames responses in `boot.rs` (outside this brief's
//! may-touch list) with a fixed header set, and `dispatch_route`
//! returns only `(status_line, body)`. The retry interval is therefore
//! surfaced inside the JSON body (`retry_after_ms`) only. Adding a
//! true header requires a separate brief that touches `boot.rs`.

use dashmap::DashMap;
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tokio::time::Instant;

#[cfg(test)]
#[path = "rate_limit_tests.rs"]
mod tests;

pub(super) const DEFAULT_RPM: u32 = 100;
pub(super) const FORGE_RUN_RPM: u32 = 10;
pub(super) const SKILL_EXECUTE_RPM: u32 = 30;
pub(super) const EVICT_AGE: Duration = Duration::from_secs(10 * 60);
pub(super) const SWEEP_INTERVAL: Duration = Duration::from_secs(60);

pub(super) const ROUTE_FORGE_RUN: &str = "forge.run";
pub(super) const ROUTE_SKILL_EXECUTE: &str = "skill.execute";

/// Retry hint returned when a bucket is empty.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct RetryAfter {
    pub retry_after_ms: u64,
}

#[derive(Debug)]
pub(super) struct Bucket {
    tokens: f64,
    last_seen: Instant,
}

impl Bucket {
    fn new(max_tokens: u32, now: Instant) -> Self {
        Self {
            tokens: f64::from(max_tokens),
            last_seen: now,
        }
    }

    /// Refill lazily based on monotonic elapsed wall-time, then try to
    /// consume one token. `Ok(())` when consumed.
    fn try_take(&mut self, now: Instant, rpm: u32) -> Result<(), RetryAfter> {
        let cap = f64::from(rpm.max(1));
        let elapsed = now.saturating_duration_since(self.last_seen).as_secs_f64();
        self.tokens = (self.tokens + elapsed * (cap / 60.0)).min(cap);
        self.last_seen = now;
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            Ok(())
        } else {
            let needed = 1.0 - self.tokens;
            let tokens_per_sec = cap / 60.0;
            let wait_ms = ((needed / tokens_per_sec) * 1000.0).ceil();
            let retry_after_ms = if wait_ms >= u64::MAX as f64 {
                u64::MAX
            } else {
                (wait_ms as u64).max(1)
            };
            Err(RetryAfter { retry_after_ms })
        }
    }
}

/// In-memory `(route, tenant) → Bucket` store. Cheap to clone (`Arc`).
pub struct RateLimiter {
    buckets: DashMap<(String, String), Bucket>,
    default_rpm: u32,
    overrides: HashMap<String, u32>,
}

impl RateLimiter {
    /// Construct with explicit settings. Used by tests.
    pub fn new(default_rpm: u32, overrides: HashMap<String, u32>) -> Self {
        Self {
            buckets: DashMap::new(),
            default_rpm: default_rpm.max(1),
            overrides,
        }
    }

    /// Construct from `ARGO_RATE_LIMIT_*` env vars. Invalid values fall
    /// back to documented defaults rather than failing gateway start.
    pub fn from_env() -> Self {
        let default_rpm = read_env_rpm("ARGO_RATE_LIMIT_DEFAULT_RPM", DEFAULT_RPM);
        let mut overrides = HashMap::new();
        overrides.insert(
            ROUTE_FORGE_RUN.to_string(),
            read_env_rpm(&env_var_for_route(ROUTE_FORGE_RUN), FORGE_RUN_RPM),
        );
        overrides.insert(
            ROUTE_SKILL_EXECUTE.to_string(),
            read_env_rpm(&env_var_for_route(ROUTE_SKILL_EXECUTE), SKILL_EXECUTE_RPM),
        );
        Self::new(default_rpm, overrides)
    }

    /// Resolve the configured rpm for a route — override or default.
    pub fn rpm_for(&self, route: &str) -> u32 {
        self.overrides
            .get(route)
            .copied()
            .unwrap_or(self.default_rpm)
    }

    /// Try to consume one token for `(route, tenant)` at wall-clock now.
    pub fn acquire(&self, route: &str, tenant: &str) -> Result<(), RetryAfter> {
        self.acquire_at(route, tenant, Instant::now())
    }

    /// Test-friendly variant taking an explicit `Instant`.
    pub(super) fn acquire_at(
        &self,
        route: &str,
        tenant: &str,
        now: Instant,
    ) -> Result<(), RetryAfter> {
        let rpm = self.rpm_for(route);
        let key = (route.to_string(), tenant.to_string());
        let mut entry = self
            .buckets
            .entry(key)
            .or_insert_with(|| Bucket::new(rpm, now));
        entry.try_take(now, rpm)
    }

    /// Drop buckets idle for longer than [`EVICT_AGE`].
    pub(super) fn evict_idle(&self, now: Instant) {
        self.buckets
            .retain(|_, bucket| now.saturating_duration_since(bucket.last_seen) < EVICT_AGE);
    }

    /// Tracked `(route, tenant)` pairs. Used by tests.
    #[cfg(test)]
    pub(super) fn bucket_count(&self) -> usize {
        self.buckets.len()
    }
}

/// Spawn the periodic eviction sweep. Aborts when the join handle is
/// dropped — the gateway holds it for the duration of the process.
pub fn spawn_sweep_task(limiter: Arc<RateLimiter>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(SWEEP_INTERVAL);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        // First tick fires immediately; consume so the first eviction
        // is `SWEEP_INTERVAL` after spawn, not at t=0.
        ticker.tick().await;
        loop {
            ticker.tick().await;
            limiter.evict_idle(Instant::now());
        }
    })
}

static LIMITER: OnceLock<Arc<RateLimiter>> = OnceLock::new();

/// Initialise the process-wide limiter and spawn the sweep task.
/// Idempotent — only the first caller wins. Safe to call from any
/// async context with a tokio runtime in scope.
pub fn init() -> Arc<RateLimiter> {
    let mut spawned = false;
    let limiter = LIMITER
        .get_or_init(|| {
            spawned = true;
            Arc::new(RateLimiter::from_env())
        })
        .clone();
    if spawned {
        // Only spawn the sweep on the run that actually installed the
        // limiter — avoids stacked sweep tasks under racy initialisation.
        let _sweep = spawn_sweep_task(Arc::clone(&limiter));
    }
    limiter
}

/// Process-wide limiter handle, if [`init`] has run.
pub fn global() -> Option<Arc<RateLimiter>> {
    LIMITER.get().cloned()
}

/// Map a raw HTTP request to its route-rate-limit key.
///
/// Hot routes get stable aliases (`forge.run`, `skill.execute`); other
/// routes use a `<METHOD> /<path-template>` key with UUID segments
/// masked to `:id` so per-resource calls share a bucket.
pub fn classify_request(req: &str) -> Cow<'static, str> {
    let line = req.lines().next().unwrap_or("");
    let mut parts = line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let target = parts.next().unwrap_or("/");
    classify_route(method, target)
}

pub(super) fn classify_route(method: &str, target: &str) -> Cow<'static, str> {
    let path = target.split_once('?').map(|(p, _)| p).unwrap_or(target);
    let segments: Vec<&str> = path
        .trim_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();
    if method == "POST"
        && segments.len() == 3
        && segments[0] == "sessions"
        && segments[2] == "runs"
    {
        return Cow::Borrowed(ROUTE_FORGE_RUN);
    }
    if method == "POST"
        && segments.len() == 3
        && segments[0] == "skills"
        && segments[2] == "refine"
    {
        return Cow::Borrowed(ROUTE_SKILL_EXECUTE);
    }
    let templated = segments
        .iter()
        .map(|seg| {
            if uuid::Uuid::parse_str(seg).is_ok() {
                ":id"
            } else {
                seg
            }
        })
        .collect::<Vec<_>>()
        .join("/");
    Cow::Owned(format!("{method} /{templated}"))
}

/// Extract the bucket-key tenant from the request. Order: `?tenant=`
/// query param, `X-Tenant-Id` header, `ARGO_DEFAULT_TENANT` /
/// `ARGO_WEBINBOX_TENANT_ID` env var, then `"anonymous"`.
pub fn extract_tenant(req: &str) -> String {
    if let Some(target) = request_target(req) {
        if let Some((_, qs)) = target.split_once('?') {
            for kv in qs.split('&') {
                if let Some(("tenant", value)) = kv.split_once('=') {
                    if !value.is_empty() {
                        return value.to_string();
                    }
                }
            }
        }
    }
    if let Some(value) = header(req, "x-tenant-id") {
        if !value.is_empty() {
            return value.to_string();
        }
    }
    std::env::var("ARGO_DEFAULT_TENANT")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| {
            std::env::var("ARGO_WEBINBOX_TENANT_ID")
                .ok()
                .filter(|s| !s.is_empty())
        })
        .unwrap_or_else(|| "anonymous".to_string())
}

fn read_env_rpm(name: &str, default: u32) -> u32 {
    std::env::var(name)
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
        .filter(|n| *n > 0)
        .unwrap_or(default)
}

pub(super) fn env_var_for_route(route: &str) -> String {
    let mut buf = String::with_capacity(20 + route.len());
    buf.push_str("ARGO_RATE_LIMIT_");
    let mut prev_underscore = false;
    for ch in route.chars() {
        if ch.is_ascii_alphanumeric() {
            buf.push(ch.to_ascii_uppercase());
            prev_underscore = false;
        } else if !prev_underscore {
            buf.push('_');
            prev_underscore = true;
        }
    }
    if !buf.ends_with('_') {
        buf.push('_');
    }
    buf.push_str("RPM");
    buf
}

fn request_target(req: &str) -> Option<&str> {
    req.lines().next()?.split_whitespace().nth(1)
}

fn header<'a>(req: &'a str, key: &str) -> Option<&'a str> {
    let header_end = req.find("\r\n\r\n").unwrap_or(req.len());
    for line in req[..header_end].split("\r\n").skip(1) {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        if name.trim().eq_ignore_ascii_case(key) {
            return Some(value.trim());
        }
    }
    None
}
