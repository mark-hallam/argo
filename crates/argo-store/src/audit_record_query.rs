//! Read-side queries over `audit_record` HTTP request rows.
//!
//! These helpers intentionally reuse the append-only audit chain
//! instead of introducing a parallel request-log table.

use crate::{SessionStore, StoreResult};
use argo_types::ArgoError;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::collections::BTreeMap;

/// Optional filters for [`error_rate_in_window_filtered`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ErrorRateFilter {
    /// Exact tenant id match against `payload_json.tenant_id`.
    pub tenant_id: Option<String>,
    /// Route/path filter. A trailing `*` is treated as a prefix.
    pub route: Option<String>,
}

/// Availability/SLA snapshot for one audit window.
///
/// `success_count` is the subset of `total` whose HTTP status was
/// < 500 (anything 4xx-and-below is treated as a successful response
/// from the SLO perspective).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AvailabilitySnapshot {
    /// Window start, UTC epoch microseconds.
    pub start_ts: i64,
    /// Window end, UTC epoch microseconds.
    pub end_ts: i64,
    /// Count of request rows with a parseable status code.
    pub total: u64,
    /// Count of rows whose `status_code < 500`.
    pub success_count: u64,
}

impl AvailabilitySnapshot {
    /// Successful-response percentage for the window.
    #[must_use]
    pub fn success_pct(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            (self.success_count as f64 / self.total as f64) * 100.0
        }
    }
}

/// Latency percentiles (microseconds) over one audit window.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LatencyPercentiles {
    /// Window start, UTC epoch microseconds.
    pub start_ts: i64,
    /// Window end, UTC epoch microseconds.
    pub end_ts: i64,
    /// Total samples considered.
    pub total: u64,
    /// 50th-percentile latency, microseconds.
    pub p50_us: u64,
    /// 95th-percentile latency, microseconds.
    pub p95_us: u64,
    /// 99th-percentile latency, microseconds.
    pub p99_us: u64,
}

/// HTTP error-rate snapshot for one audit window.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ErrorRateSnapshot {
    /// Window start, UTC epoch microseconds.
    pub start_ts: i64,
    /// Window end, UTC epoch microseconds.
    pub end_ts: i64,
    /// Count of request rows with a parseable status code.
    pub total: u64,
    /// Count of HTTP 5xx request rows.
    pub errors: u64,
    /// Histogram keyed by HTTP status code.
    pub per_status_histogram: BTreeMap<u16, u64>,
}

impl ErrorRateSnapshot {
    /// Error percentage in the window.
    #[must_use]
    pub fn error_rate_pct(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            (self.errors as f64 / self.total as f64) * 100.0
        }
    }
}

/// Aggregate `http.request` audit rows across a time window.
///
/// ```
/// # async fn demo(store: &argo_store::SessionStore) -> argo_store::StoreResult<()> {
/// let snapshot = argo_store::audit_record_query::error_rate_in_window(
///     store,
///     0,
///     i64::MAX,
/// ).await?;
/// assert!(snapshot.total >= snapshot.errors);
/// # Ok(()) }
/// ```
pub async fn error_rate_in_window(
    store: &SessionStore,
    start_ts: i64,
    end_ts: i64,
) -> StoreResult<ErrorRateSnapshot> {
    error_rate_in_window_filtered(store, start_ts, end_ts, &ErrorRateFilter::default()).await
}

