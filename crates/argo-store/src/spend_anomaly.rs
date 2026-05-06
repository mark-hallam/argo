//! Per-(worker, hour-of-day) spend baseline + z-score anomaly
//! detector (Stream `claude-spend-anomaly-alerts`).
//!
//! Sister of [`crate::anomaly`] (which baselines per-action
//! latency + retry behaviour from #847). This module specifically
//! covers SPEND: a worker burning USD faster than its historical
//! USD-per-hour baseline should ALERT before it actually breaches
//! the daily cap.
//!
//! ## Welford rolling stats
//!
//! Each (worker_id, hour_of_day) bucket keeps a running mean +
//! M2 (sum of squared deviations) so variance / stddev can be
//! computed without re-walking history. Update is O(1) per
//! sample; storage is O(workers × 24).
//!
//! ## Detection
//!
//! The hourly tick (driven by `argo budget anomalies tick`)
//! computes USD-per-hour for the most recent complete UTC hour
//! per active worker, folds it into the baseline, then checks
//! `|z| > ARGO_SPEND_ANOMALY_Z` (default 4.0). On exceedance, a
//! row lands in `worker_spend_anomaly` and the caller is
//! responsible for firing any NotifyChannel alerts (the cli
//! handler does this — keeping the store layer a pure data
//! sink).
use chrono::{DateTime, Datelike, TimeZone, Timelike, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

use argo_types::{ArgoError, ArgoResult};

const SPEND_BASELINE_SCHEMA: &str =
    include_str!("../migrations/26__spend_baseline.sql");
const SPEND_ANOMALY_SCHEMA: &str =
    include_str!("../migrations/27__spend_anomaly.sql");

/// Default `|z|` threshold above which a (worker, hour) sample
/// is flagged. `4.0` is intentionally aggressive — the
/// claude-anomaly-detector pattern already uses 3.0 for
/// behavioural metrics, and spend anomalies should trigger only
/// on genuinely unusual events (a per-hour USD spike that's 4σ
/// above the mean is a real outlier).
pub const DEFAULT_SPEND_Z_THRESHOLD: f64 = 4.0;

/// Minimum samples a baseline needs before z-score detection is
/// active. Below this, the tick still folds the sample into the
/// baseline but doesn't emit anomaly events.
pub const MIN_SAMPLES_FOR_DETECT: u64 = 5;

/// Read the spend-anomaly threshold from `$ARGO_SPEND_ANOMALY_Z`
/// or fall back to [`DEFAULT_SPEND_Z_THRESHOLD`]. Non-finite or
/// non-positive values silently fall back too.
#[must_use]
pub fn configured_threshold() -> f64 {
    std::env::var("ARGO_SPEND_ANOMALY_Z")
        .ok()
        .and_then(|s| s.parse::<f64>().ok())
        .filter(|v| v.is_finite() && *v > 0.0)
        .unwrap_or(DEFAULT_SPEND_Z_THRESHOLD)
}

/// Welford rolling-stats baseline for one (worker, hour-of-day).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpendBaseline {
    /// Worker the baseline tracks.
    pub worker_id: String,
    /// 0..=23 UTC hour-of-day this baseline summarises.
    pub hour_of_day: u8,
    /// Welford running mean of USD-per-hour spend.
    pub mean_usd_per_hour: f64,
    /// Welford running M2 (sum of squared deviations from the
    /// running mean). Variance is `m2 / (n-1)` for `n >= 2`,
    /// `0` otherwise; stddev is `sqrt(variance)`.
    pub m2_usd_per_hour: f64,
    /// Number of samples folded into this baseline.
    pub sample_count: u64,
    /// UTC microseconds at which this baseline was last touched.
    pub updated_at_micros: i64,
}

