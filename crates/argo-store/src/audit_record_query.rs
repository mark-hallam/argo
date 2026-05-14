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
