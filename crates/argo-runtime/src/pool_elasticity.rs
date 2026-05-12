//! Worker-pool elasticity recommender (`mission-worker-pool-elasticity`).
//!
//! The recommender is a **pure function** over a [`PoolMetricsSnapshot`] +
//! an [`ElasticityConfig`] + the last scale-event timestamp; it returns a
//! [`ScaleRecommendation`] of `Hold` / `ScaleUp` / `ScaleDown`. Keeping
//! the decision pure makes it trivial to unit-test against synthetic
//! snapshots and to replay against historical audit data.
//!
//! Applying a recommendation is the impure half — it spawns or drains a
//! worker through the [`PoolController`] trait. Only `AutoScale` mode
//! actually mutates the pool; `Observe` and `RecommendOnly` are no-ops at
//! the apply step (the recommendation still surfaces in metrics + audit so
//! the operator can validate the recommender before flipping to AutoScale).
//!
//! Default mode is [`ElasticityMode::RecommendOnly`]. See the operator
//! runbook for the Observe → RecommendOnly → AutoScale promotion path.
use argo_types::ArgoResult;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Duration;

/// Operating mode for the elasticity recommender.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ElasticityMode {
    /// Snapshots captured + audited; no recommendation produced.
    Observe,
    /// Recommendation produced + surfaced; pool is never mutated.
    #[default]
    RecommendOnly,
    /// Recommendation produced AND applied via [`PoolController`].
    AutoScale,
}

/// Operator-tunable bounds for the recommender.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElasticityConfig {
    /// Hard floor; `1` is the safe default — a pool of 0 workers
    /// cannot accept inbound briefs.
    pub min_workers: u32,
    /// Hard ceiling tuned against host CPU / memory / per-worker budget.
    pub max_workers: u32,
    /// Steady-state queue depth per worker the recommender drives the
    /// pool toward. Scale-up at 1.5×, scale-down at 0.3× of this target.
    pub target_queue_depth_per_worker: u32,
    /// How often the cron-driven tick observes pool state.
    pub scale_check_interval: Duration,
    /// Minimum time between successive scale-ups (5 min default).
    pub scale_up_cooldown: Duration,
    /// Minimum time between successive scale-downs (10 min default,
    /// more conservative than up to avoid flapping).
    pub scale_down_cooldown: Duration,
    /// Operating mode (see [`ElasticityMode`]).
    pub mode: ElasticityMode,
}

impl Default for ElasticityConfig {
    fn default() -> Self {
        Self {
            min_workers: 1,
            max_workers: 8,
            target_queue_depth_per_worker: 3,
            scale_check_interval: Duration::from_secs(60),
            scale_up_cooldown: Duration::from_secs(300),
            scale_down_cooldown: Duration::from_secs(600),
            mode: ElasticityMode::default(),
        }
    }
}

/// Captured pool state at one tick.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolMetricsSnapshot {
    /// When the snapshot was taken.
    pub ts: DateTime<Utc>,
    /// Workers currently alive (not yet drained).
    pub active_workers: u32,
    /// Combined TODO + CLAIMED queue depth at `ts`.
    pub queue_depth: u32,
    /// Rolling utilisation per worker in `[0, 1]` over the last
    /// `scale_check_interval`. Empty when telemetry is missing — in
    /// which case the recommender treats utilisation as `1.0` and
    /// never scales down.
    pub per_worker_utilization: Vec<f32>,
}

/// Output of one recommender call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScaleRecommendation {
    /// No action; pool is sized correctly OR a cooldown is active.
    Hold,
    /// Spawn one worker, taking active count to `target`.
    ScaleUp {
        /// Worker count after the spawn (clamped to `max_workers`).
        target: u32,
        /// Human-readable rationale.
        reason: String,
    },
    /// Drain one worker, taking active count to `target`.
    ScaleDown {
        /// Worker count after the drain (clamped to `min_workers`).
        target: u32,
        /// Human-readable rationale.
        reason: String,
    },
}

impl ScaleRecommendation {
    /// Direction tag used by metrics / audit emitters.
    #[must_use]
    pub fn direction(&self) -> &'static str {
        match self {
            Self::Hold => "hold",
            Self::ScaleUp { .. } => "up",
            Self::ScaleDown { .. } => "down",
        }
    }

    /// Target worker count, with `Hold` falling back to `current`.
    #[must_use]
    pub fn target_or(&self, current: u32) -> u32 {
        match self {
            Self::Hold => current,
            Self::ScaleUp { target, .. } | Self::ScaleDown { target, .. } => *target,
        }
    }
}