/// Aggregate `http.request` audit rows across a filtered time window.
///
/// # Errors
/// Returns [`ArgoError::Provider`] when SQLite rejects the query or a
/// matching audit payload is malformed JSON.
pub async fn error_rate_in_window_filtered(
    store: &SessionStore,
    start_ts: i64,
    end_ts: i64,
    filter: &ErrorRateFilter,
) -> StoreResult<ErrorRateSnapshot> {
    let rows = sqlx::query(
        "SELECT payload_json FROM audit_record \
         WHERE kind = 'http.request' AND created_at >= ? AND created_at <= ?",
    )
    .bind(start_ts)
    .bind(end_ts)
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("audit_record error-rate query: {e}")))?;

    let mut snapshot = ErrorRateSnapshot {
        start_ts,
        end_ts,
        ..Default::default()
    };
    for row in rows {
        let payload_json: String = row
            .try_get("payload_json")
            .map_err(|e| ArgoError::Provider(format!("audit_record payload column: {e}")))?;
        let payload: serde_json::Value = serde_json::from_str(&payload_json)
            .map_err(|e| ArgoError::Provider(format!("audit_record payload json: {e}")))?;
        if !matches_filter(&payload, filter) {
            continue;
        }
        let Some(status) = status_code(&payload) else {
            continue;
        };
        snapshot.total += 1;
        if status >= 500 {
            snapshot.errors += 1;
        }
        *snapshot.per_status_histogram.entry(status).or_insert(0) += 1;
    }
    Ok(snapshot)
}

/// Availability snapshot over `http.request` audit rows in a window.
///
/// Counts every request with a parseable status code; rows whose
/// status is `< 500` are credited as successful for SLO accounting
/// (4xx is a client-side issue, not a service-availability failure).
///
/// # Errors
/// Returns [`ArgoError::Provider`] when SQLite rejects the query or
/// a row payload is malformed JSON.
///
/// ```
/// # async fn demo(store: &argo_store::SessionStore) -> argo_store::StoreResult<()> {
/// let snapshot = argo_store::audit_record_query::availability_in_window(
///     store,
///     0,
///     i64::MAX,
/// ).await?;
/// assert!(snapshot.total >= snapshot.success_count);
/// # Ok(()) }
/// ```
pub async fn availability_in_window(
    store: &SessionStore,
    start_ts: i64,
    end_ts: i64,
) -> StoreResult<AvailabilitySnapshot> {
    availability_in_window_filtered(store, start_ts, end_ts, &ErrorRateFilter::default()).await
}

/// Filtered counterpart to [`availability_in_window`]. Shares the
/// same tenant/route filter struct as the error-rate path so the
/// gateway/CLI surfaces can compose both dashboards from one
/// filter shape.
///
/// # Errors
/// Returns [`ArgoError::Provider`] when SQLite rejects the query or
/// a row payload is malformed JSON.
pub async fn availability_in_window_filtered(
    store: &SessionStore,
    start_ts: i64,
    end_ts: i64,
    filter: &ErrorRateFilter,
) -> StoreResult<AvailabilitySnapshot> {
    let rows = sqlx::query(
        "SELECT payload_json FROM audit_record \
         WHERE kind = 'http.request' AND created_at >= ? AND created_at <= ?",
    )
    .bind(start_ts)
    .bind(end_ts)
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("audit_record availability query: {e}")))?;

    let mut snapshot = AvailabilitySnapshot {
        start_ts,
        end_ts,
        ..Default::default()
    };
    for row in rows {
        let payload_json: String = row
            .try_get("payload_json")
            .map_err(|e| ArgoError::Provider(format!("audit_record payload column: {e}")))?;
        let payload: serde_json::Value = serde_json::from_str(&payload_json)
            .map_err(|e| ArgoError::Provider(format!("audit_record payload json: {e}")))?;
        if !matches_filter(&payload, filter) {
            continue;
        }
        let Some(status) = status_code(&payload) else {
            continue;
        };
        snapshot.total += 1;
        if status < 500 {
            snapshot.success_count += 1;
        }
    }
    Ok(snapshot)
}

