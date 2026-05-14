//! Per-executor-kind action latency tracker — power-of-2 ms
//! histogram with p50/p95/p99 + max readout (Stream
//! `claude-latency-percentiles-tracking`).
//!
//! Sister of [`crate::anomaly`] (which baselines per-action latency
//! for anomaly detection). This module is the *visibility* surface:
//! every `Executor::dispatch` call records a duration into a
//! per-executor-kind histogram so operators can answer "how slow
//! is the slow path" without wading through audit logs.
//!
//! ## Bucketing
//!
//! Buckets are power-of-2 milliseconds, indexed `0..N_BUCKETS`.
//! Bucket `i` for `i < N_BUCKETS - 1` covers the half-open range
//! `[2^i ms, 2^(i+1) ms)`; the last bucket is the `+Inf` overflow.
//! Anything `< 1ms` rounds up into bucket 0. With `N_BUCKETS = 16`
//! this covers `1..16384ms` plus `>= 16384ms` overflow — enough
//! resolution for everything short of a multi-minute action while
//! keeping the schema tiny (16 rows per kind).
//!
//! ## Percentile math
//!
//! Histogram percentiles are *interpolation-within-bucket*: locate
//! the bucket containing the p-th sample, then linearly interpolate
//! between the bucket's lower and upper boundaries assuming uniform
//! distribution inside it. The overflow bucket has no upper bound;
//! we return its lower bound (i.e. p99 saturates at
//! `2^(N_BUCKETS - 1) ms` once the overflow bucket fires).
//!
//! ## Persistence
//!
//! `latency_buckets(executor_kind, bucket_idx, count)` holds the
//! rolling totals. [`flush_to_store`] is *additive*:
//! `INSERT … ON CONFLICT DO UPDATE SET count = count + excluded`.
//! Pair `drain()` + `flush_to_store()` so already-flushed samples
//! don't double-count.

use sqlx::{Row, SqlitePool};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use argo_types::{ArgoError, ArgoResult};

/// Number of power-of-2 ms buckets, including the `+Inf` overflow.
/// Sized to cover `1ms..16384ms` plus an `>= 16384ms` overflow
/// bucket — see module docs.
pub const N_BUCKETS: usize = 16;

/// CREATE TABLE statement for the persistence layer. Kept inline
/// (not in `migrations/`) so the module is self-contained and the
/// brief stays inside its may-touch list. Mirrors the
/// `spend_anomaly::ensure_schema` pattern.
const CREATE_LATENCY_BUCKETS_SQL: &str = "
CREATE TABLE IF NOT EXISTS latency_buckets (
    executor_kind TEXT NOT NULL,
    bucket_idx    INTEGER NOT NULL,
    count         INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (executor_kind, bucket_idx)
)
";

/// Compute the bucket index for a duration. Anything `< 1ms` rounds
/// to bucket 0; durations at or above the final bucket boundary
/// land in the `+Inf` overflow bucket.
#[must_use]
pub fn bucket_index(dur: Duration) -> usize {
    let ms = dur.as_millis();
    if ms < 1 {
        return 0;
    }
    let leading = (ms as u64).leading_zeros();
    let log2 = (63 - leading) as usize;
    if log2 >= N_BUCKETS - 1 {
        N_BUCKETS - 1
    } else {
        log2
    }
}

/// Lower bound of bucket `i` in milliseconds. Bucket 0 starts at
/// `1ms` (any sample `< 1ms` already rounded into it).
#[must_use]
pub fn bucket_lower_ms(i: usize) -> u64 {
    1u64 << i
}

/// Upper bound of bucket `i` in milliseconds. The overflow bucket
/// has no real upper bound; we return its lower bound so callers
/// that interpolate can saturate gracefully.
#[must_use]
pub fn bucket_upper_ms(i: usize) -> u64 {
    if i >= N_BUCKETS - 1 {
        1u64 << (N_BUCKETS - 1)
    } else {
        1u64 << (i + 1)
    }
}

/// In-memory per-(executor_kind) histogram. Cheap to clone (single
/// `Arc<Mutex<...>>`) so the dispatch wrapper and the periodic
/// flush task can share one tracker.
#[derive(Debug, Clone, Default)]
pub struct LatencyTracker {
    inner: Arc<Mutex<HashMap<String, [u64; N_BUCKETS]>>>,
}

impl LatencyTracker {
    /// Construct a fresh, empty tracker.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record one sample. `kind` is the stable executor-kind tag
    /// (e.g. `"human"`, `"mcp"`, `"computer_use"`).
    pub fn record(&self, kind: &str, dur: Duration) {
        let idx = bucket_index(dur);
        let Ok(mut guard) = self.inner.lock() else {
            // Poisoned mutex — drop the sample rather than panic.
            // Latency tracking is best-effort observability.
            return;
        };
        let entry = guard.entry(kind.to_string()).or_insert([0u64; N_BUCKETS]);
        entry[idx] = entry[idx].saturating_add(1);
    }

    /// Snapshot the current counts per kind. Returns a fresh
    /// `HashMap` clone so the caller can iterate without holding
    /// the mutex.
    #[must_use]
    pub fn snapshot(&self) -> HashMap<String, [u64; N_BUCKETS]> {
        match self.inner.lock() {
            Ok(guard) => guard.clone(),
            Err(_) => HashMap::new(),
        }
    }

    /// Atomic snapshot-and-reset — primary entry point for the
    /// periodic flush loop. The persisted table is the cumulative
    /// source of truth, so the in-memory map is cleared after each
    /// drain.
    pub fn drain(&self) -> HashMap<String, [u64; N_BUCKETS]> {
        match self.inner.lock() {
            Ok(mut guard) => std::mem::take(&mut *guard),
            Err(_) => HashMap::new(),
        }
    }
}

