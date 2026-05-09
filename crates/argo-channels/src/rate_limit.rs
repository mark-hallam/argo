//! Per-channel approval rate limiting via a token-bucket trio.
//!
//! Each [`NotifyChannel`](crate::NotifyChannel) impl wraps its
//! `send` call in [`RateLimit::try_acquire`]. When any of the
//! per-second / per-minute / per-hour windows is exhausted, the
//! channel returns
//! [`ChannelError::RateLimited`](crate::ChannelError::RateLimited)
//! with the longest wait across exhausted windows. The notification
//! dispatcher in `worker_actions_notify.rs` honours that wait as
//! backoff before its next attempt (capped at 60s).
//!
//! ## Default budgets per channel (brief V1.4)
//!
//! | channel  | per_sec | per_min | per_hour |
//! |----------|---------|---------|----------|
//! | slack    | 5       | 60      | —        |
//! | sms      | 1       | 10      | —        |
//! | email    | 5       | 30      | —        |
//! | push     | 5       | 100     | —        |
//! | voice    | —       | 1       | 5        |
//! | imessage | 5       | 60      | —        |
//! | whatsapp | 5       | 30      | —        |
//! | webinbox | unlimited (opts out) |         |
//!
//! Operators override these via the channel config block —
//! e.g. `[rate_limit] per_sec = 20` raises just the per-second
//! window while keeping per-minute / per-hour at the channel
//! default. Missing fields preserve the default; explicit values
//! tighten or relax the corresponding window.
//!
//! Cloning a [`RateLimit`] clones the underlying
//! `Arc<Mutex<...>>`, so every clone shares the same bucket
//! state — necessary because the channel struct is itself cloned
//! per request in some call sites.

use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Operator-tunable per-channel budget. `None` on a window means
/// that window is unlimited (the bucket is omitted entirely).
///
/// All three windows are evaluated independently; the limit fires
/// when any one of them is exhausted, and the returned `Duration`
/// is the longest wait across the exhausted windows so that one
/// retry will satisfy every constraint.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct RateLimitConfig {
    /// Tokens replenished per second.
    pub per_sec: Option<u32>,
    /// Tokens replenished per minute.
    pub per_min: Option<u32>,
    /// Tokens replenished per hour.
    pub per_hour: Option<u32>,
}

impl RateLimitConfig {
    /// All three windows unlimited.
    #[must_use]
    pub const fn unlimited() -> Self {
        Self {
            per_sec: None,
            per_min: None,
            per_hour: None,
        }
    }

    /// Build a config explicitly. `0` is treated as "no tokens",
    /// not "unlimited" — use [`Self::unlimited`] or `None` fields
    /// for that.
    #[must_use]
    pub const fn new(per_sec: Option<u32>, per_min: Option<u32>, per_hour: Option<u32>) -> Self {
        Self {
            per_sec,
            per_min,
            per_hour,
        }
    }

    /// Slack default: 5/sec, 60/min.
    #[must_use]
    pub const fn slack_default() -> Self {
        Self::new(Some(5), Some(60), None)
    }

    /// SMS default: 1/sec, 10/min.
    #[must_use]
    pub const fn sms_default() -> Self {
        Self::new(Some(1), Some(10), None)
    }

    /// Email default: 5/sec, 30/min.
    #[must_use]
    pub const fn email_default() -> Self {
        Self::new(Some(5), Some(30), None)
    }

    /// Push default: 5/sec, 100/min.
    #[must_use]
    pub const fn push_default() -> Self {
        Self::new(Some(5), Some(100), None)
    }

    /// Voice default: 1/min, 5/hour. No per-second limit because
    /// outbound calls are inherently slow to place.
    #[must_use]
    pub const fn voice_default() -> Self {
        Self::new(None, Some(1), Some(5))
    }

    /// iMessage default: 5/sec, 60/min.
    #[must_use]
    pub const fn imessage_default() -> Self {
        Self::new(Some(5), Some(60), None)
    }

    /// WhatsApp default: 5/sec, 30/min.
    #[must_use]
    pub const fn whatsapp_default() -> Self {
        Self::new(Some(5), Some(30), None)
    }

