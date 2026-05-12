//! Unit tests for `pool_elasticity` — the 7 cases enumerated in
//! `mission-worker-pool-elasticity`. Synthetic snapshots only; no
//! I/O, no async runtime.
use super::{
    apply_recommendation, recommend, ElasticityConfig, ElasticityMode, PoolController,
    PoolMetricsSnapshot, ScaleRecommendation,
};
use argo_types::{ArgoError, ArgoResult};
use async_trait::async_trait;
use chrono::Utc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

fn cfg() -> ElasticityConfig {
    ElasticityConfig {
        min_workers: 1,
        max_workers: 4,
        target_queue_depth_per_worker: 3,
        scale_check_interval: Duration::from_secs(60),
        scale_up_cooldown: Duration::from_secs(300),
        scale_down_cooldown: Duration::from_secs(600),
        mode: ElasticityMode::RecommendOnly,
    }
}

fn snap(active: u32, depth: u32, util: &[f32]) -> PoolMetricsSnapshot {
    PoolMetricsSnapshot {
        ts: Utc::now(),
        active_workers: active,
        queue_depth: depth,
        per_worker_utilization: util.to_vec(),
    }
}

#[test]
fn recommend_hold_at_target_depth() {
    // 2 workers × target 3 = capacity 6; queue at 6 is inside the
    // hold band (below 1.5× = 9, above 0.3× = 1.8). Util mid.
    let r = recommend(&snap(2, 6, &[0.5, 0.5]), &cfg(), None);
    assert_eq!(r, ScaleRecommendation::Hold);
}

#[test]
fn recommend_scale_up_on_150pct_over_target() {
    // capacity 6; 10 > 9 = 1.5× capacity → ScaleUp.
    let r = recommend(&snap(2, 10, &[0.9, 0.9]), &cfg(), None);
    let ScaleRecommendation::ScaleUp { target, reason } = r else {
        panic!("expected ScaleUp, got {r:?}");
    };
    assert_eq!(target, 3, "scale up by exactly one");
    assert!(reason.contains("1.5×"), "rationale must cite threshold: {reason}");
}

#[test]
fn recommend_scale_down_on_30pct_under_and_low_util() {
    // 3 workers × target 3 = capacity 9; queue 2 < 0.3× capacity
    // (2.7), avg util 0.10 < 0.25 → ScaleDown.
    let r = recommend(&snap(3, 2, &[0.1, 0.1, 0.1]), &cfg(), None);
    let ScaleRecommendation::ScaleDown { target, reason } = r else {
        panic!("expected ScaleDown, got {r:?}");
    };
    assert_eq!(target, 2, "scale down by exactly one");
    assert!(reason.contains("0.3×"), "rationale must cite threshold: {reason}");
}

#[test]
fn scale_up_blocked_during_cooldown() {
    let s = snap(2, 10, &[0.9, 0.9]);
    // Last scale was 30s ago; cooldown is 300s — block.
    let last = s.ts - chrono::Duration::seconds(30);
    let r = recommend(&s, &cfg(), Some(last));
    assert_eq!(r, ScaleRecommendation::Hold);
}

#[test]
fn scale_down_blocked_when_already_at_min() {
    // 1 worker == min_workers; even with low util + low queue,
    // recommender holds.
    let r = recommend(&snap(1, 0, &[0.05]), &cfg(), None);
    assert_eq!(r, ScaleRecommendation::Hold);
}

#[test]
fn scale_up_blocked_when_already_at_max() {
    let mut c = cfg();
    c.max_workers = 2;
    let r = recommend(&snap(2, 20, &[0.99, 0.99]), &c, None);
    assert_eq!(r, ScaleRecommendation::Hold);
}

#[test]
fn recommendation_shape_matches_spec() {
    // The three variants serialise with the documented field shape;
    // downstream audit / replay tooling depends on this.
    let up = ScaleRecommendation::ScaleUp {
        target: 3,
        reason: "queue > 1.5× target".into(),
    };
    let down = ScaleRecommendation::ScaleDown {
        target: 1,
        reason: "queue < 0.3× target".into(),
    };
    assert_eq!(up.direction(), "up");
    assert_eq!(down.direction(), "down");
    assert_eq!(ScaleRecommendation::Hold.direction(), "hold");
    assert_eq!(up.target_or(99), 3);
    assert_eq!(ScaleRecommendation::Hold.target_or(7), 7);
    let json = serde_json::to_string(&up).expect("serialise ScaleUp");
    assert!(json.contains("ScaleUp"));
    assert!(json.contains("\"target\":3"));
    assert!(json.contains("\"reason\""));
}

// ────────────── PoolController + apply_recommendation tests ─────────

#[derive(Default)]
struct RecordingController {
    spawned: AtomicUsize,
    drained: AtomicUsize,
    fail_spawn: bool,
}

#[async_trait]
impl PoolController for RecordingController {
    async fn spawn_worker(&self) -> ArgoResult<()> {
        if self.fail_spawn {
            return Err(ArgoError::Provider("spawn failed".into()));
        }
        self.spawned.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
    async fn drain_worker(&self) -> ArgoResult<()> {
        self.drained.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

#[tokio::test]
async fn apply_is_noop_outside_autoscale() {
    let ctrl = RecordingController::default();
    let c = cfg(); // RecommendOnly
    let rec = ScaleRecommendation::ScaleUp {
        target: 3,
        reason: "test".into(),
    };
    apply_recommendation(&ctrl, &rec, &c).await.expect("ok");
    assert_eq!(ctrl.spawned.load(Ordering::SeqCst), 0);
    assert_eq!(ctrl.drained.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn apply_spawns_on_autoscale_up() {
    let ctrl = RecordingController::default();
    let mut c = cfg();
    c.mode = ElasticityMode::AutoScale;
    let rec = ScaleRecommendation::ScaleUp {
        target: 3,
        reason: "test".into(),
    };
    apply_recommendation(&ctrl, &rec, &c).await.expect("ok");
    assert_eq!(ctrl.spawned.load(Ordering::SeqCst), 1);
    assert_eq!(ctrl.drained.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn apply_drains_on_autoscale_down() {
    let ctrl = RecordingController::default();
    let mut c = cfg();
    c.mode = ElasticityMode::AutoScale;
    let rec = ScaleRecommendation::ScaleDown {
        target: 1,
        reason: "test".into(),
    };
    apply_recommendation(&ctrl, &rec, &c).await.expect("ok");
    assert_eq!(ctrl.spawned.load(Ordering::SeqCst), 0);
    assert_eq!(ctrl.drained.load(Ordering::SeqCst), 1);
}

#[test]
fn observe_mode_always_holds() {
    let mut c = cfg();
    c.mode = ElasticityMode::Observe;
    // Queue 10× over target with high util — would normally ScaleUp.
    let r = recommend(&snap(2, 50, &[0.99, 0.99]), &c, None);
    assert_eq!(r, ScaleRecommendation::Hold);
}