/// Process-wide tracker singleton. Initialised lazily on first
/// access. The dispatch wrapper records here; the CLI handler and
/// the periodic flush task share the same instance.
pub fn global() -> Arc<LatencyTracker> {
    static GLOBAL: OnceLock<Arc<LatencyTracker>> = OnceLock::new();
    GLOBAL
        .get_or_init(|| Arc::new(LatencyTracker::new()))
        .clone()
}

/// Install the `latency_buckets` schema if absent. Idempotent;
/// safe to call from any process that owns the pool. The table is
/// intentionally outside the structured migration runner so the
/// module owns its own DDL (mirrors the `spend_anomaly` /
/// `worker_budget` pattern).
pub async fn ensure_schema(pool: &SqlitePool) -> ArgoResult<()> {
    sqlx::query(CREATE_LATENCY_BUCKETS_SQL)
        .execute(pool)
        .await
        .map_err(|e| ArgoError::Io(format!("create latency_buckets: {e}")))?;
    Ok(())
}

/// Additively persist a snapshot's counts. Each `(kind, bucket_idx)`
/// row is upserted with `count = count + ?`. The caller passes the
/// exact map returned from [`LatencyTracker::drain`] so already-
/// flushed samples don't double-count.
pub async fn flush_to_store(
    pool: &SqlitePool,
    snapshot: &HashMap<String, [u64; N_BUCKETS]>,
) -> ArgoResult<()> {
    if snapshot.is_empty() {
        return Ok(());
    }
    ensure_schema(pool).await?;
    for (kind, buckets) in snapshot {
        for (idx, count) in buckets.iter().enumerate() {
            if *count == 0 {
                continue;
            }
            let count_i = i64::try_from(*count).unwrap_or(i64::MAX);
            let idx_i = i64::try_from(idx).unwrap_or(0);
            sqlx::query(
                "INSERT INTO latency_buckets (executor_kind, bucket_idx, count) \
                 VALUES (?1, ?2, ?3) \
                 ON CONFLICT(executor_kind, bucket_idx) \
                 DO UPDATE SET count = count + excluded.count",
            )
            .bind(kind)
            .bind(idx_i)
            .bind(count_i)
            .execute(pool)
            .await
            .map_err(|e| ArgoError::Provider(format!("flush latency bucket: {e}")))?;
        }
    }
    Ok(())
}

/// Read the persisted bucket totals into a kind→[counts] map.
pub async fn load_persisted(
    pool: &SqlitePool,
) -> ArgoResult<HashMap<String, [u64; N_BUCKETS]>> {
    ensure_schema(pool).await?;
    let rows = sqlx::query(
        "SELECT executor_kind, bucket_idx, count \
         FROM latency_buckets \
         WHERE count > 0 \
         ORDER BY executor_kind, bucket_idx",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("load latency buckets: {e}")))?;
    let mut out: HashMap<String, [u64; N_BUCKETS]> = HashMap::new();
    for row in rows {
        let kind: String = row.get("executor_kind");
        let idx: i64 = row.get("bucket_idx");
        let count: i64 = row.get("count");
        if !(0..N_BUCKETS as i64).contains(&idx) || count <= 0 {
            continue;
        }
        let entry = out.entry(kind).or_insert([0u64; N_BUCKETS]);
        entry[idx as usize] = u64::try_from(count).unwrap_or(0);
    }
    Ok(out)
}

/// Compute the p-th percentile of a histogram by
/// interpolation-within-bucket. `p` is in `0.0..=1.0`; values
/// outside that range are clamped. Returns `None` if the histogram
/// is empty.
#[must_use]
pub fn percentile(buckets: &[u64; N_BUCKETS], p: f64) -> Option<Duration> {
    let total: u64 = buckets.iter().sum();
    if total == 0 {
        return None;
    }
    let p = p.clamp(0.0, 1.0);
    // Rank is the 1-based index of the target sample. ceil() so
    // p=0.5 over an even count picks the upper of the two medians,
    // matching the common "nearest-rank" convention.
    let rank = ((p * total as f64).ceil() as u64).max(1).min(total);
    let mut cumulative: u64 = 0;
    for (i, count) in buckets.iter().enumerate() {
        if *count == 0 {
            continue;
        }
        let next = cumulative + count;
        if rank <= next {
            let in_bucket = rank - cumulative;
            let lower = bucket_lower_ms(i) as f64;
            let upper = bucket_upper_ms(i) as f64;
            // Saturate at the lower bound for the overflow bucket
            // (no real upper bound to interpolate against).
            let interp_ms = if i == N_BUCKETS - 1 {
                lower
            } else {
                let frac = (in_bucket as f64 - 0.5) / *count as f64;
                lower + frac.clamp(0.0, 1.0) * (upper - lower)
            };
            return Some(Duration::from_micros((interp_ms * 1000.0) as u64));
        }
        cumulative = next;
    }
    None
}

/// Highest non-empty bucket's upper bound — used as the "max"
/// figure in the percentile readout. Returns the overflow bucket's
/// lower bound when the overflow bucket is the highest occupied.
#[must_use]
pub fn max_observed(buckets: &[u64; N_BUCKETS]) -> Option<Duration> {
    for i in (0..N_BUCKETS).rev() {
        if buckets[i] > 0 {
            let ms = if i == N_BUCKETS - 1 {
                bucket_lower_ms(i)
            } else {
                bucket_upper_ms(i)
            };
            return Some(Duration::from_millis(ms));
        }
    }
    None
}

/// Total number of samples in the histogram.
#[must_use]
pub fn total_samples(buckets: &[u64; N_BUCKETS]) -> u64 {
    buckets.iter().sum()
}

#[cfg(test)]
#[path = "latency_tracker_tests.rs"]
mod tests;