    /// Apply a partial operator override on top of channel
    /// defaults. Each `Some` field in `override_` replaces the
    /// matching field in `self`; `None` keeps the default.
    #[must_use]
    pub fn with_override(self, override_: Self) -> Self {
        Self {
            per_sec: override_.per_sec.or(self.per_sec),
            per_min: override_.per_min.or(self.per_min),
            per_hour: override_.per_hour.or(self.per_hour),
        }
    }
}

#[derive(Debug)]
struct TokenBucket {
    capacity: f64,
    refill_per_sec: f64,
    tokens: f64,
    last_refill: Instant,
}

impl TokenBucket {
    fn new(capacity: f64, refill_per_sec: f64) -> Self {
        Self {
            capacity,
            refill_per_sec,
            tokens: capacity,
            last_refill: Instant::now(),
        }
    }

    fn refill(&mut self, now: Instant) {
        let elapsed = now.saturating_duration_since(self.last_refill).as_secs_f64();
        if elapsed > 0.0 && self.refill_per_sec > 0.0 {
            self.tokens = (self.tokens + elapsed * self.refill_per_sec).min(self.capacity);
        }
        self.last_refill = now;
    }

    fn shortfall_wait(&self) -> Option<Duration> {
        if self.tokens + f64::EPSILON >= 1.0 {
            return None;
        }
        if self.refill_per_sec <= 0.0 {
            return Some(Duration::from_secs(60 * 60));
        }
        let needed = (1.0 - self.tokens).max(0.0);
        Some(Duration::from_secs_f64(needed / self.refill_per_sec))
    }
}

#[derive(Debug)]
struct RateLimitInner {
    per_sec: Option<TokenBucket>,
    per_min: Option<TokenBucket>,
    per_hour: Option<TokenBucket>,
}

/// Triple-window token-bucket limiter. Cheap to clone — all
/// clones share the same underlying state.
#[derive(Debug, Clone)]
pub struct RateLimit {
    inner: Arc<Mutex<RateLimitInner>>,
}

impl RateLimit {
    /// Build a limiter from a config. `None` windows are omitted
    /// from the bucket trio entirely.
    #[must_use]
    pub fn from_config(config: RateLimitConfig) -> Self {
        let inner = RateLimitInner {
            per_sec: config
                .per_sec
                .map(|n| TokenBucket::new(f64::from(n), f64::from(n))),
            per_min: config
                .per_min
                .map(|n| TokenBucket::new(f64::from(n), f64::from(n) / 60.0)),
            per_hour: config
                .per_hour
                .map(|n| TokenBucket::new(f64::from(n), f64::from(n) / 3600.0)),
        };
        Self {
            inner: Arc::new(Mutex::new(inner)),
        }
    }

    /// All three windows unlimited (the limiter is a no-op).
    #[must_use]
    pub fn unlimited() -> Self {
        Self::from_config(RateLimitConfig::unlimited())
    }

    /// Try to consume one token from every configured window.
    /// On success, all configured windows have been decremented
    /// atomically. On failure, returns the longest wait across
    /// the exhausted windows; nothing is decremented.
    pub fn try_acquire(&self) -> Result<(), Duration> {
        let mut guard = match self.inner.lock() {
            Ok(g) => g,
            Err(poison) => poison.into_inner(),
        };
        let now = Instant::now();
        let inner = &mut *guard;
        for slot in [
            inner.per_sec.as_mut(),
            inner.per_min.as_mut(),
            inner.per_hour.as_mut(),
        ]
        .into_iter()
        .flatten()
        {
            slot.refill(now);
        }
        let mut max_wait: Option<Duration> = None;
        for slot in [
            inner.per_sec.as_ref(),
            inner.per_min.as_ref(),
            inner.per_hour.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            if let Some(d) = slot.shortfall_wait() {
                max_wait = Some(max_wait.map_or(d, |cur| cur.max(d)));
            }
        }
        if let Some(d) = max_wait {
            return Err(d);
        }
        for slot in [
            inner.per_sec.as_mut(),
            inner.per_min.as_mut(),
            inner.per_hour.as_mut(),
        ]
        .into_iter()
        .flatten()
        {
            slot.tokens -= 1.0;
        }
        Ok(())
    }
}

impl Default for RateLimit {
    fn default() -> Self {
        Self::unlimited()
    }
}

#[cfg(test)]
#[path = "rate_limit_tests.rs"]
mod tests;
