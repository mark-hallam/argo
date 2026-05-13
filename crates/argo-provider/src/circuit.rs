//! Per-provider circuit breaker.
//!
//! When a provider (Anthropic, OpenAI, Bedrock, …) degrades, every
//! in-flight request retrying it exhausts our pools before failing.
//! This breaker fails fast on the dead provider for a cooldown,
//! probes once on cooldown elapse, and resets when the probe
//! succeeds.
//!
//! State machine per provider id:
//!
//! - **Closed** — normal dispatch. `record_failure` increments a
//!   consecutive-failure counter; `record_success` resets it. After
//!   `failure_threshold` consecutive failures the breaker opens.
//! - **Open** — `try_acquire` returns `circuit open` immediately
//!   for the cooldown window. The first `try_acquire` after cooldown
//!   admits exactly one probe (HalfOpen).
//! - **HalfOpen** — concurrent calls are rejected as Open until the
//!   probe resolves. Probe success → Closed and cooldown reset to
//!   base; probe failure → Open with cooldown doubled (capped at
//!   `cooldown_max`).
//!
//! Failure classification is the caller's responsibility:
//! `record_failure` is for HTTP 5xx / timeout / connect-refused.
//! HTTP 4xx and 429 should call `record_ignored` (state unchanged).
//! See [`is_breaker_failure`] for the heuristic that pairs with the
//! transports' error formatting.
//!
//! ```
//! use argo_provider::circuit::{Breaker, BreakerConfig};
//! use std::time::Duration;
//! let breaker = Breaker::new(BreakerConfig {
//!     failure_threshold: 3,
//!     cooldown: Duration::from_secs(1),
//!     cooldown_max: Duration::from_secs(60),
//! });
//! breaker.try_acquire("anthropic").unwrap().record_success();
//! ```

use argo_types::{ArgoError, ArgoResult};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Default consecutive-failure threshold before tripping.
pub const DEFAULT_FAILURE_THRESHOLD: u32 = 5;
/// Default base cooldown when the breaker opens.
pub const DEFAULT_COOLDOWN_SECS: u64 = 30;
/// Default cap on the cooldown after repeated probe failures.
pub const DEFAULT_COOLDOWN_MAX_SECS: u64 = 300;

/// Tunable knobs.
#[derive(Debug, Clone, Copy)]
pub struct BreakerConfig {
    /// Consecutive failures that trip Closed → Open.
    pub failure_threshold: u32,
    /// Base cooldown the first time the breaker opens.
    pub cooldown: Duration,
    /// Hard cap on the cooldown after repeated probe failures.
    pub cooldown_max: Duration,
}

impl Default for BreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: DEFAULT_FAILURE_THRESHOLD,
            cooldown: Duration::from_secs(DEFAULT_COOLDOWN_SECS),
            cooldown_max: Duration::from_secs(DEFAULT_COOLDOWN_MAX_SECS),
        }
    }
}

impl BreakerConfig {
    /// Read config from `ARGO_CB_FAILURE_THRESHOLD`, `ARGO_CB_COOLDOWN_SECS`,
    /// `ARGO_CB_COOLDOWN_MAX_SECS`. Invalid / zero values fall back to
    /// the defaults; `cooldown_max` is at least `cooldown`.
    #[must_use]
    pub fn from_env() -> Self {
        let threshold =
            parse_env_u32("ARGO_CB_FAILURE_THRESHOLD").unwrap_or(DEFAULT_FAILURE_THRESHOLD);
        let cooldown = parse_env_u64("ARGO_CB_COOLDOWN_SECS").unwrap_or(DEFAULT_COOLDOWN_SECS);
        let cooldown_max = parse_env_u64("ARGO_CB_COOLDOWN_MAX_SECS")
            .unwrap_or(DEFAULT_COOLDOWN_MAX_SECS)
            .max(cooldown);
        Self {
            failure_threshold: threshold,
            cooldown: Duration::from_secs(cooldown),
            cooldown_max: Duration::from_secs(cooldown_max),
        }
    }
}

fn parse_env_u32(key: &str) -> Option<u32> {
    std::env::var(key)
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
        .filter(|n| *n > 0)
}

