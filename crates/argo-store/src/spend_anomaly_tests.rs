//! Unit + integration tests for the spend-anomaly detector
//! (Stream `claude-spend-anomaly-alerts`). Sibling test file so
//! `spend_anomaly.rs` stays focused on the data model + queries.
use super::{
    configured_threshold, ensure_schema, list_active_worker_ids, list_anomalies, load_baseline,
    observe_recent_hour, previous_hour_bounds, record_anomaly, save_baseline, AnomalyFilters,
    SpendBaseline, DEFAULT_SPEND_Z_THRESHOLD, MIN_SAMPLES_FOR_DETECT,
};
use chrono::{TimeZone, Utc};
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::SqlitePool;

async fn fresh_pool() -> SqlitePool {
    let pool = SqlitePoolOptions::new()
        .connect("sqlite::memory:")
        .await
        .expect("in-memory pool");
    sqlx::query(include_str!("../migrations/14__worker_budget.sql"))
        .execute(&pool)
        .await
        .expect("worker_budget schema");
    ensure_schema(&pool).await.expect("spend anomaly schema");
    pool
}

#[test]
fn empty_baseline_has_zero_stddev_and_no_z_score() {
    let b = SpendBaseline::empty("worker-1", 9);
    assert_eq!(b.sample_count, 0);
    assert_eq!(b.stddev(), 0.0);
    assert!(b.z_score_for(100.0).is_none(), "no detect below MIN_SAMPLES");
}

#[test]
fn welford_update_single_sample_sets_mean_and_keeps_zero_variance() {
    let mut b = SpendBaseline::empty("worker-1", 9);
    b.update(2.0, 1_000);
    assert_eq!(b.sample_count, 1);
    assert!((b.mean_usd_per_hour - 2.0).abs() < 1e-9);
    assert_eq!(b.m2_usd_per_hour, 0.0);
    assert_eq!(b.stddev(), 0.0);
    assert_eq!(b.updated_at_micros, 1_000);
}

#[test]
fn welford_update_many_samples_matches_naive_mean_and_stddev() {
    let samples = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0];
    let mut b = SpendBaseline::empty("w", 0);
    for (i, s) in samples.iter().enumerate() {
        b.update(*s, (i + 1) as i64 * 1000);
    }
    let n = samples.len() as f64;
    let naive_mean = samples.iter().sum::<f64>() / n;
    let naive_var =
        samples.iter().map(|x| (x - naive_mean).powi(2)).sum::<f64>() / (n - 1.0);
    let naive_stddev = naive_var.sqrt();
    assert!(
        (b.mean_usd_per_hour - naive_mean).abs() < 1e-9,
        "Welford mean {} ≠ naive {}",
        b.mean_usd_per_hour,
        naive_mean
    );
    assert!(
        (b.stddev() - naive_stddev).abs() < 1e-9,
        "Welford stddev {} ≠ naive {}",
        b.stddev(),
        naive_stddev
    );
}

#[test]
fn z_score_returns_none_below_min_samples() {
    let mut b = SpendBaseline::empty("w", 0);
    for i in 0..(MIN_SAMPLES_FOR_DETECT - 1) {
        b.update((i + 1) as f64, 0);
    }
    assert!(b.z_score_for(100.0).is_none());
}

#[test]
fn z_score_zero_when_observed_equals_mean() {
    let mut b = SpendBaseline::empty("w", 0);
    for v in [10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0] {
        b.update(v, 0);
    }
    // All-equal samples → stddev=0 → z_score returns None (no
    // variance to score against).
    assert!(b.z_score_for(10.0).is_none());

    // Now mix in a different sample so stddev > 0.
    b.update(11.0, 0);
    let z = b.z_score_for(b.mean_usd_per_hour).expect("has stddev now");
    assert!(z.abs() < 1e-9, "z={z}");
}

#[test]
fn z_score_matches_expected_for_normal_baseline() {
    // 10 samples N(mean=5, stddev≈1.581)
    let mut b = SpendBaseline::empty("w", 0);
    for v in [3.0, 4.0, 4.0, 5.0, 5.0, 5.0, 6.0, 6.0, 7.0, 5.0] {
        b.update(v, 0);
    }
    let mean = b.mean_usd_per_hour;
    let stddev = b.stddev();
    let observed = mean + 4.0 * stddev;
    let z = b.z_score_for(observed).expect("detect");
    assert!((z - 4.0).abs() < 1e-9, "expected z=4, got z={z}");
    let observed_low = mean - 4.0 * stddev;
    let z_low = b.z_score_for(observed_low).expect("detect");
    assert!(
        (z_low + 4.0).abs() < 1e-9,
        "spike-/drought- preserved sign: {z_low}"
    );
}

