//! Per-(worker, action_kind) behavioural baselines + anomaly
//! event log (Stream `claude-anomaly-detector`). Welford running
//! stats + z-score detection (env: `$ARGO_ANOMALY_Z_THRESHOLD`).
//!
//! Schema is owned by migration `v19_anomaly` (in
//! `migrations/v19.rs`). Callers are expected to run
//! `SessionStore::migrate()` before using this module.
//!
//! SQLite persistence helpers live in
//! [`crate::anomaly_persistence`] and are re-exported here so the
//! public API at `argo_store::anomaly::*` is unchanged.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use argo_types::ArgoResult;

pub use crate::anomaly_persistence::{
    insert_event, list_for_worker, load_baseline, save_baseline,
};

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

#[cfg(test)]
#[path = "anomaly_tests.rs"]
mod tests;
