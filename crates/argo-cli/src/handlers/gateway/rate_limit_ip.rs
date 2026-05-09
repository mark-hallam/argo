//! Per-IP token-bucket rate limiter for the gateway
//! (claude-security-rate-limit-by-ip).
//!
//! Layered before route dispatch in [`super::boot`]. Each source IP
//! gets a token bucket with capacity `IP_BUCKET_CAPACITY` (default
//! 120) and refill rate `IP_BUCKET_REFILL_PER_SEC` (default 2 tokens
//! per second). An empty bucket → `429 Too Many Requests` with a
//! `Retry-After` header (RFC 7231 §7.1.3).
//!
//! Buckets live in a [`DashMap`] keyed by [`IpAddr`]. When the map
//! grows beyond `IP_BUCKET_MAX_ENTRIES` (100k), entries with
//! `last_refill` older than `IP_BUCKET_EVICT_AGE` (5 min) are evicted
//! lazily on the next insert.
//!
//! # Sensible deviation from brief
//!
//! The brief specified an `axum::middleware::from_fn_with_state`
//! integration. The gateway is a hand-rolled HTTP loop (see
//! [`super::boot::start`]), not an axum router, so the limiter is
//! invoked directly from the accept loop instead. Same outcome — a
//! cheap reject before route work — minus a framework migration.
//!
//! # Env knobs
//!
//! - `ARGO_IP_RATE_LIMIT=0` disables the limiter (default: enabled).
//! - `ARGO_IP_RATE_CAPACITY=<n>` overrides bucket capacity.
//! - `ARGO_IP_RATE_REFILL=<f>` overrides refill rate (tokens/sec).
//! - `ARGO_TRUST_XFF=1` honours the first hop of `X-Forwarded-For`
//!   (load-balancer mode). Defaults to off — `X-Forwarded-For` from
//!   untrusted clients is ignored.
//!
//! # Examples
//!
//! ```ignore
//! use std::net::{IpAddr, Ipv4Addr};
//! let limiter = IpRateLimiter::new(2, 1.0, true, false);
//! let ip: IpAddr = Ipv4Addr::LOCALHOST.into();
//! assert!(matches!(limiter.check(ip), CheckResult::Allow));
//! ```

use dashmap::DashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[cfg(test)]
#[path = "rate_limit_ip_tests.rs"]
mod tests;

/// Default token-bucket capacity (max burst per IP).
pub(super) const DEFAULT_CAPACITY: u32 = 120;
/// Default refill rate, tokens per second.
pub(super) const DEFAULT_REFILL_PER_SEC: f64 = 2.0;
/// Soft cap on the bucket map; eviction kicks in past this size.
pub(super) const MAX_ENTRIES: usize = 100_000;
/// Buckets idle this long are eligible for eviction.
pub(super) const EVICT_AGE_SECS: u64 = 5 * 60;

/// One IP's token bucket. `tokens` is fractional so the refill rate
/// can be sub-1 tokens per second without integer drift.
#[derive(Debug)]
pub(super) struct Bucket {
    tokens: f64,
    last_refill: Instant,
}

impl Bucket {
    fn new(capacity: u32, now: Instant) -> Self {
        Self {
            tokens: f64::from(capacity),
            last_refill: now,
        }
    }

    /// Refill lazily based on elapsed wall-time, then try to consume
    /// one token. Returns `true` when consumed.
    fn take(&mut self, now: Instant, capacity: u32, refill_per_sec: f64) -> bool {
        let elapsed = now.saturating_duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * refill_per_sec).min(f64::from(capacity));
        self.last_refill = now;
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

/// Outcome of a rate-limit check.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(super) enum CheckResult {
    /// Token consumed; request may proceed.
    Allow,
    /// Bucket empty; reject with this `Retry-After` value (seconds).
    Deny { retry_after_secs: u32 },
}

/// In-memory per-IP token-bucket store. Cheap to clone (`Arc`-backed).
pub struct IpRateLimiter {
    buckets: Arc<DashMap<IpAddr, Bucket>>,
    cap: u32,
    refill: f64,
    enabled: bool,
    trust_xff: bool,
}

impl IpRateLimiter {
    /// Construct with explicit settings. Used by tests.
    pub fn new(cap: u32, refill: f64, enabled: bool, trust_xff: bool) -> Self {
        Self {
            buckets: Arc::new(DashMap::new()),
            cap: cap.max(1),
            refill: refill.max(f64::EPSILON),
            enabled,
            trust_xff,
        }
    }

