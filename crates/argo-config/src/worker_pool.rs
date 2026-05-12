//! `[worker_pool]` TOML — worker-pool elasticity recommender bounds
//! (`mission-worker-pool-elasticity`).
//!
//! Sized in its own module (not `sections.rs`) because that file
//! already brushes the per-file 500-LOC ceiling. The struct stays
//! provider-shape (no behaviour) so `argo-cli` / `argo-runtime` can
//! consume it without forcing argo-config to depend on argo-runtime.
use serde::{Deserialize, Serialize};

/// `[worker_pool]` root.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct WorkerPoolConfig {
    /// `[worker_pool.elasticity]`.
    pub elasticity: WorkerPoolElasticityConfig,
}

/// `[worker_pool.elasticity]` — see
/// `argo_runtime::pool_elasticity::ElasticityConfig`. Stringly-typed
/// `mode` (parsed via [`WorkerPoolElasticityConfig::mode_index`]) so
/// operators can flip via TOML without learning the enum spelling.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WorkerPoolElasticityConfig {
    /// `Observe` | `RecommendOnly` | `AutoScale`. Default
    /// `RecommendOnly` — safe-by-default; AutoScale is opt-in.
    pub mode: String,
    /// Hard floor — `1` so a pool can always accept inbound briefs.
    pub min_workers: u32,
    /// Hard ceiling — tuned by the operator against host CPU /
    /// per-worker concurrency budget.
    pub max_workers: u32,
    /// Steady-state queue depth per worker the recommender drives
    /// the pool toward.
    pub target_queue_depth_per_worker: u32,
    /// Cron tick cadence in seconds.
    pub scale_check_interval_secs: u64,
    /// Minimum interval between successive scale-ups (5 min default).
    pub scale_up_cooldown_secs: u64,
    /// Minimum interval between successive scale-downs (10 min default;
    /// more conservative than up to avoid flapping).
    pub scale_down_cooldown_secs: u64,
}

impl Default for WorkerPoolElasticityConfig {
    fn default() -> Self {
        Self {
            mode: "RecommendOnly".into(),
            min_workers: 1,
            max_workers: 8,
            target_queue_depth_per_worker: 3,
            scale_check_interval_secs: 60,
            scale_up_cooldown_secs: 300,
            scale_down_cooldown_secs: 600,
        }
    }
}

impl WorkerPoolElasticityConfig {
    /// Stable 0 / 1 / 2 index for `Observe` / `RecommendOnly` /
    /// `AutoScale`. Unknown strings collapse to `RecommendOnly`
    /// (safe-by-default). Consumers map this to their own
    /// enum — keeps argo-config provider-shape.
    #[must_use]
    pub fn mode_index(&self) -> u8 {
        match self.mode.to_ascii_lowercase().as_str() {
            "observe" => 0,
            "autoscale" => 2,
            _ => 1,
        }
    }
}