impl SpendBaseline {
    /// Empty baseline at hour `h` for worker `worker_id`.
    #[must_use]
    pub fn empty(worker_id: &str, hour_of_day: u8) -> Self {
        Self {
            worker_id: worker_id.to_string(),
            hour_of_day,
            mean_usd_per_hour: 0.0,
            m2_usd_per_hour: 0.0,
            sample_count: 0,
            updated_at_micros: 0,
        }
    }

    /// Standard deviation derived from M2 + sample_count. `0.0`
    /// for fewer than 2 samples (variance undefined).
    #[must_use]
    pub fn stddev(&self) -> f64 {
        if self.sample_count < 2 {
            return 0.0;
        }
        let n = (self.sample_count - 1) as f64;
        (self.m2_usd_per_hour / n).sqrt()
    }

    /// Fold one observed sample into the rolling stats (Welford's
    /// online algorithm). Mutates in place; pure function over
    /// (mean, m2, count).
    pub fn update(&mut self, observed_usd_per_hour: f64, now_micros: i64) {
        self.sample_count = self.sample_count.saturating_add(1);
        let n = self.sample_count as f64;
        let delta = observed_usd_per_hour - self.mean_usd_per_hour;
        self.mean_usd_per_hour += delta / n;
        let delta2 = observed_usd_per_hour - self.mean_usd_per_hour;
        self.m2_usd_per_hour += delta * delta2;
        self.updated_at_micros = now_micros;
    }

    /// Compute z-score for an observed sample against this
    /// baseline. Returns `None` when the baseline has fewer than
    /// [`MIN_SAMPLES_FOR_DETECT`] samples (detection inactive)
    /// OR when stddev is 0 (no variance to score against — first
    /// few samples or all-equal history).
    #[must_use]
    pub fn z_score_for(&self, observed_usd_per_hour: f64) -> Option<f64> {
        if self.sample_count < MIN_SAMPLES_FOR_DETECT {
            return None;
        }
        let stddev = self.stddev();
        if stddev == 0.0 {
            return None;
        }
        Some((observed_usd_per_hour - self.mean_usd_per_hour) / stddev)
    }
}

/// One persisted anomaly event row from `worker_spend_anomaly`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpendAnomaly {
    /// UUIDv7 anomaly id.
    pub id: String,
    /// Worker that triggered the anomaly.
    pub worker_id: String,
    /// USD-per-hour observed in the most recent hour bucket.
    pub observed_usd_per_hour: f64,
    /// Baseline mean at detection time (snapshot).
    pub baseline_mean_usd: f64,
    /// Baseline stddev at detection time (snapshot).
    pub baseline_stddev_usd: f64,
    /// `(observed - mean) / stddev` — sign preserved so callers
    /// can tell spike (+) from drought (−).
    pub z_score: f64,
    /// 0..=23 hour-of-day this anomaly fires for.
    pub hour_of_day: u8,
    /// UTC micros at which the anomaly was recorded.
    pub observed_at_micros: i64,
}

/// Ensure the baseline + anomaly tables exist (idempotent).
/// Runs both migrations inline so callers can construct a fresh
/// pool without coordinating with the global migration runner.
pub async fn ensure_schema(pool: &SqlitePool) -> ArgoResult<()> {
    sqlx::query(SPEND_BASELINE_SCHEMA)
        .execute(pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("spend baseline schema: {e}")))?;
    sqlx::query(SPEND_ANOMALY_SCHEMA)
        .execute(pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("spend anomaly schema: {e}")))?;
    Ok(())
}