#[test]
fn configured_threshold_falls_back_to_default_when_unset() {
    // SAFETY: tests run sequentially; restore env after.
    let prev = std::env::var("ARGO_SPEND_ANOMALY_Z").ok();
    std::env::remove_var("ARGO_SPEND_ANOMALY_Z");
    assert_eq!(configured_threshold(), DEFAULT_SPEND_Z_THRESHOLD);
    if let Some(v) = prev {
        std::env::set_var("ARGO_SPEND_ANOMALY_Z", v);
    }
}

#[test]
fn configured_threshold_reads_env_when_set_to_positive_float() {
    let prev = std::env::var("ARGO_SPEND_ANOMALY_Z").ok();
    std::env::set_var("ARGO_SPEND_ANOMALY_Z", "5.5");
    assert!((configured_threshold() - 5.5).abs() < 1e-9);
    // Non-finite / non-positive falls back.
    std::env::set_var("ARGO_SPEND_ANOMALY_Z", "-1");
    assert_eq!(configured_threshold(), DEFAULT_SPEND_Z_THRESHOLD);
    std::env::set_var("ARGO_SPEND_ANOMALY_Z", "garbage");
    assert_eq!(configured_threshold(), DEFAULT_SPEND_Z_THRESHOLD);
    if let Some(v) = prev {
        std::env::set_var("ARGO_SPEND_ANOMALY_Z", v);
    } else {
        std::env::remove_var("ARGO_SPEND_ANOMALY_Z");
    }
}

#[test]
fn previous_hour_bounds_returns_correct_window_and_hour_of_day() {
    let now = Utc.with_ymd_and_hms(2026, 5, 7, 14, 25, 0).unwrap();
    let (start, end, hod) = previous_hour_bounds(now);
    assert_eq!(hod, 13);
    let start_dt = Utc.timestamp_opt(start, 0).single().expect("start dt");
    let end_dt = Utc.timestamp_opt(end, 0).single().expect("end dt");
    assert_eq!(start_dt.hour(), 13);
    assert_eq!(end_dt.hour(), 14);
    assert_eq!(end_dt.minute(), 0);
    assert_eq!(end - start, 3600);
}

#[tokio::test]
async fn save_and_load_baseline_round_trip() {
    let pool = fresh_pool().await;
    let mut b = SpendBaseline::empty("w-1", 14);
    for v in [1.0, 2.0, 3.0, 4.0, 5.0] {
        b.update(v, 1_000);
    }
    save_baseline(&pool, &b).await.expect("save");
    let loaded = load_baseline(&pool, "w-1", 14).await.expect("load");
    assert_eq!(loaded.sample_count, b.sample_count);
    assert!((loaded.mean_usd_per_hour - b.mean_usd_per_hour).abs() < 1e-9);
    assert!((loaded.m2_usd_per_hour - b.m2_usd_per_hour).abs() < 1e-9);
}

#[tokio::test]
async fn record_and_list_anomaly_round_trip() {
    let pool = fresh_pool().await;
    let id = record_anomaly(&pool, "w-1", 50.0, 5.0, 1.5, 30.0, 14, 1_000)
        .await
        .expect("record");
    assert!(!id.is_empty());
    let rows = list_anomalies(
        &pool,
        &AnomalyFilters {
            worker_id: Some("w-1"),
            since_micros: None,
        },
        100,
    )
    .await
    .expect("list");
    assert_eq!(rows.len(), 1);
    let row = &rows[0];
    assert_eq!(row.worker_id, "w-1");
    assert_eq!(row.hour_of_day, 14);
    assert!((row.observed_usd_per_hour - 50.0).abs() < 1e-9);
    assert!((row.z_score - 30.0).abs() < 1e-9);
}

#[tokio::test]
async fn list_anomalies_filters_by_worker_and_since() {
    let pool = fresh_pool().await;
    record_anomaly(&pool, "w-1", 10.0, 1.0, 1.0, 9.0, 0, 1_000_000)
        .await
        .expect("rec1");
    record_anomaly(&pool, "w-2", 20.0, 1.0, 1.0, 19.0, 0, 2_000_000)
        .await
        .expect("rec2");
    record_anomaly(&pool, "w-1", 30.0, 1.0, 1.0, 29.0, 0, 3_000_000)
        .await
        .expect("rec3");
    let only_w1 = list_anomalies(
        &pool,
        &AnomalyFilters {
            worker_id: Some("w-1"),
            since_micros: None,
        },
        100,
    )
    .await
    .expect("list");
    assert_eq!(only_w1.len(), 2);
    assert!(only_w1.iter().all(|a| a.worker_id == "w-1"));
    let recent = list_anomalies(
        &pool,
        &AnomalyFilters {
            worker_id: None,
            since_micros: Some(2_500_000),
        },
        100,
    )
    .await
    .expect("list");
    assert_eq!(recent.len(), 1);
    assert_eq!(recent[0].worker_id, "w-1");
    assert!((recent[0].observed_usd_per_hour - 30.0).abs() < 1e-9);
}

