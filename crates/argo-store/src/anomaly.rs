//! Per-(worker, action_kind) behavioural baselines + anomaly
//! event log (Stream `claude-anomaly-detector`). Welford running
//! stats + z-score detection (env: `$ARGO_ANOMALY_Z_THRESHOLD`).
//!
//! Schema is owned by migration `v19_anomaly` (in
//! `migrations/v19.rs`). Callers are expected to run
//! `SessionStore::migrate()` before using this module.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

use argo_types::{ArgoError, ArgoResult};

/// Default `|z|` threshold above which a sample is flagged.
pub const DEFAULT_Z_THRESHOLD: f64 = 3.0;

/// No-op shim retained for callers in crates outside the
/// may-touch list (e.g. `argo-memory` tests). The anomaly
/// schema is now installed by [`crate::SessionStore::migrate`]
/// via `migrations/v19.rs`; callers that need the tables should
/// call `migrate` instead. This shim returns `Ok(())` so existing
/// call sites compile without churn.
#[allow(clippy::missing_errors_doc)]
pub async fn ensure_schema(_pool: &SqlitePool) -> ArgoResult<()> {
    Ok(())
}

/// Minimum baseline sample count before z-score detection emits.
pub const MIN_SAMPLES_FOR_DETECT: u64 = 5;

/// Read the anomaly threshold from env or fall back to default.
#[must_use]
pub fn configured_threshold() -> f64 {
    std::env::var("ARGO_ANOMALY_Z_THRESHOLD")
        .ok()
        .and_then(|s| s.parse::<f64>().ok())
        .filter(|v| v.is_finite() && *v > 0.0)
        .unwrap_or(DEFAULT_Z_THRESHOLD)
}

/// Per-(worker, action_kind) Welford running stats.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BehaviorBaseline {
    /// Worker the baseline tracks.
    pub worker_id: String,
    /// Action kind (e.g. `post_listing`).
    pub action_kind: String,
    /// Welford running mean of `latency_ms`.
    pub latency_mean: f64,
    /// Welford running M2 for `latency_ms`.
    pub latency_m2: f64,
    /// Welford running mean of `retry_count`.
    pub retry_mean: f64,
    /// Welford M2 for `retry_count`.
    pub retry_m2: f64,
    /// Samples folded into this baseline.
    pub sample_count: u64,
    /// Last update.
    pub updated_at: DateTime<Utc>,
}

impl BehaviorBaseline {
    /// Empty baseline.
    #[must_use]
    pub fn empty(worker_id: &str, action_kind: &str) -> Self {
        Self {
            worker_id: worker_id.to_string(),
            action_kind: action_kind.to_string(),
            latency_mean: 0.0,
            latency_m2: 0.0,
            retry_mean: 0.0,
            retry_m2: 0.0,
            sample_count: 0,
            updated_at: Utc::now(),
        }
    }

    /// Population variance for `latency_ms`. None when n < 2.
    #[must_use]
    pub fn latency_variance(&self) -> Option<f64> {
        (self.sample_count >= 2).then(|| self.latency_m2 / self.sample_count as f64)
    }

    /// Stddev for `latency_ms`.
    #[must_use]
    pub fn latency_stddev(&self) -> Option<f64> {
        self.latency_variance().map(f64::sqrt)
    }

    /// Variance for `retry_count`.
    #[must_use]
    pub fn retry_variance(&self) -> Option<f64> {
        (self.sample_count >= 2).then(|| self.retry_m2 / self.sample_count as f64)
    }

    /// Stddev for `retry_count`.
    #[must_use]
    pub fn retry_stddev(&self) -> Option<f64> {
        self.retry_variance().map(f64::sqrt)
    }

    /// Welford fold one sample into the baseline.
    pub fn fold(&mut self, sample: &BehaviorSample, now: DateTime<Utc>) {
        self.sample_count += 1;
        let n = self.sample_count as f64;
        let lx = sample.latency_ms as f64;
        let l_delta = lx - self.latency_mean;
        self.latency_mean += l_delta / n;
        self.latency_m2 += l_delta * (lx - self.latency_mean);
        let rx = sample.retry_count as f64;
        let r_delta = rx - self.retry_mean;
        self.retry_mean += r_delta / n;
        self.retry_m2 += r_delta * (rx - self.retry_mean);
        self.updated_at = now;
    }

