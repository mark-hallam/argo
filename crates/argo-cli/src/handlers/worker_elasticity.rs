//! `argo worker elasticity status|config` — read-only surface for the
//! worker-pool elasticity recommender (`mission-worker-pool-elasticity`).
//!
//! `status` reads the in-process latest tick + scale-event counters
//! recorded by `argo_runtime::pool_elasticity_cron::tick` and emits
//! JSON / text. `config` resolves the active `[worker_pool.elasticity]`
//! section from the loaded `argo.toml` (falling back to shipped
//! defaults) and prints the bounds, cooldowns, and operating mode.
//!
//! v1 is read-only — the AutoScale mode flip lives in `argo.toml`
//! and is reloaded on daemon restart, not via CLI mutation.

use argo_config::{load, WorkerPoolElasticityConfig};
use argo_runtime::{
    latest_tick, scale_down_total, scale_up_total, ElasticityConfig, ElasticityMode,
    PoolMetricsSnapshot, ScaleRecommendation, TickState,
};
use argo_types::ArgoResult;
use serde::Serialize;
use std::time::Duration;

#[derive(Debug, Serialize)]
struct StatusJson {
    snapshot: Option<PoolMetricsSnapshot>,
    recommendation: Option<ScaleRecommendation>,
    mode: Option<ElasticityMode>,
    scale_up_total: u64,
    scale_down_total: u64,
}

#[derive(Debug, Serialize)]
struct ConfigJson {
    mode: ElasticityMode,
    min_workers: u32,
    max_workers: u32,
    target_queue_depth_per_worker: u32,
    scale_check_interval_secs: u64,
    scale_up_cooldown_secs: u64,
    scale_down_cooldown_secs: u64,
}

/// `argo worker elasticity status`.
///
/// # Errors
/// Currently infallible; the signature matches the dispatcher's
/// expectation so a future store-backed snapshot doesn't break callers.
pub fn status(json: bool) -> ArgoResult<()> {
    let latest = latest_tick();
    let payload = StatusJson {
        snapshot: latest.as_ref().map(|t| t.snapshot.clone()),
        recommendation: latest.as_ref().map(|t| t.recommendation.clone()),
        mode: latest.as_ref().map(|t| t.mode),
        scale_up_total: scale_up_total(),
        scale_down_total: scale_down_total(),
    };
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".to_string())
        );
    } else {
        render_status_text(&payload, latest.as_ref());
    }
    Ok(())
}

/// `argo worker elasticity config`.
///
/// # Errors
/// Propagates from [`argo_config::load`] on parse failure.
pub fn config(json: bool) -> ArgoResult<()> {
    let cfg = load()?;
    let elastic = to_elasticity_config(&cfg.worker_pool.elasticity);
    let payload = ConfigJson {
        mode: elastic.mode,
        min_workers: elastic.min_workers,
        max_workers: elastic.max_workers,
        target_queue_depth_per_worker: elastic.target_queue_depth_per_worker,
        scale_check_interval_secs: elastic.scale_check_interval.as_secs(),
        scale_up_cooldown_secs: elastic.scale_up_cooldown.as_secs(),
        scale_down_cooldown_secs: elastic.scale_down_cooldown.as_secs(),
    };
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".to_string())
        );
    } else {
        render_config_text(&payload);
    }
    Ok(())
}

fn render_status_text(payload: &StatusJson, latest: Option<&TickState>) {
    println!("# argo worker elasticity status");
    println!("scale_up_total   = {}", payload.scale_up_total);
    println!("scale_down_total = {}", payload.scale_down_total);
    match latest {
        Some(tick) => {
            println!("mode             = {:?}", tick.mode);
            println!("active_workers   = {}", tick.snapshot.active_workers);
            println!("queue_depth      = {}", tick.snapshot.queue_depth);
            let avg = avg_util(&tick.snapshot.per_worker_utilization);
            println!("avg_utilization  = {avg:.2}");
            println!("recommendation   = {:?}", tick.recommendation);
        }
        None => {
            println!("(no tick recorded since process start)");
        }
    }
}

fn render_config_text(payload: &ConfigJson) {
    println!("# [worker_pool.elasticity]");
    println!("mode                          = {:?}", payload.mode);
    println!("min_workers                   = {}", payload.min_workers);
    println!("max_workers                   = {}", payload.max_workers);
    println!(
        "target_queue_depth_per_worker = {}",
        payload.target_queue_depth_per_worker
    );
    println!(
        "scale_check_interval_secs     = {}",
        payload.scale_check_interval_secs
    );
    println!(
        "scale_up_cooldown_secs        = {}",
        payload.scale_up_cooldown_secs
    );
    println!(
        "scale_down_cooldown_secs      = {}",
        payload.scale_down_cooldown_secs
    );
}

fn avg_util(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    samples.iter().sum::<f32>() / samples.len() as f32
}

/// Convert the TOML-shaped argo-config struct into the argo-runtime
/// recommender config. Conversion lives here (not in argo-config) so
/// argo-config stays free of argo-runtime as a dependency.
pub fn to_elasticity_config(c: &WorkerPoolElasticityConfig) -> ElasticityConfig {
    ElasticityConfig {
        min_workers: c.min_workers,
        max_workers: c.max_workers,
        target_queue_depth_per_worker: c.target_queue_depth_per_worker,
        scale_check_interval: Duration::from_secs(c.scale_check_interval_secs),
        scale_up_cooldown: Duration::from_secs(c.scale_up_cooldown_secs),
        scale_down_cooldown: Duration::from_secs(c.scale_down_cooldown_secs),
        mode: match c.mode_index() {
            0 => ElasticityMode::Observe,
            2 => ElasticityMode::AutoScale,
            _ => ElasticityMode::RecommendOnly,
        },
    }
}