/// Pure recommender.
///
/// - Scale up when `queue_depth > active * target * 1.5`, not in
///   scale-up cooldown, and below `max_workers`.
/// - Scale down when `queue_depth < active * target * 0.3`, average
///   utilisation `< 0.25`, not in scale-down cooldown, and above
///   `min_workers`.
/// - Otherwise `Hold`. One worker per tick — multi-step scaling is
///   unstable; cooldowns let the system settle.
///
/// ```
/// use argo_runtime::{ElasticityConfig, PoolMetricsSnapshot, ScaleRecommendation, recommend};
/// use chrono::Utc;
/// let cfg = ElasticityConfig::default();
/// let snap = PoolMetricsSnapshot {
///     ts: Utc::now(), active_workers: 2, queue_depth: 20,
///     per_worker_utilization: vec![0.9, 0.9],
/// };
/// assert!(matches!(recommend(&snap, &cfg, None), ScaleRecommendation::ScaleUp { .. }));
/// ```
#[must_use]
pub fn recommend(
    snapshot: &PoolMetricsSnapshot,
    cfg: &ElasticityConfig,
    last_scale_at: Option<DateTime<Utc>>,
) -> ScaleRecommendation {
    if cfg.mode == ElasticityMode::Observe {
        return ScaleRecommendation::Hold;
    }
    let active = snapshot.active_workers.max(1);
    let target = cfg.target_queue_depth_per_worker.max(1);
    let capacity = u64::from(active) * u64::from(target);
    let depth = u64::from(snapshot.queue_depth);
    // Compare without integer truncation:
    //   depth > capacity * 1.5  ⇔  depth × 2 > capacity × 3
    //   depth < capacity * 0.3  ⇔  depth × 10 < capacity × 3
    let scale_up_trips = depth.saturating_mul(2) > capacity.saturating_mul(3);
    let scale_down_trips = depth.saturating_mul(10) < capacity.saturating_mul(3);
    if scale_up_trips && active < cfg.max_workers {
        if in_cooldown(last_scale_at, snapshot.ts, cfg.scale_up_cooldown) {
            return ScaleRecommendation::Hold;
        }
        return ScaleRecommendation::ScaleUp {
            target: (active + 1).min(cfg.max_workers),
            reason: format!(
                "queue {} > 1.5× target ({}× {})",
                snapshot.queue_depth, active, target
            ),
        };
    }
    let avg_util = average_util(&snapshot.per_worker_utilization);
    if scale_down_trips && avg_util < 0.25 && active > cfg.min_workers {
        if in_cooldown(last_scale_at, snapshot.ts, cfg.scale_down_cooldown) {
            return ScaleRecommendation::Hold;
        }
        return ScaleRecommendation::ScaleDown {
            target: active.saturating_sub(1).max(cfg.min_workers),
            reason: format!(
                "queue {} < 0.3× target ({}× {}); avg utilisation {:.2} < 0.25",
                snapshot.queue_depth, active, target, avg_util
            ),
        };
    }
    ScaleRecommendation::Hold
}

fn in_cooldown(last: Option<DateTime<Utc>>, now: DateTime<Utc>, window: Duration) -> bool {
    let Some(last) = last else {
        return false;
    };
    let Ok(elapsed) = now.signed_duration_since(last).to_std() else {
        return false;
    };
    elapsed < window
}

fn average_util(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 1.0;
    }
    samples.iter().sum::<f32>() / samples.len() as f32
}

/// Pool-side actions the elasticity applier can request. Wired by the
/// worker-supervisor surface (out of scope for this brief).
#[async_trait]
pub trait PoolController: Send + Sync {
    /// Spawn one worker. Called on `ScaleUp` in `AutoScale` mode.
    async fn spawn_worker(&self) -> ArgoResult<()>;
    /// Mark one worker `DrainRequested`. The worker MUST complete any
    /// in-flight brief (or release the claim cleanly) before
    /// terminating; never SIGKILL.
    async fn drain_worker(&self) -> ArgoResult<()>;
}

/// Apply a recommendation. Returns `Ok(())` without touching the pool
/// when `mode != AutoScale` or `rec == Hold`.
///
/// # Errors
/// Propagates from [`PoolController::spawn_worker`] / [`PoolController::drain_worker`].
pub async fn apply_recommendation(
    controller: &dyn PoolController,
    rec: &ScaleRecommendation,
    cfg: &ElasticityConfig,
) -> ArgoResult<()> {
    if cfg.mode != ElasticityMode::AutoScale {
        return Ok(());
    }
    match rec {
        ScaleRecommendation::Hold => Ok(()),
        ScaleRecommendation::ScaleUp { .. } => controller.spawn_worker().await,
        ScaleRecommendation::ScaleDown { .. } => controller.drain_worker().await,
    }
}