/// Load one (worker, hour) baseline. Returns
/// [`SpendBaseline::empty`] when the row doesn't yet exist —
/// callers can immediately call [`SpendBaseline::update`] then
/// upsert via [`save_baseline`].
pub async fn load_baseline(
    pool: &SqlitePool,
    worker_id: &str,
    hour_of_day: u8,
) -> ArgoResult<SpendBaseline> {
    ensure_schema(pool).await?;
    let row: Option<(f64, f64, i64, i64)> = sqlx::query_as(
        "SELECT mean_usd_per_hour, m2_usd_per_hour, sample_count, updated_at_micros \
         FROM worker_spend_baseline WHERE worker_id = ? AND hour_of_day = ?",
    )
    .bind(worker_id)
    .bind(i64::from(hour_of_day))
    .fetch_optional(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("spend baseline load: {e}")))?;
    Ok(match row {
        Some((mean, m2, count, updated)) => SpendBaseline {
            worker_id: worker_id.to_string(),
            hour_of_day,
            mean_usd_per_hour: mean,
            m2_usd_per_hour: m2,
            sample_count: u64::try_from(count).unwrap_or(0),
            updated_at_micros: updated,
        },
        None => SpendBaseline::empty(worker_id, hour_of_day),
    })
}

/// Upsert a baseline row.
pub async fn save_baseline(pool: &SqlitePool, baseline: &SpendBaseline) -> ArgoResult<()> {
    ensure_schema(pool).await?;
    sqlx::query(
        "INSERT INTO worker_spend_baseline \
         (worker_id, hour_of_day, mean_usd_per_hour, m2_usd_per_hour, sample_count, updated_at_micros) \
         VALUES (?, ?, ?, ?, ?, ?) \
         ON CONFLICT(worker_id, hour_of_day) \
         DO UPDATE SET \
            mean_usd_per_hour = excluded.mean_usd_per_hour, \
            m2_usd_per_hour = excluded.m2_usd_per_hour, \
            sample_count = excluded.sample_count, \
            updated_at_micros = excluded.updated_at_micros",
    )
    .bind(&baseline.worker_id)
    .bind(i64::from(baseline.hour_of_day))
    .bind(baseline.mean_usd_per_hour)
    .bind(baseline.m2_usd_per_hour)
    .bind(i64::try_from(baseline.sample_count).unwrap_or(i64::MAX))
    .bind(baseline.updated_at_micros)
    .execute(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("spend baseline save: {e}")))?;
    Ok(())
}

/// Insert one anomaly event. Returns the inserted id so the
/// caller can include it in the notify payload.
#[allow(clippy::too_many_arguments)]
pub async fn record_anomaly(
    pool: &SqlitePool,
    worker_id: &str,
    observed_usd_per_hour: f64,
    baseline_mean_usd: f64,
    baseline_stddev_usd: f64,
    z_score: f64,
    hour_of_day: u8,
    now_micros: i64,
) -> ArgoResult<String> {
    ensure_schema(pool).await?;
    let id = Uuid::now_v7().to_string();
    sqlx::query(
        "INSERT INTO worker_spend_anomaly \
         (id, worker_id, observed_usd_per_hour, baseline_mean_usd, baseline_stddev_usd, z_score, hour_of_day, observed_at_micros) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(worker_id)
    .bind(observed_usd_per_hour)
    .bind(baseline_mean_usd)
    .bind(baseline_stddev_usd)
    .bind(z_score)
    .bind(i64::from(hour_of_day))
    .bind(now_micros)
    .execute(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("spend anomaly insert: {e}")))?;
    Ok(id)
}

/// Filter set for [`list_anomalies`]. Both fields default to
/// `None` (no filter); pass `since_micros` for "last N days /
/// hours / minutes" queries from the cli's `--since <duration>`
/// flag.
#[derive(Debug, Default, Clone)]
pub struct AnomalyFilters<'a> {
    /// Restrict to one worker.
    pub worker_id: Option<&'a str>,
    /// `observed_at_micros >= since_micros` floor.
    pub since_micros: Option<i64>,
}

