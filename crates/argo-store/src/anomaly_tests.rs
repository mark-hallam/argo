//! Tests for `anomaly` — sibling file to keep `anomaly.rs` under
//! the LOC budget.
use super::*;
use chrono::Utc;

fn s(latency_ms: i64, retry: u32) -> BehaviorSample {
    BehaviorSample {
        latency_ms,
        retry_count: retry,
    }
}

#[test]
fn welford_fold_matches_naive_mean_and_variance() {
    let mut b = BehaviorBaseline::empty("w-1", "k");
    let now = Utc::now();
    let samples = [10_i64, 20, 30, 40, 50];
    for v in samples {
        b.fold(&s(v, 0), now);
    }
    assert_eq!(b.sample_count, 5);
    let mean = samples.iter().sum::<i64>() as f64 / 5.0;
    assert!((b.latency_mean - mean).abs() < 1e-9);
    let variance: f64 = samples.iter().map(|v| (*v as f64 - mean).powi(2)).sum::<f64>() / 5.0;
    let got_var = b.latency_variance().unwrap();
    assert!((got_var - variance).abs() < 1e-9);
    assert!((b.latency_stddev().unwrap() - variance.sqrt()).abs() < 1e-9);
}

#[test]
fn z_score_signed_after_two_samples() {
    let mut b = BehaviorBaseline::empty("w-1", "k");
    let now = Utc::now();
    b.fold(&s(10, 0), now);
    b.fold(&s(20, 0), now);
    let (l_z, _) = b.z_scores(&s(25, 0));
    assert!((l_z.unwrap() - 2.0).abs() < 1e-9);
    let (l_z, _) = b.z_scores(&s(5, 0));
    assert!(l_z.unwrap() < 0.0);
}

#[test]
fn z_score_none_for_small_baseline() {
    let mut b = BehaviorBaseline::empty("w-1", "k");
    b.fold(&s(10, 0), Utc::now());
    assert!(b.latency_variance().is_none());
    assert!(b.z_scores(&s(100, 0)).0.is_none());
}

#[test]
fn z_score_none_when_stddev_zero() {
    let mut b = BehaviorBaseline::empty("w-1", "k");
    let now = Utc::now();
    for _ in 0..5 {
        b.fold(&s(42, 0), now);
    }
    assert_eq!(b.latency_stddev().unwrap(), 0.0);
    assert!(b.z_scores(&s(99, 0)).0.is_none());
}

#[test]
fn threshold_paths_serial() {
    // Combined to avoid env-var cross-test races; configured_threshold
    // reads a process-global var so both paths must run in sequence.
    std::env::remove_var("ARGO_ANOMALY_Z_THRESHOLD");
    assert_eq!(configured_threshold(), DEFAULT_Z_THRESHOLD);
    std::env::set_var("ARGO_ANOMALY_Z_THRESHOLD", "2.5");
    assert!((configured_threshold() - 2.5).abs() < 1e-9);
    std::env::set_var("ARGO_ANOMALY_Z_THRESHOLD", "garbage");
    assert_eq!(configured_threshold(), DEFAULT_Z_THRESHOLD);
    std::env::set_var("ARGO_ANOMALY_Z_THRESHOLD", "-1.0");
    assert_eq!(configured_threshold(), DEFAULT_Z_THRESHOLD);
    std::env::remove_var("ARGO_ANOMALY_Z_THRESHOLD");
}

async fn pool() -> sqlx::SqlitePool {
    // The anomaly schema lives in `migrations/v19.rs`; opening a
    // `SessionStore` and migrating gives us the worker_anomaly_event
    // and worker_behavior_baseline tables this module reads/writes.
    let store = crate::SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    store.pool().clone()
}

#[tokio::test]
async fn save_and_load_round_trips_baseline() {
    let p = pool().await;
    let mut b = BehaviorBaseline::empty("w-1", "post_listing");
    b.fold(&s(100, 1), Utc::now());
    b.fold(&s(120, 2), Utc::now());
    save_baseline(&p, &b).await.unwrap();
    let loaded = load_baseline(&p, "w-1", "post_listing").await.unwrap();
    assert_eq!(loaded.sample_count, 2);
    assert!((loaded.latency_mean - 110.0).abs() < 1e-9);
    assert!((loaded.retry_mean - 1.5).abs() < 1e-9);
}

#[tokio::test]
async fn record_no_events_below_min_samples() {
    let p = pool().await;
    for _ in 0..4 {
        let evts = record_sample(
            &p,
            "w-1",
            "x",
            "post_listing",
            s(100, 0),
            3.0,
            Utc::now(),
        )
        .await
        .unwrap();
        assert!(evts.is_empty());
    }
}

#[tokio::test]
async fn record_emits_event_on_extreme_z() {
    let p = pool().await;
    for v in [100, 102, 98, 101, 99, 100] {
        record_sample(
            &p,
            "w-2",
            "seed",
            "post_listing",
            s(v, 0),
            3.0,
            Utc::now(),
        )
        .await
        .unwrap();
    }
    let evts = record_sample(
        &p,
        "w-2",
        "spike",
        "post_listing",
        s(1_000, 0),
        3.0,
        Utc::now(),
    )
    .await
    .unwrap();
    let lat = evts.iter().find(|e| e.dimension == "latency_ms").unwrap();
    assert!(lat.z_score.abs() >= 3.0);
    assert_eq!(lat.action_id, "spike");
}
