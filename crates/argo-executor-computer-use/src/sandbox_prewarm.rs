//! Burst pre-warming for the sandbox pool.
//!
//! E2B sandbox creation has a cold-start cost (~10-15s). When a
//! vertical fires a burst of sandboxed actions, each cold-start
//! chains and adds latency to every action in the burst. The
//! [`SandboxPrewarmer`] keeps a small reservoir of pre-opened
//! sandboxes during high-activity windows so the next acquire
//! returns instantly.
//!
//! Two triggers drive pre-warming:
//!
//! 1. **Sliding-window spawn rate.** Every demand event
//!    ([`SandboxPrewarmer::record_spawn`]) lands in a 5-min
//!    sliding window. When the count in the window crosses the
//!    operator-configured threshold (default `3`), the prewarmer
//!    signals that `burst_size` extra sandboxes (default `2`)
//!    should be opened.
//! 2. **Hourly tick during operator-active hours.** Once per
//!    active hour the prewarmer signals that one sandbox should
//!    be opened. Active hours are the inverse of the operator's
//!    quiet-hours config and are supplied via
//!    [`SandboxPrewarmConfig::active_hours`].
//!
//! Both triggers are pure functions of timestamps + reservoir
//! state. The async I/O (calling E2B's `open_sandbox` and stamping
//! the result into the reservoir) lives at the call site (see
//! [`crate::pool::SandboxPool`]). The split keeps this module
//! offline-testable; the unit tests cover every threshold edge
//! without a live E2B endpoint.
//!
//! ## Audit emission
//!
//! Pre-warm + consume events are emitted via `tracing::info!` with
//! a `kind` field — `runtime.sandbox_prewarmed` and
//! `runtime.sandbox_prewarm_consumed` respectively. Operators tail
//! these via the structured-log pipeline already wired for other
//! `runtime.*` audit kinds (e.g. `runtime.action_throttled`).
//!
//! ## Disabled by default
//!
//! [`SandboxPrewarmConfig::enabled`] is `false` by default. The
//! pool builder skips construction of the prewarmer entirely when
//! the config is off — the dispatch path is zero-overhead until an
//! operator opts in. Pre-warming consumes E2B-minutes even when
//! the reservoir goes unused, so the default respects operator
//! wallet.

use crate::e2b::E2bSandbox;
use chrono::{DateTime, Timelike, Utc};
use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Default sliding window for spawn-rate detection (5 minutes).
pub const DEFAULT_WINDOW_SECONDS: u64 = 300;
/// Default spawn count above which a burst pre-warm fires.
pub const DEFAULT_SPAWN_RATE_THRESHOLD: u32 = 3;
/// Default extra sandboxes opened per burst trigger.
pub const DEFAULT_BURST_SIZE: u32 = 2;
/// Default cap on the pre-warm reservoir.
pub const DEFAULT_MAX_PREWARM: u32 = 5;
/// Default idle TTL after which a pre-warmed sandbox is reaped.
pub const DEFAULT_IDLE_TTL_SECONDS: u64 = 600;

/// Operator-tunable knobs. Mirrors the values consumed off
/// `[runtime.sandbox_prewarm]` by [`argo_config::SandboxPrewarmConfig`].
/// Kept as a standalone struct here so the executor crate doesn't
/// take a build-time dependency on `argo-config` just for the
/// types; the config crate maps its TOML struct into this one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxPrewarmConfig {
    /// Master enable. When `false`, the prewarmer is not
    /// constructed at all (no overhead).
    pub enabled: bool,
    /// Spawn-events-per-window above which a burst fires.
    pub spawn_rate_threshold: u32,
    /// Sliding-window width.
    pub window_seconds: u64,
    /// Extra sandboxes opened on each burst trigger.
    pub burst_size: u32,
    /// Reservoir cap. Excess pre-warmed sandboxes are closed by
    /// the caller (see [`SandboxPrewarmer::try_admit`]).
    pub max_prewarm: u32,
    /// Pre-warmed sandbox idle TTL. Past this, the reaper drops
    /// the entry (caller closes).
    pub idle_ttl_seconds: u64,
    /// Operator-active hours (0..=23). Empty means "always
    /// active" — the hourly trigger fires every hour. Non-empty
    /// restricts the trigger to the listed hours, inclusive.
    pub active_hours: Vec<u8>,
}