/// p50/p95/p99 latency over `http.request` rows in a window.
///
/// SQLite has no `PERCENTILE_CONT`; we materialize the samples,
/// sort them ascending in-memory, and read off the requested
/// percentile slots. Sample volume is bounded by the audit
/// retention policy so the in-memory sort is acceptable.
///
/// # Errors
/// Returns [`ArgoError::Provider`] when SQLite rejects the query or
/// a row payload is malformed JSON.
///
/// ```
/// # async fn demo(store: &argo_store::SessionStore) -> argo_store::StoreResult<()> {
/// let pcts = argo_store::audit_record_query::latency_percentiles_in_window(
///     store,
///     0,
///     i64::MAX,
/// ).await?;
/// assert!(pcts.p50_us <= pcts.p95_us);
/// assert!(pcts.p95_us <= pcts.p99_us);
/// # Ok(()) }
/// ```
pub async fn latency_percentiles_in_window(
    store: &SessionStore,
    start_ts: i64,
    end_ts: i64,
) -> StoreResult<LatencyPercentiles> {
    latency_percentiles_in_window_filtered(store, start_ts, end_ts, &ErrorRateFilter::default())
        .await
}

/// Filtered counterpart to [`latency_percentiles_in_window`].
///
/// # Errors
/// Returns [`ArgoError::Provider`] when SQLite rejects the query or
/// a row payload is malformed JSON.
pub async fn latency_percentiles_in_window_filtered(
    store: &SessionStore,
    start_ts: i64,
    end_ts: i64,
    filter: &ErrorRateFilter,
) -> StoreResult<LatencyPercentiles> {
    let rows = sqlx::query(
        "SELECT payload_json FROM audit_record \
         WHERE kind = 'http.request' AND created_at >= ? AND created_at <= ?",
    )
    .bind(start_ts)
    .bind(end_ts)
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("audit_record latency query: {e}")))?;

    let mut samples_us: Vec<u64> = Vec::with_capacity(rows.len());
    for row in rows {
        let payload_json: String = row
            .try_get("payload_json")
            .map_err(|e| ArgoError::Provider(format!("audit_record payload column: {e}")))?;
        let payload: serde_json::Value = serde_json::from_str(&payload_json)
            .map_err(|e| ArgoError::Provider(format!("audit_record payload json: {e}")))?;
        if !matches_filter(&payload, filter) {
            continue;
        }
        let Some(duration_us) = duration_us(&payload) else {
            continue;
        };
        samples_us.push(duration_us);
    }
    samples_us.sort_unstable();
    let total = samples_us.len() as u64;
    Ok(LatencyPercentiles {
        start_ts,
        end_ts,
        total,
        p50_us: percentile(&samples_us, 50),
        p95_us: percentile(&samples_us, 95),
        p99_us: percentile(&samples_us, 99),
    })
}

fn percentile(sorted: &[u64], pct: u8) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    // Nearest-rank percentile on a sorted ascending slice. For
    // P=99 over 1 sample we want that one sample, so the index
    // is clamped into [0, len-1].
    let rank = (sorted.len() as f64 * (pct as f64 / 100.0)).ceil() as usize;
    let idx = rank.saturating_sub(1).min(sorted.len() - 1);
    sorted[idx]
}

fn duration_us(payload: &serde_json::Value) -> Option<u64> {
    if let Some(us) = payload.get("duration_us").and_then(|v| v.as_u64()) {
        return Some(us);
    }
    payload
        .get("duration_ms")
        .and_then(|v| v.as_u64())
        .map(|ms| ms.saturating_mul(1_000))
}

fn status_code(payload: &serde_json::Value) -> Option<u16> {
    payload
        .get("status")
        .or_else(|| payload.get("status_code"))
        .and_then(|v| v.as_u64())
        .and_then(|v| u16::try_from(v).ok())
}

fn matches_filter(payload: &serde_json::Value, filter: &ErrorRateFilter) -> bool {
    if let Some(expected) = filter.tenant_id.as_deref() {
        if payload.get("tenant_id").and_then(|v| v.as_str()) != Some(expected) {
            return false;
        }
    }
    if let Some(route) = filter.route.as_deref() {
        let Some(path) = payload.get("path").and_then(|v| v.as_str()) else {
            return false;
        };
        if let Some(prefix) = route.strip_suffix('*') {
            return path.starts_with(prefix);
        }
        return path == route;
    }
    true
}

#[cfg(test)]
#[path = "audit_record_query_tests.rs"]
mod tests;
