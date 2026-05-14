//! `[runtime.sandbox_prewarm]` — burst pre-warming for the E2B
//! sandbox pool (`mission-burst-pre-warming-sandbox`,
//! MISSION_BACKLOG §8 #255).
//!
//! Sibling of [`crate::sections_context`]; the parent module re-exports
//! these symbols so external imports stay `argo_config::SandboxPrewarmConfig`.
use serde::{Deserialize, Serialize};

/// Default spawn-rate threshold for the burst pre-warm trigger.
pub const DEFAULT_SANDBOX_PREWARM_SPAWN_RATE_THRESHOLD: u32 = 3;
/// Default reservoir cap.
pub const DEFAULT_SANDBOX_PREWARM_MAX: u32 = 5;
/// Default sliding-window width for spawn-rate detection.
pub const DEFAULT_SANDBOX_PREWARM_WINDOW_SECS: u64 = 300;
/// Default extra sandboxes opened on each burst trigger.
pub const DEFAULT_SANDBOX_PREWARM_BURST_SIZE: u32 = 2;
/// Default idle TTL for pre-warmed sandboxes.
pub const DEFAULT_SANDBOX_PREWARM_IDLE_TTL_SECS: u64 = 600;

/// `[runtime.sandbox_prewarm]` — burst pre-warming for the E2B
/// sandbox pool (`mission-burst-pre-warming-sandbox`,
/// MISSION_BACKLOG §8 #255). Disabled by default so opting in is
/// always a deliberate operator action; pre-warming consumes
/// E2B-minutes even when the reservoir goes unused.
///
/// The in-memory shape lives in
/// `argo_executor_computer_use::sandbox_prewarm::SandboxPrewarmConfig`;
/// this struct mirrors the TOML-side knobs and is mapped at the
/// executor's bootstrap step. The split keeps `argo-config` free
/// of a build-time dependency on the executor crate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SandboxPrewarmConfig {
    /// Master enable. When `false`, the prewarmer is not
    /// constructed and the dispatch path stays zero-overhead.
    pub enabled: bool,
    /// Spawn-events-per-window above which a burst pre-warm
    /// fires. Strict greater-than: with `3`, 3 spawns/window does
    /// NOT trigger; 4 does.
    pub spawn_rate_threshold: u32,
    /// Reservoir cap. Excess pre-warmed sandboxes are closed
    /// rather than admitted.
    pub max_prewarm: u32,
    /// Sliding-window width (seconds) used by the spawn-rate
    /// trigger.
    pub window_seconds: u64,
    /// Extra sandboxes opened on each burst trigger.
    pub burst_size: u32,
    /// Pre-warmed sandbox idle TTL (seconds). Past this the
    /// reservoir reaper drops the entry. Mirrors the in-band pool
    /// idle TTL so the two layers stay aligned.
    pub idle_ttl_seconds: u64,
    /// Operator-active hours (`0..=23`). Empty means "every hour
    /// is active" (no quiet hours configured). Non-empty is the
    /// inverse of the operator's quiet-hours block — the hourly
    /// pre-warm trigger fires only when `now.hour()` matches an
    /// entry.
    pub active_hours: Vec<u8>,
}

impl Default for SandboxPrewarmConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            spawn_rate_threshold: DEFAULT_SANDBOX_PREWARM_SPAWN_RATE_THRESHOLD,
            max_prewarm: DEFAULT_SANDBOX_PREWARM_MAX,
            window_seconds: DEFAULT_SANDBOX_PREWARM_WINDOW_SECS,
            burst_size: DEFAULT_SANDBOX_PREWARM_BURST_SIZE,
            idle_ttl_seconds: DEFAULT_SANDBOX_PREWARM_IDLE_TTL_SECS,
            active_hours: Vec::new(),
        }
    }
}