/// Audit payload emitted per elasticity tick. Stable serde shape so
/// downstream sinks (audit chain, log shippers, replay) can decode it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElasticityAuditRow {
    /// Always `"pool_elasticity_tick"`.
    pub kind: &'static str,
    /// Captured snapshot.
    pub snapshot: PoolMetricsSnapshot,
    /// Recommender output.
    pub recommendation: ScaleRecommendation,
    /// Whether [`apply_recommendation`] was invoked. False for
    /// `Observe` / `RecommendOnly`; true for `AutoScale` even when
    /// the recommendation was `Hold`.
    pub applied: bool,
    /// Mode at tick time (so replays reproduce the decision without
    /// separately tracking config evolution).
    pub mode: ElasticityMode,
}

/// Emit an audit row for this tick. v1 ships a structured tracing
/// event — the canonical sink for daemon stdout / journalctl. A
/// follow-up brief wires this into `argo-audit::AuditSink` once the
/// supervisor surface lands; the [`ElasticityAuditRow`] serde shape is
/// the wire contract callers can depend on.
///
/// # Errors
/// Currently infallible. Signature keeps `ArgoResult` so a future
/// `AuditSink::append` swap-in does not break callers.
pub fn emit_audit_row(
    snapshot: &PoolMetricsSnapshot,
    rec: &ScaleRecommendation,
    cfg: &ElasticityConfig,
) -> ArgoResult<ElasticityAuditRow> {
    let applied = cfg.mode == ElasticityMode::AutoScale;
    let row = ElasticityAuditRow {
        kind: "pool_elasticity_tick",
        snapshot: snapshot.clone(),
        recommendation: rec.clone(),
        applied,
        mode: cfg.mode,
    };
    tracing::info!(
        target: "argo_runtime::pool_elasticity",
        kind = row.kind,
        active = snapshot.active_workers,
        queue_depth = snapshot.queue_depth,
        direction = rec.direction(),
        target_workers = rec.target_or(snapshot.active_workers),
        applied,
        mode = ?cfg.mode,
        "pool elasticity tick"
    );
    Ok(row)
}

/// Latest tick observed via [`record_tick`]. Read by metrics + CLI
/// status. `None` until the first tick fires.
static LATEST_TICK: Mutex<Option<TickState>> = Mutex::new(None);
static SCALE_UP_TOTAL: AtomicU64 = AtomicU64::new(0);
static SCALE_DOWN_TOTAL: AtomicU64 = AtomicU64::new(0);

/// Snapshot of the last elasticity tick — exposed to metrics + CLI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TickState {
    /// Captured pool snapshot.
    pub snapshot: PoolMetricsSnapshot,
    /// Recommender output for this tick.
    pub recommendation: ScaleRecommendation,
    /// Mode at tick time.
    pub mode: ElasticityMode,
}

/// Stamp the latest tick into module-local state so metrics + the
/// `argo worker elasticity-status` CLI can read it.
pub fn record_tick(snapshot: &PoolMetricsSnapshot, rec: &ScaleRecommendation, mode: ElasticityMode) {
    match rec {
        ScaleRecommendation::ScaleUp { .. } => {
            SCALE_UP_TOTAL.fetch_add(1, Ordering::Relaxed);
        }
        ScaleRecommendation::ScaleDown { .. } => {
            SCALE_DOWN_TOTAL.fetch_add(1, Ordering::Relaxed);
        }
        ScaleRecommendation::Hold => {}
    }
    if let Ok(mut guard) = LATEST_TICK.lock() {
        *guard = Some(TickState {
            snapshot: snapshot.clone(),
            recommendation: rec.clone(),
            mode,
        });
    }
}

/// Most recent tick, if any. Cloned so callers do not hold the lock.
#[must_use]
pub fn latest_tick() -> Option<TickState> {
    LATEST_TICK.lock().ok().and_then(|g| g.clone())
}

/// Counter — scale-up recommendations observed since process boot.
#[must_use]
pub fn scale_up_total() -> u64 {
    SCALE_UP_TOTAL.load(Ordering::Relaxed)
}

/// Counter — scale-down recommendations observed since process boot.
#[must_use]
pub fn scale_down_total() -> u64 {
    SCALE_DOWN_TOTAL.load(Ordering::Relaxed)
}

#[doc(hidden)]
pub fn reset_metrics_for_tests() {
    SCALE_UP_TOTAL.store(0, Ordering::Relaxed);
    SCALE_DOWN_TOTAL.store(0, Ordering::Relaxed);
    if let Ok(mut g) = LATEST_TICK.lock() {
        *g = None;
    }
}

#[cfg(test)]
#[path = "pool_elasticity_tests.rs"]
mod tests;