/// Read recent anomaly rows ordered newest-first, capped at
/// `limit`.
pub async fn list_anomalies(
    pool: &SqlitePool,
    filters: &AnomalyFilters<'_>,
    limit: u32,
) -> ArgoResult<Vec<SpendAnomaly>> {
    ensure_schema(pool).await?;
    let mut sql = String::from(
        "SELECT id, worker_id, observed_usd_per_hour, baseline_mean_usd, \
                baseline_stddev_usd, z_score, hour_of_day, observed_at_micros \
         FROM worker_spend_anomaly",
    );
    let mut clauses: Vec<&str> = Vec::new();
    if filters.worker_id.is_some() {
        clauses.push("worker_id = ?");
    }
    if filters.since_micros.is_some() {
        clauses.push("observed_at_micros >= ?");
    }
    if !clauses.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&clauses.join(" AND "));
    }
    sql.push_str(" ORDER BY observed_at_micros DESC LIMIT ?");
    let mut q = sqlx::query_as::<_, (String, String, f64, f64, f64, f64, i64, i64)>(&sql);
    if let Some(w) = filters.worker_id {
        q = q.bind(w);
    }
    if let Some(s) = filters.since_micros {
        q = q.bind(s);
    }
    q = q.bind(i64::from(limit));
    let rows = q
        .fetch_all(pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("spend anomaly list: {e}")))?;
    Ok(rows
        .into_iter()
        .map(|(id, worker_id, observed, mean, stddev, z, hour, obs_at)| SpendAnomaly {
            id,
            worker_id,
            observed_usd_per_hour: observed,
            baseline_mean_usd: mean,
            baseline_stddev_usd: stddev,
            z_score: z,
            hour_of_day: u8::try_from(hour).unwrap_or(0),
            observed_at_micros: obs_at,
        })
        .collect())
}

/// Compute USD-per-hour spend for one worker in the most recent
/// complete UTC hour. Sums `worker_budget.usd_micros` rows
/// charged in `[hour_start, hour_end)` and divides by 1_000_000
/// to return dollars (per hour).
pub async fn observe_recent_hour(
    pool: &SqlitePool,
    worker_id: &str,
    now: DateTime<Utc>,
) -> ArgoResult<(u8, f64)> {
    let (hour_start, hour_end, hour_of_day) = previous_hour_bounds(now);
    let total: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(usd_micros), 0) FROM worker_budget \
         WHERE worker_id = ? AND charged_at >= ? AND charged_at < ?",
    )
    .bind(worker_id)
    .bind(hour_start)
    .bind(hour_end)
    .fetch_one(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("spend observe hour: {e}")))?;
    let dollars_per_hour = (total.max(0) as f64) / 1_000_000.0;
    Ok((hour_of_day, dollars_per_hour))
}

/// Discover all worker_ids that have charged anything in the
/// `worker_budget` table. Used by the cli tick to walk every
/// active worker without needing the IdentityStore handle.
pub async fn list_active_worker_ids(pool: &SqlitePool) -> ArgoResult<Vec<String>> {
    ensure_schema(pool).await?;
    let rows: Vec<(String,)> =
        sqlx::query_as("SELECT DISTINCT worker_id FROM worker_budget ORDER BY worker_id")
            .fetch_all(pool)
            .await
            .map_err(|e| ArgoError::Provider(format!("spend active workers: {e}")))?;
    Ok(rows.into_iter().map(|(w,)| w).collect())
}

/// Compute the [start, end) bounds for the *previous* complete
/// UTC hour relative to `now`. Returns (start_secs, end_secs,
/// hour_of_day).
#[must_use]
pub fn previous_hour_bounds(now: DateTime<Utc>) -> (i64, i64, u8) {
    let truncated = Utc
        .with_ymd_and_hms(now.year(), now.month(), now.day(), now.hour(), 0, 0)
        .single()
        .unwrap_or(now);
    let prev_start = truncated - chrono::Duration::hours(1);
    let prev_end = truncated;
    (
        prev_start.timestamp(),
        prev_end.timestamp(),
        u8::try_from(prev_start.hour()).unwrap_or(0),
    )
}

#[cfg(test)]
#[path = "spend_anomaly_tests.rs"]
mod tests;