fn parse_env_u64(key: &str) -> Option<u64> {
    std::env::var(key)
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .filter(|n| *n > 0)
}

/// Public state label.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Normal dispatch.
    Closed,
    /// Failing fast.
    Open,
    /// One probe in flight.
    HalfOpen,
}

#[derive(Debug)]
struct ProviderEntry {
    state: CircuitState,
    consecutive_failures: u32,
    cooldown: Duration,
    open_until: Option<Instant>,
    probe_in_flight: bool,
}

impl ProviderEntry {
    fn new(base_cooldown: Duration) -> Self {
        Self {
            state: CircuitState::Closed,
            consecutive_failures: 0,
            cooldown: base_cooldown,
            open_until: None,
            probe_in_flight: false,
        }
    }
}

/// Per-provider breaker. Cheap to clone (internally `Arc`-shared).
#[derive(Debug, Clone)]
pub struct Breaker {
    inner: Arc<BreakerInner>,
}

#[derive(Debug)]
struct BreakerInner {
    config: BreakerConfig,
    states: Mutex<HashMap<String, ProviderEntry>>,
}

impl Breaker {
    /// Build a breaker with explicit config.
    ///
    /// ```
    /// use argo_provider::circuit::{Breaker, BreakerConfig};
    /// let _ = Breaker::new(BreakerConfig::default());
    /// ```
    #[must_use]
    pub fn new(config: BreakerConfig) -> Self {
        Self {
            inner: Arc::new(BreakerInner {
                config,
                states: Mutex::new(HashMap::new()),
            }),
        }
    }

    /// Build a breaker with config from env (`ARGO_CB_*`).
    #[must_use]
    pub fn from_env() -> Self {
        Self::new(BreakerConfig::from_env())
    }

    /// Snapshot of the breaker's config.
    #[must_use]
    pub fn config(&self) -> BreakerConfig {
        self.inner.config
    }

    /// Public state for tests / observability.
    #[must_use]
    pub fn state_of(&self, provider_id: &str) -> CircuitState {
        self.inner
            .states
            .lock()
            .get(provider_id)
            .map_or(CircuitState::Closed, |e| e.state)
    }

    /// Try to acquire dispatch permission for `provider_id`.
    ///
    /// Returns [`Permit`] when admitted, or [`ArgoError::Provider`]
    /// tagged `circuit open: …` when the breaker is Open or has a
    /// probe in flight.
    pub fn try_acquire(&self, provider_id: &str) -> ArgoResult<Permit> {
        self.try_acquire_at(provider_id, Instant::now())
    }

    pub(crate) fn try_acquire_at(&self, provider_id: &str, now: Instant) -> ArgoResult<Permit> {
        let cfg = self.inner.config;
        let mut guard = self.inner.states.lock();
        let entry = guard
            .entry(provider_id.to_string())
            .or_insert_with(|| ProviderEntry::new(cfg.cooldown));
        match entry.state {
            CircuitState::Closed => Ok(Permit::new(self.clone(), provider_id, false)),
            CircuitState::Open => {
                let elapsed = entry.open_until.is_none_or(|until| now >= until);
                if elapsed && !entry.probe_in_flight {
                    entry.state = CircuitState::HalfOpen;
                    entry.probe_in_flight = true;
                    Ok(Permit::new(self.clone(), provider_id, true))
                } else {
                    Err(circuit_open_error(provider_id))
                }
            }
            CircuitState::HalfOpen => Err(circuit_open_error(provider_id)),
        }
    }

    fn record_success(&self, provider_id: &str, was_probe: bool) {
        let cfg = self.inner.config;
        let mut guard = self.inner.states.lock();
        let entry = guard
            .entry(provider_id.to_string())
            .or_insert_with(|| ProviderEntry::new(cfg.cooldown));
        entry.consecutive_failures = 0;
        entry.state = CircuitState::Closed;
        entry.open_until = None;
        entry.cooldown = cfg.cooldown;
        if was_probe {
            entry.probe_in_flight = false;
        }
    }

