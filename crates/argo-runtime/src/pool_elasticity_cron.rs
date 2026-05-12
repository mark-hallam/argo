//! Cron-driven tick wrapper for the elasticity recommender. The
//! supervisor surface lands in a follow-up brief
//! (`mission-cron-tick-maintenance-jobs`); this module ships the
//! tick function the supervisor will call, plus the system-job
//! descriptor that registers it once cron-tick is in.
//!
//! Default cadence is 60s — short enough that cooldowns dominate
//! response time, long enough that the recommender cost stays
//! noise.
use crate::pool_elasticity::{
    apply_recommendation, recommend, record_tick, ElasticityConfig, PoolController,
    PoolMetricsSnapshot, ScaleRecommendation,
};
use argo_types::ArgoResult;
use chrono::{DateTime, Utc};

/// System-job name registered with cron-tick once
/// `mission-cron-tick-maintenance-jobs` lands.
pub const SYSTEM_JOB_NAME: &str = "pool_elasticity_tick";

/// Default cadence — 60s, as specified by the mission brief.
pub const DEFAULT_INTERVAL_SECS: u64 = 60;

/// One cron tick: recommend → apply (if `AutoScale`) → record state
/// for metrics / status. Returns the recommendation so the caller
/// can plumb it into an audit row.
///
/// # Errors
/// Propagates from [`apply_recommendation`].
pub async fn tick(
    controller: &dyn PoolController,
    snapshot: &PoolMetricsSnapshot,
    cfg: &ElasticityConfig,
    last_scale_at: Option<DateTime<Utc>>,
) -> ArgoResult<ScaleRecommendation> {
    let rec = recommend(snapshot, cfg, last_scale_at);
    apply_recommendation(controller, &rec, cfg).await?;
    record_tick(snapshot, &rec, cfg.mode);
    Ok(rec)
}

/// "When is the next tick due?" helper used by the cron registrar
/// to schedule the next firing relative to `now`.
#[must_use]
pub fn next_tick_at(now: DateTime<Utc>, cfg: &ElasticityConfig) -> DateTime<Utc> {
    now + chrono::Duration::from_std(cfg.scale_check_interval).unwrap_or(chrono::Duration::seconds(
        DEFAULT_INTERVAL_SECS as i64,
    ))
}