impl Default for SandboxPrewarmConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            spawn_rate_threshold: DEFAULT_SPAWN_RATE_THRESHOLD,
            window_seconds: DEFAULT_WINDOW_SECONDS,
            burst_size: DEFAULT_BURST_SIZE,
            max_prewarm: DEFAULT_MAX_PREWARM,
            idle_ttl_seconds: DEFAULT_IDLE_TTL_SECONDS,
            active_hours: Vec::new(),
        }
    }
}

/// One entry in the pre-warm reservoir.
#[derive(Debug, Clone)]
pub struct PrewarmedSandbox {
    /// Template the sandbox was opened from. Consume is keyed
    /// off this so a pre-warmed `marketplace_browser` is never
    /// handed to a `customer_support` profile.
    pub template: String,
    /// The opened sandbox handle.
    pub sandbox: E2bSandbox,
    /// Wallclock the sandbox was admitted to the reservoir.
    /// Idle TTL is computed from this.
    pub prewarmed_at: Instant,
}

/// In-process pre-warm reservoir + decision logic.
///
/// Cloneable shapes (wrapped in `Arc<Mutex<…>>`) live one level
/// up; see [`crate::pool::SandboxPool::with_prewarmer`].
#[derive(Debug)]
pub struct SandboxPrewarmer {
    config: SandboxPrewarmConfig,
    reservoir: VecDeque<PrewarmedSandbox>,
    spawn_events: VecDeque<Instant>,
    last_hourly_hour: Option<u32>,
}

impl SandboxPrewarmer {
    /// Build a prewarmer with the given config.
    #[must_use]
    pub fn new(config: SandboxPrewarmConfig) -> Self {
        Self {
            config,
            reservoir: VecDeque::new(),
            spawn_events: VecDeque::new(),
            last_hourly_hour: None,
        }
    }

    /// Read-only view of the live config.
    #[must_use]
    pub fn config(&self) -> &SandboxPrewarmConfig {
        &self.config
    }

    /// Current reservoir occupancy.
    #[must_use]
    pub fn occupancy(&self) -> usize {
        self.reservoir.len()
    }

    /// Record a spawn-demand event. Evicts events older than the
    /// configured window so [`Self::spawn_count_in_window`] only
    /// sees recent demand.
    pub fn record_spawn(&mut self, now: Instant) {
        let window = Duration::from_secs(self.config.window_seconds);
        self.spawn_events.push_back(now);
        while let Some(&oldest) = self.spawn_events.front() {
            if now.duration_since(oldest) > window {
                self.spawn_events.pop_front();
            } else {
                break;
            }
        }
    }

    /// Count of spawn events within the sliding window relative
    /// to `now`. Same eviction rule as [`Self::record_spawn`].
    #[must_use]
    pub fn spawn_count_in_window(&self, now: Instant) -> usize {
        let window = Duration::from_secs(self.config.window_seconds);
        self.spawn_events
            .iter()
            .filter(|t| now.duration_since(**t) <= window)
            .count()
    }

    /// Returns `burst_size` when the in-window spawn count
    /// strictly exceeds `spawn_rate_threshold` AND the reservoir
    /// has room. Zero otherwise.
    ///
    /// "Strictly exceeds" mirrors the brief: with threshold `3`,
    /// 3 spawns/window does NOT trigger; 4 does. Operators who
    /// want aggressive pre-warming lower the threshold.
    #[must_use]
    pub fn should_burst_prewarm(&self, now: Instant) -> u32 {
        if !self.config.enabled {
            return 0;
        }
        let count = u32::try_from(self.spawn_count_in_window(now)).unwrap_or(u32::MAX);
        if count <= self.config.spawn_rate_threshold {
            return 0;
        }
        let occupancy = u32::try_from(self.reservoir.len()).unwrap_or(u32::MAX);
        let headroom = self.config.max_prewarm.saturating_sub(occupancy);
        headroom.min(self.config.burst_size)
    }