    /// Z-scores against the CURRENT baseline (BEFORE folding).
    #[must_use]
    pub fn z_scores(&self, sample: &BehaviorSample) -> (Option<f64>, Option<f64>) {
        let l_z = self.latency_stddev().and_then(|s| {
            (s > 0.0).then(|| (sample.latency_ms as f64 - self.latency_mean) / s)
        });
        let r_z = self.retry_stddev().and_then(|s| {
            (s > 0.0).then(|| (sample.retry_count as f64 - self.retry_mean) / s)
        });
        (l_z, r_z)
    }
}

/// One behavioural sample drawn from a completed action.
#[derive(Debug, Clone, Copy)]
pub struct BehaviorSample {
    /// End-to-end latency in milliseconds.
    pub latency_ms: i64,
    /// Number of retries before completion.
    pub retry_count: u32,
}

/// One anomaly row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyEvent {
    /// Stable event id (UUIDv7).
    pub id: String,
    /// Worker the event was attributed to.
    pub worker_id: String,
    /// Action that triggered the event.
    pub action_id: String,
    /// Action kind.
    pub action_kind: String,
    /// Dimension (`latency_ms` / `retry_count`).
    pub dimension: String,
    /// Observed value.
    pub observed: f64,
    /// Baseline mean at detection.
    pub baseline_mean: f64,
    /// Baseline stddev at detection.
    pub baseline_stddev: f64,
    /// Signed z-score.
    pub z_score: f64,
    /// `|z|` threshold that fired.
    pub threshold: f64,
    /// Baseline sample count at detection.
    pub sample_count: u64,
    /// When the event was recorded.
    pub observed_at: DateTime<Utc>,
}

/// One-shot: load → check → fold → save → maybe-emit.
pub async fn record_sample(
    pool: &SqlitePool,
    worker_id: &str,
    action_id: &str,
    action_kind: &str,
    sample: BehaviorSample,
    threshold: f64,
    now: DateTime<Utc>,
) -> ArgoResult<Vec<AnomalyEvent>> {
    let mut baseline = load_baseline(pool, worker_id, action_kind).await?;
    let mut emitted = Vec::new();
    if baseline.sample_count >= MIN_SAMPLES_FOR_DETECT {
        let (l_z, r_z) = baseline.z_scores(&sample);
        if let Some(z) = l_z {
            if z.abs() >= threshold {
                emitted.push(
                    emit_event(
                        pool,
                        worker_id,
                        action_id,
                        action_kind,
                        "latency_ms",
                        sample.latency_ms as f64,
                        &baseline,
                        z,
                        threshold,
                        now,
                    )
                    .await?,
                );
            }
        }
        if let Some(z) = r_z {
            if z.abs() >= threshold {
                emitted.push(
                    emit_event(
                        pool,
                        worker_id,
                        action_id,
                        action_kind,
                        "retry_count",
                        sample.retry_count as f64,
                        &baseline,
                        z,
                        threshold,
                        now,
                    )
                    .await?,
                );
            }
        }
    }
    baseline.fold(&sample, now);
    save_baseline(pool, &baseline).await?;
    Ok(emitted)
}

#[allow(clippy::too_many_arguments)]
async fn emit_event(
    pool: &SqlitePool,
    worker_id: &str,
    action_id: &str,
    action_kind: &str,
    dimension: &str,
    observed: f64,
    baseline: &BehaviorBaseline,
    z: f64,
    threshold: f64,
    now: DateTime<Utc>,
) -> ArgoResult<AnomalyEvent> {
    let id = insert_event(
        pool, worker_id, action_id, action_kind, dimension, observed, baseline, z, threshold, now,
    )
    .await?;
    let (mean, stddev) = if dimension == "latency_ms" {
        (baseline.latency_mean, baseline.latency_stddev().unwrap_or(0.0))
    } else {
        (baseline.retry_mean, baseline.retry_stddev().unwrap_or(0.0))
    };
    Ok(AnomalyEvent {
        id,
        worker_id: worker_id.to_string(),
        action_id: action_id.to_string(),
        action_kind: action_kind.to_string(),
        dimension: dimension.to_string(),
        observed,
        baseline_mean: mean,
        baseline_stddev: stddev,
        z_score: z,
        threshold,
        sample_count: baseline.sample_count,
        observed_at: now,
    })
}

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

#[cfg(test)]
#[path = "anomaly_tests.rs"]
mod tests;