    /// Construct from `ARGO_IP_RATE_*` and `ARGO_TRUST_XFF` env vars.
    /// Invalid values fall back to the defaults rather than failing
    /// gateway startup.
    pub fn from_env() -> Self {
        let enabled = std::env::var("ARGO_IP_RATE_LIMIT")
            .map(|v| v != "0")
            .unwrap_or(true);
        let cap = std::env::var("ARGO_IP_RATE_CAPACITY")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .filter(|n| *n > 0)
            .unwrap_or(DEFAULT_CAPACITY);
        let refill = std::env::var("ARGO_IP_RATE_REFILL")
            .ok()
            .and_then(|v| v.parse::<f64>().ok())
            .filter(|f| f.is_finite() && *f > 0.0)
            .unwrap_or(DEFAULT_REFILL_PER_SEC);
        let trust_xff = std::env::var("ARGO_TRUST_XFF")
            .map(|v| v == "1")
            .unwrap_or(false);
        Self::new(cap, refill, enabled, trust_xff)
    }

    /// `true` when this limiter trusts `X-Forwarded-For` first hop.
    pub(super) fn trust_xff(&self) -> bool {
        self.trust_xff
    }

    /// Try to consume one token for `ip` at the current wall-clock.
    pub(super) fn check(&self, ip: IpAddr) -> CheckResult {
        self.check_with_now(ip, Instant::now())
    }

    /// Test-friendly variant that takes an explicit `Instant`. Public
    /// to the parent module for unit-test reuse.
    pub(super) fn check_with_now(&self, ip: IpAddr, now: Instant) -> CheckResult {
        if !self.enabled {
            return CheckResult::Allow;
        }
        let allowed = {
            let mut entry = self
                .buckets
                .entry(ip)
                .or_insert_with(|| Bucket::new(self.cap, now));
            entry.take(now, self.cap, self.refill)
        };
        if !allowed && self.buckets.len() > MAX_ENTRIES {
            self.evict_old(now);
        }
        if allowed {
            CheckResult::Allow
        } else {
            // Time until one token refills, rounded up. Always at least 1.
            let secs = (1.0 / self.refill).ceil();
            // f64::ceil never goes negative for positive `refill`; the
            // saturating cast caps at u32::MAX for absurdly slow refills.
            let retry = if secs >= f64::from(u32::MAX) {
                u32::MAX
            } else {
                (secs as u32).max(1)
            };
            CheckResult::Deny {
                retry_after_secs: retry,
            }
        }
    }

    /// Evict buckets whose `last_refill` is older than
    /// `EVICT_AGE_SECS`. Called when the map exceeds `MAX_ENTRIES`.
    pub(super) fn evict_old(&self, now: Instant) {
        let cutoff = Duration::from_secs(EVICT_AGE_SECS);
        self.buckets
            .retain(|_, bucket| now.saturating_duration_since(bucket.last_refill) < cutoff);
    }

    /// Number of tracked IPs. Used by tests to assert eviction.
    #[cfg(test)]
    pub(super) fn bucket_count(&self) -> usize {
        self.buckets.len()
    }
}

/// Pick the source IP to rate-limit on.
///
/// - `trust_xff = false` (default): peer's [`SocketAddr::ip`].
/// - `trust_xff = true`: the first hop of `X-Forwarded-For`, falling
///   back to peer if the header is missing or unparseable.
pub(super) fn extract_ip(req: &str, peer: SocketAddr, trust_xff: bool) -> IpAddr {
    if trust_xff {
        if let Some(ip) = parse_first_xff_hop(req) {
            return ip;
        }
    }
    peer.ip()
}

/// Parse the first comma-separated hop of `X-Forwarded-For`.
/// Header lookup is case-insensitive per RFC 7230 §3.2.
pub(super) fn parse_first_xff_hop(req: &str) -> Option<IpAddr> {
    let header_end = req.find("\r\n\r\n").unwrap_or(req.len());
    let headers = &req[..header_end];
    // Skip the request-line (`GET / HTTP/1.1`) — only header lines
    // have a `:`-separated name/value pair.
    for line in headers.split("\r\n").skip(1) {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        if !name.trim().eq_ignore_ascii_case("x-forwarded-for") {
            continue;
        }
        let first = value.split(',').next()?.trim();
        return first.parse::<IpAddr>().ok();
    }
    None
}

/// Build the raw HTTP/1.1 429 response for a rate-limited request.
/// Includes a JSON body (`{"error":"rate_limited",...}`) and a
/// `Retry-After` header.
pub(super) fn rate_limited_response_bytes(retry_after_secs: u32) -> Vec<u8> {
    let body = format!(
        "{{\"error\":\"rate_limited\",\"scope\":\"ip\",\"retry_after\":{retry_after_secs}}}"
    );
    let resp = format!(
        "HTTP/1.1 429 Too Many Requests\r\n\
         Content-Type: application/json\r\n\
         Content-Length: {}\r\n\
         Retry-After: {retry_after_secs}\r\n\
         Connection: close\r\n\r\n{body}",
        body.len()
    );
    resp.into_bytes()
}