    /// Returns `true` exactly once per active hour, on the first
    /// call whose `now.hour()` matches an entry in `active_hours`
    /// and differs from the last-fired hour. Empty `active_hours`
    /// is treated as "every hour is active". State is kept in
    /// `last_hourly_hour` so subsequent calls in the same hour
    /// return `false`.
    pub fn should_hourly_prewarm(&mut self, now: DateTime<Utc>) -> bool {
        if !self.config.enabled {
            return false;
        }
        if self.reservoir.len() >= self.config.max_prewarm as usize {
            return false;
        }
        let hour = now.hour();
        let active = self.config.active_hours.is_empty()
            || self
                .config
                .active_hours
                .iter()
                .any(|h| u32::from(*h) == hour);
        if !active {
            return false;
        }
        if self.last_hourly_hour == Some(hour) {
            return false;
        }
        self.last_hourly_hour = Some(hour);
        true
    }

    /// Admit a freshly-opened sandbox to the reservoir.
    ///
    /// Returns `true` if admitted, `false` if the reservoir is at
    /// cap. The caller MUST close any sandbox a `false` rejects —
    /// otherwise it leaks E2B-minutes until the out-of-band reaper
    /// catches it.
    pub fn try_admit(&mut self, entry: PrewarmedSandbox) -> bool {
        if self.reservoir.len() >= self.config.max_prewarm as usize {
            return false;
        }
        tracing::info!(
            kind = "runtime.sandbox_prewarmed",
            template = %entry.template,
            sandbox_id = %entry.sandbox.id,
            occupancy = self.reservoir.len() + 1,
            "pre-warmed sandbox admitted to reservoir"
        );
        self.reservoir.push_back(entry);
        true
    }

    /// FIFO take of the oldest pre-warmed sandbox matching
    /// `template`. Emits a `runtime.sandbox_prewarm_consumed`
    /// audit on hit.
    pub fn try_consume(&mut self, template: &str) -> Option<E2bSandbox> {
        let pos = self
            .reservoir
            .iter()
            .position(|e| e.template == template)?;
        let entry = self.reservoir.remove(pos)?;
        tracing::info!(
            kind = "runtime.sandbox_prewarm_consumed",
            template = %entry.template,
            sandbox_id = %entry.sandbox.id,
            occupancy = self.reservoir.len(),
            "pre-warmed sandbox handed to dispatch"
        );
        Some(entry.sandbox)
    }

    /// Remove every reservoir entry idle past `idle_ttl_seconds`.
    /// Returns the drained entries so the caller can close their
    /// underlying sandboxes (best-effort — see
    /// `crate::pool::SandboxPool::reap_idle` for the same
    /// pattern). Sibling brief `mission-cu-sandbox-pool-reaper`
    /// handles the out-of-band E2B-side sweep.
    pub fn drain_idle(&mut self, now: Instant) -> Vec<PrewarmedSandbox> {
        let ttl = Duration::from_secs(self.config.idle_ttl_seconds);
        let mut drained = Vec::new();
        let mut keep: VecDeque<PrewarmedSandbox> =
            VecDeque::with_capacity(self.reservoir.len());
        for entry in self.reservoir.drain(..) {
            if now.duration_since(entry.prewarmed_at) > ttl {
                drained.push(entry);
            } else {
                keep.push_back(entry);
            }
        }
        self.reservoir = keep;
        drained
    }
}

#[cfg(test)]
#[path = "sandbox_prewarm_tests.rs"]
mod tests;