    fn record_failure(&self, provider_id: &str, was_probe: bool, now: Instant) {
        let cfg = self.inner.config;
        let mut guard = self.inner.states.lock();
        let entry = guard
            .entry(provider_id.to_string())
            .or_insert_with(|| ProviderEntry::new(cfg.cooldown));
        if was_probe {
            entry.probe_in_flight = false;
            let next = entry.cooldown.saturating_mul(2);
            entry.cooldown = next.min(cfg.cooldown_max);
            entry.state = CircuitState::Open;
            entry.open_until = Some(now + entry.cooldown);
            return;
        }
        entry.consecutive_failures = entry.consecutive_failures.saturating_add(1);
        if entry.consecutive_failures >= cfg.failure_threshold {
            entry.state = CircuitState::Open;
            entry.cooldown = cfg.cooldown;
            entry.open_until = Some(now + entry.cooldown);
        }
    }

    fn release_probe(&self, provider_id: &str) {
        let mut guard = self.inner.states.lock();
        if let Some(entry) = guard.get_mut(provider_id) {
            entry.probe_in_flight = false;
        }
    }
}

/// Permission to make one provider call. Outcome must be reported via
/// [`Permit::record_success`], [`Permit::record_failure`], or
/// [`Permit::record_ignored`]. A probe permit dropped without a record
/// auto-reports as failure so the half-open slot does not wedge.
#[must_use = "a Permit without an outcome record leaves the half-open slot held"]
#[derive(Debug)]
pub struct Permit {
    breaker: Breaker,
    provider_id: String,
    is_probe: bool,
    consumed: bool,
}

impl Permit {
    fn new(breaker: Breaker, provider_id: &str, is_probe: bool) -> Self {
        Self {
            breaker,
            provider_id: provider_id.to_string(),
            is_probe,
            consumed: false,
        }
    }

    /// Was this admitted as the half-open probe?
    #[must_use]
    pub fn is_probe(&self) -> bool {
        self.is_probe
    }

    /// Record a successful call.
    pub fn record_success(mut self) {
        self.breaker
            .record_success(&self.provider_id, self.is_probe);
        self.consumed = true;
    }

    /// Record a breaker-relevant failure (HTTP 5xx / timeout / connect refused).
    pub fn record_failure(self) {
        self.record_failure_at(Instant::now());
    }

    pub(crate) fn record_failure_at(mut self, now: Instant) {
        self.breaker
            .record_failure(&self.provider_id, self.is_probe, now);
        self.consumed = true;
    }

    /// Record an outcome the breaker should ignore (HTTP 4xx, 429, cancel).
    /// State unchanged; for a probe, the half-open slot is released.
    pub fn record_ignored(mut self) {
        if self.is_probe {
            self.breaker.release_probe(&self.provider_id);
        }
        self.consumed = true;
    }
}

impl Drop for Permit {
    fn drop(&mut self) {
        if self.consumed || !self.is_probe {
            return;
        }
        self.breaker
            .record_failure(&self.provider_id, true, Instant::now());
    }
}

fn circuit_open_error(provider_id: &str) -> ArgoError {
    ArgoError::Provider(format!("circuit open for provider {provider_id}"))
}

/// Heuristic classification of an [`ArgoError`] from a provider call.
///
/// Returns `true` for breaker-relevant failures (HTTP 5xx, network-layer
/// errors); `false` for HTTP 4xx / 429 and any non-provider variant.
/// Transports format errors as `"<id> HTTP {status}: …"` for HTTP and
/// `"reqwest send: …"` / `"reqwest send (stream): …"` for socket-level
/// failures; we parse those shapes. Unrecognised messages fall through
/// to `false` — don't trip on uncertainty.
#[must_use]
pub fn is_breaker_failure(err: &ArgoError) -> bool {
    let msg = match err {
        ArgoError::Provider(m) => m.as_str(),
        _ => return false,
    };
    if let Some(status) = parse_http_status(msg) {
        return (500..600).contains(&status);
    }
    msg.starts_with("reqwest send")
}

fn parse_http_status(msg: &str) -> Option<u16> {
    let i = msg.find(" HTTP ")?;
    let rest = &msg[i + " HTTP ".len()..];
    let end = rest
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(rest.len());
    rest[..end].parse::<u16>().ok()
}

#[cfg(test)]
#[path = "circuit_tests.rs"]
mod tests;
