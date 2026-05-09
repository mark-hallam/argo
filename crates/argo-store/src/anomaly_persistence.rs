//! SQLite persistence for [`crate::anomaly`].
//!
//! Split out of `anomaly.rs` to keep the parent file under the
//! 500-LOC ceiling. Pure-Rust types and the in-memory detection
//! logic stay in `anomaly.rs`; this file owns the SQL.
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use uuid::Uuid;

use argo_types::{ArgoError, ArgoResult};

use crate::anomaly::{AnomalyEvent, BehaviorBaseline};

/// Read the persisted baseline. Returns empty when absent.
pub async fn load_baseline(
    pool: &SqlitePool,
    worker_id: &str,
    action_kind: &str,
) -> ArgoResult<BehaviorBaseline> {
    let row: Option<RawBaseline> = sqlx::query_as::<_, RawBaseline>(
        "SELECT worker_id, action_kind, latency_mean, latency_m2, \
         retry_mean, retry_m2, sample_count, updated_at \
         FROM worker_behavior_baseline \
         WHERE worker_id = ? AND action_kind = ?",
    )
    .bind(worker_id)
    .bind(action_kind)
    .fetch_optional(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("anomaly baseline load: {e}")))?;
    Ok(row
        .map(BehaviorBaseline::from)
        .unwrap_or_else(|| BehaviorBaseline::empty(worker_id, action_kind)))
}

/// Persist a baseline row.
pub async fn save_baseline(pool: &SqlitePool, b: &BehaviorBaseline) -> ArgoResult<()> {
    sqlx::query(
        "INSERT INTO worker_behavior_baseline \
         (worker_id, action_kind, latency_mean, latency_m2, \
          retry_mean, retry_m2, sample_count, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?) \
         ON CONFLICT(worker_id, action_kind) DO UPDATE SET \
             latency_mean = excluded.latency_mean, \
             latency_m2 = excluded.latency_m2, \
             retry_mean = excluded.retry_mean, \
             retry_m2 = excluded.retry_m2, \
             sample_count = excluded.sample_count, \
             updated_at = excluded.updated_at",
    )
    .bind(&b.worker_id)
    .bind(&b.action_kind)
    .bind(b.latency_mean)
    .bind(b.latency_m2)
    .bind(b.retry_mean)
    .bind(b.retry_m2)
    .bind(b.sample_count as i64)
    .bind(b.updated_at.timestamp_micros())
    .execute(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("anomaly baseline save: {e}")))?;
    Ok(())
}

/// Insert one anomaly event row. Returns the new id.
#[allow(clippy::too_many_arguments)]
pub async fn insert_event(
    pool: &SqlitePool,
    worker_id: &str,
    action_id: &str,
    action_kind: &str,
    dimension: &str,
    observed: f64,
    baseline: &BehaviorBaseline,
    z_score: f64,
    threshold: f64,
    now: DateTime<Utc>,
) -> ArgoResult<String> {
    let id = Uuid::now_v7();
    let mean = if dimension == "latency_ms" {
        baseline.latency_mean
    } else {
        baseline.retry_mean
    };
    let stddev = if dimension == "latency_ms" {
        baseline.latency_stddev().unwrap_or(0.0)
    } else {
        baseline.retry_stddev().unwrap_or(0.0)
    };
    sqlx::query(
        "INSERT INTO worker_anomaly_event \
         (id, worker_id, action_id, action_kind, dimension, observed, \
          baseline_mean, baseline_stddev, z_score, threshold, sample_count, \
          observed_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id.as_bytes().as_slice())
    .bind(worker_id)
    .bind(action_id)
    .bind(action_kind)
    .bind(dimension)
    .bind(observed)
    .bind(mean)
    .bind(stddev)
    .bind(z_score)
    .bind(threshold)
    .bind(baseline.sample_count as i64)
    .bind(now.timestamp_micros())
    .execute(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("anomaly event insert: {e}")))?;
    Ok(id.to_string())
}

/// List recent anomaly events for one worker, newest first.
pub async fn list_for_worker(
    pool: &SqlitePool,
    worker_id: &str,
    since_micros: Option<i64>,
    limit: Option<u32>,
) -> ArgoResult<Vec<AnomalyEvent>> {
    let lim = limit.unwrap_or(100).min(1000);
    let rows: Vec<RawEvent> = match since_micros {
        Some(since) => sqlx::query_as::<_, RawEvent>(
            "SELECT id, worker_id, action_id, action_kind, dimension, observed, \
             baseline_mean, baseline_stddev, z_score, threshold, sample_count, \
             observed_at FROM worker_anomaly_event \
             WHERE worker_id = ? AND observed_at >= ? \
             ORDER BY observed_at DESC LIMIT ?",
        )
        .bind(worker_id)
        .bind(since)
        .bind(lim as i64),
        None => sqlx::query_as::<_, RawEvent>(
            "SELECT id, worker_id, action_id, action_kind, dimension, observed, \
             baseline_mean, baseline_stddev, z_score, threshold, sample_count, \
             observed_at FROM worker_anomaly_event \
             WHERE worker_id = ? \
             ORDER BY observed_at DESC LIMIT ?",
        )
        .bind(worker_id)
        .bind(lim as i64),
    }
    .fetch_all(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("anomaly event list: {e}")))?;
    rows.into_iter().map(AnomalyEvent::try_from).collect()
}

#[derive(sqlx::FromRow)]
struct RawBaseline {
    worker_id: String,
    action_kind: String,
    latency_mean: f64,
    latency_m2: f64,
    retry_mean: f64,
    retry_m2: f64,
    sample_count: i64,
    updated_at: i64,
}

impl From<RawBaseline> for BehaviorBaseline {
    fn from(r: RawBaseline) -> Self {
        Self {
            worker_id: r.worker_id,
            action_kind: r.action_kind,
            latency_mean: r.latency_mean,
            latency_m2: r.latency_m2,
            retry_mean: r.retry_mean,
            retry_m2: r.retry_m2,
            sample_count: r.sample_count.max(0) as u64,
            updated_at: DateTime::<Utc>::from_timestamp_micros(r.updated_at)
                .unwrap_or_else(Utc::now),
        }
    }
}

#[derive(sqlx::FromRow)]
struct RawEvent {
    id: Vec<u8>,
    worker_id: String,
    action_id: String,
    action_kind: String,
    dimension: String,
    observed: f64,
    baseline_mean: f64,
    baseline_stddev: f64,
    z_score: f64,
    threshold: f64,
    sample_count: i64,
    observed_at: i64,
}

impl TryFrom<RawEvent> for AnomalyEvent {
    type Error = ArgoError;
    fn try_from(r: RawEvent) -> ArgoResult<Self> {
        let id = Uuid::from_slice(&r.id)
            .map_err(|e| ArgoError::Schema(format!("anomaly id uuid: {e}")))?;
        let observed_at = DateTime::<Utc>::from_timestamp_micros(r.observed_at)
            .ok_or_else(|| ArgoError::Schema(format!("anomaly observed_at: {}", r.observed_at)))?;
        Ok(Self {
            id: id.to_string(),
            worker_id: r.worker_id,
            action_id: r.action_id,
            action_kind: r.action_kind,
            dimension: r.dimension,
            observed: r.observed,
            baseline_mean: r.baseline_mean,
            baseline_stddev: r.baseline_stddev,
            z_score: r.z_score,
            threshold: r.threshold,
            sample_count: r.sample_count.max(0) as u64,
            observed_at,
        })
    }
}