#[tokio::test]
async fn observe_recent_hour_sums_worker_budget_in_window() {
    let pool = fresh_pool().await;
    // Set a tenant so worker_budget rows can map to one (we can
    // inject fake tenant rows the same way).
    sqlx::query(
        "INSERT INTO worker_budget (tenant_id, worker_id, action_id, usd_micros, charged_at) \
         VALUES (?, 'w-1', 'a-1', 500000, ?)",
    )
    .bind(vec![0u8; 16])
    .bind(
        Utc.with_ymd_and_hms(2026, 5, 7, 13, 30, 0)
            .unwrap()
            .timestamp(),
    )
    .execute(&pool)
    .await
    .expect("seed budget");
    // Charge inside the prev-hour window.
    sqlx::query(
        "INSERT INTO worker_budget (tenant_id, worker_id, action_id, usd_micros, charged_at) \
         VALUES (?, 'w-1', 'a-2', 500000, ?)",
    )
    .bind(vec![0u8; 16])
    .bind(
        Utc.with_ymd_and_hms(2026, 5, 7, 13, 45, 0)
            .unwrap()
            .timestamp(),
    )
    .execute(&pool)
    .await
    .expect("seed budget 2");
    // Outside the window (at the boundary of next hour) — must not count.
    sqlx::query(
        "INSERT INTO worker_budget (tenant_id, worker_id, action_id, usd_micros, charged_at) \
         VALUES (?, 'w-1', 'a-3', 999999, ?)",
    )
    .bind(vec![0u8; 16])
    .bind(
        Utc.with_ymd_and_hms(2026, 5, 7, 14, 0, 0)
            .unwrap()
            .timestamp(),
    )
    .execute(&pool)
    .await
    .expect("seed budget 3");
    let now = Utc.with_ymd_and_hms(2026, 5, 7, 14, 25, 0).unwrap();
    let (hod, dollars) = observe_recent_hour(&pool, "w-1", now)
        .await
        .expect("observe");
    assert_eq!(hod, 13);
    assert!((dollars - 1.0).abs() < 1e-9, "expected $1.00, got ${dollars}");
}

#[tokio::test]
async fn list_active_worker_ids_returns_distinct_rows() {
    let pool = fresh_pool().await;
    for (i, w) in ["w-1", "w-2", "w-1"].iter().enumerate() {
        sqlx::query(
            "INSERT INTO worker_budget (tenant_id, worker_id, action_id, usd_micros, charged_at) \
             VALUES (?, ?, ?, 1, 0)",
        )
        .bind(vec![0u8; 16])
        .bind(*w)
        .bind(format!("a-{i}"))
        .execute(&pool)
        .await
        .expect("seed");
    }
    let workers = list_active_worker_ids(&pool).await.expect("list");
    assert_eq!(workers, vec!["w-1", "w-2"]);
}

#[tokio::test]
async fn integration_full_tick_writes_anomaly_when_spend_spikes() {
    // Full chain: seed history, run one tick that updates baseline
    // and detects an anomaly when current-hour spend is far above
    // the baseline.
    let pool = fresh_pool().await;
    let now = Utc.with_ymd_and_hms(2026, 5, 7, 14, 30, 0).unwrap();

    // Seed 6 historical samples for hour 13: $1.00, $1.00, ... so
    // the baseline mean ≈ 1.0 with low stddev.
    let mut baseline = SpendBaseline::empty("w-spike", 13);
    for v in [1.0, 1.0, 1.0, 1.0, 1.0, 2.0] {
        baseline.update(v, 1_000);
    }
    save_baseline(&pool, &baseline).await.expect("seed baseline");

    // The spike: $50 in the last hour → way above $1 mean.
    let observed_dollars = 50.0;
    let mut updated = baseline.clone();
    updated.update(observed_dollars, now.timestamp_micros());
    save_baseline(&pool, &updated).await.expect("update baseline");

    // The pre-update baseline already had stddev > 0, so the
    // pre-update z-score on the spike fires.
    let z = baseline
        .z_score_for(observed_dollars)
        .expect("z computable");
    assert!(z > 4.0, "spike should clear |z|>4: z={z}");

    // Record the anomaly (cli would do this on tick).
    record_anomaly(
        &pool,
        "w-spike",
        observed_dollars,
        baseline.mean_usd_per_hour,
        baseline.stddev(),
        z,
        13,
        now.timestamp_micros(),
    )
    .await
    .expect("record");

    // CLI list path returns it.
    let rows = list_anomalies(
        &pool,
        &AnomalyFilters {
            worker_id: Some("w-spike"),
            since_micros: None,
        },
        100,
    )
    .await
    .expect("list");
    assert_eq!(rows.len(), 1);
    assert!(rows[0].z_score > 4.0);
    assert!((rows[0].observed_usd_per_hour - observed_dollars).abs() < 1e-9);
}

// chrono Timelike trait import for the previous_hour_bounds test
use chrono::Timelike;
