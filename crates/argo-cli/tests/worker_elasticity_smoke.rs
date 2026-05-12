//! End-to-end smoke for the worker-pool elasticity recommender
//! (`mission-worker-pool-elasticity`).
//!
//! Validates the `RecommendOnly` mode flow:
//! - `[worker_pool.elasticity]` parses cleanly from TOML.
//! - The cron `tick` produces a `ScaleUp` recommendation for an
//!   over-target snapshot.
//! - In `RecommendOnly` mode, the pool controller is NOT mutated
//!   (no spawn / no drain) — only the recommendation is recorded.
//! - The recorded state surfaces through `latest_tick` +
//!   `scale_up_total` / `scale_down_total` counters.
//! - The recommendation's serde shape matches the documented
//!   contract so downstream audit + replay consumers can decode it.
use argo_config::ArgoConfig;
use argo_runtime::{
    apply_recommendation, latest_tick, pool_elasticity_cron, scale_down_total, scale_up_total,
    ElasticityConfig, ElasticityMode, PoolController, PoolMetricsSnapshot, ScaleRecommendation,
};
use argo_types::ArgoResult;
use async_trait::async_trait;
use chrono::Utc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

#[derive(Default)]
struct CountingController {
    spawns: AtomicUsize,
    drains: AtomicUsize,
}

#[async_trait]
impl PoolController for CountingController {
    async fn spawn_worker(&self) -> ArgoResult<()> {
        self.spawns.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
    async fn drain_worker(&self) -> ArgoResult<()> {
        self.drains.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

const TOML: &str = r#"
[worker_pool.elasticity]
mode = "RecommendOnly"
min_workers = 1
max_workers = 6
target_queue_depth_per_worker = 4
scale_check_interval_secs = 30
scale_up_cooldown_secs = 120
scale_down_cooldown_secs = 240
"#;

#[test]
fn parses_worker_pool_elasticity_section() {
    let cfg: ArgoConfig = toml::from_str(TOML).expect("parse toml");
    let e = &cfg.worker_pool.elasticity;
    assert_eq!(e.mode, "RecommendOnly");
    assert_eq!(e.min_workers, 1);
    assert_eq!(e.max_workers, 6);
    assert_eq!(e.target_queue_depth_per_worker, 4);
    assert_eq!(e.scale_check_interval_secs, 30);
    assert_eq!(e.scale_up_cooldown_secs, 120);
    assert_eq!(e.scale_down_cooldown_secs, 240);
    assert_eq!(e.mode_index(), 1, "RecommendOnly → 1");
}

#[test]
fn defaults_drop_in_when_section_absent() {
    let cfg: ArgoConfig = toml::from_str("[core]\nprofile = \"x\"\n").expect("parse");
    let e = &cfg.worker_pool.elasticity;
    assert_eq!(e.min_workers, 1);
    assert_eq!(e.max_workers, 8);
    assert_eq!(e.target_queue_depth_per_worker, 3);
}

#[tokio::test]
async fn recommend_only_tick_records_state_without_mutating_pool() {
    argo_runtime::pool_elasticity::reset_metrics_for_tests();
    let baseline_up = scale_up_total();
    let baseline_down = scale_down_total();
    let cfg = ElasticityConfig {
        min_workers: 1,
        max_workers: 6,
        target_queue_depth_per_worker: 4,
        scale_check_interval: Duration::from_secs(30),
        scale_up_cooldown: Duration::from_secs(120),
        scale_down_cooldown: Duration::from_secs(240),
        mode: ElasticityMode::RecommendOnly,
    };
    let snap = PoolMetricsSnapshot {
        ts: Utc::now(),
        active_workers: 2,
        queue_depth: 20,
        per_worker_utilization: vec![0.95, 0.95],
    };
    let controller = CountingController::default();
    let rec = pool_elasticity_cron::tick(&controller, &snap, &cfg, None)
        .await
        .expect("tick succeeds");
    let ScaleRecommendation::ScaleUp { target, .. } = &rec else {
        panic!("expected ScaleUp recommendation, got {rec:?}");
    };
    assert_eq!(*target, 3, "scale up by exactly one");
    assert_eq!(controller.spawns.load(Ordering::SeqCst), 0, "RecommendOnly is no-op");
    assert_eq!(controller.drains.load(Ordering::SeqCst), 0);
    let latest = latest_tick().expect("tick recorded");
    assert_eq!(latest.snapshot.queue_depth, 20);
    assert_eq!(latest.mode, ElasticityMode::RecommendOnly);
    assert!(matches!(latest.recommendation, ScaleRecommendation::ScaleUp { .. }));
    assert_eq!(
        scale_up_total(),
        baseline_up + 1,
        "scale-up counter incremented"
    );
    assert_eq!(scale_down_total(), baseline_down, "no scale-down");
}

#[tokio::test]
async fn autoscale_mode_invokes_controller() {
    argo_runtime::pool_elasticity::reset_metrics_for_tests();
    let cfg = ElasticityConfig {
        mode: ElasticityMode::AutoScale,
        ..ElasticityConfig::default()
    };
    let snap = PoolMetricsSnapshot {
        ts: Utc::now(),
        active_workers: 2,
        queue_depth: 20,
        per_worker_utilization: vec![0.99, 0.99],
    };
    let controller = CountingController::default();
    apply_recommendation(
        &controller,
        &ScaleRecommendation::ScaleUp {
            target: 3,
            reason: "smoke".into(),
        },
        &cfg,
    )
    .await
    .expect("apply ok");
    assert_eq!(
        controller.spawns.load(Ordering::SeqCst),
        1,
        "AutoScale spawns"
    );
    // Cron tick path also fires the spawn.
    pool_elasticity_cron::tick(&controller, &snap, &cfg, None)
        .await
        .expect("tick");
    assert_eq!(controller.spawns.load(Ordering::SeqCst), 2);
}

#[test]
fn recommendation_serde_shape_is_stable() {
    let rec = ScaleRecommendation::ScaleUp {
        target: 5,
        reason: "queue depth pressure".into(),
    };
    let s = serde_json::to_string(&rec).unwrap();
    assert!(s.contains("ScaleUp"));
    assert!(s.contains("\"target\":5"));
    assert!(s.contains("\"reason\""));
    let parsed: ScaleRecommendation = serde_json::from_str(&s).unwrap();
    assert_eq!(parsed, rec);
}
