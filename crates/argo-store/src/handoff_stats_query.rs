//! Aggregate query helpers over the `handoff` table for the
//! `argo handoff-stats` CLI surface (Stream `codex-handoff-stats-cli`).
//!
//! All helpers take a tenant and an inclusive `[since, until)` window
//! in micros UTC. Rows are joined to `session` so the tenant filter is
//! enforced; `provider = 'orchestrator'` matches the persistence
//! tag used by [`crate::SessionStore`] consumers.
//!
//! The helpers fan out around a single `fetch_window` reader so unit
//! tests can seed a small in-memory DB and exercise each axis
//! independently. Expected per-window cardinality (default `7d`) is
//! in the thousands, well within a load-everything-then-fold budget.

use crate::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use serde::Serialize;
use sqlx::Row;

/// Logical outcome bucket for a handoff row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    /// Terminal, receiver met `expected_outcome`.
    Resolved,
    /// Acknowledged by the receiver but the same session held a
    /// later handoff — the chain advanced past this row.
    Escalated,
    /// Terminal without resolution (`expired` or `cancelled`).
    Abandoned,
    /// Still in flight (`pending` or `acknowledged` with no later
    /// handoff in its session).
    Pending,
}

/// One unaggregated handoff row inside the query window.
#[derive(Debug, Clone)]
pub struct HandoffWindowRow {
    /// Handoff id (text).
    pub handoff_id: String,
    /// Session id (text).
    pub session_id: String,
    /// `from_worker_id` text column.
    pub from_worker_id: String,
    /// `to` column — `EscalationStep` JSON. Use [`destination_label`]
    /// to project to a stable group key.
    pub to_json: String,
    /// `expected_outcome` free-text — used as the "skill" axis since
    /// handoff rows do not carry a typed skill identifier.
    pub expected_outcome: String,
    /// Raw status string: `pending` / `acknowledged` / `settled` /
    /// `expired` / `cancelled`.
    pub status: String,
    /// Insertion time (micros UTC).
    pub created_at: i64,
    /// Terminal-resolution time (micros UTC), if any.
    pub resolved_at: Option<i64>,
}

/// Overview block printed at the top of `argo handoff-stats`.
#[derive(Debug, Clone, Serialize)]
pub struct OverviewStats {
    /// Total handoff rows in the window.
    pub total: u64,
    /// Count by status.
    pub resolved: u64,
    /// Escalated (chain advanced past this row).
    pub escalated: u64,
    /// Abandoned (`expired` or `cancelled`).
    pub abandoned: u64,
    /// Still in flight.
    pub pending: u64,
    /// `resolved / (resolved + abandoned)`. `None` if no terminal
    /// rows in the window.
    pub success_rate: Option<f64>,
    /// `sessions_with_multiple_handoffs / sessions_with_any_handoff`.
    /// `None` if no sessions appear in the window.
    pub escalation_rate: Option<f64>,
    /// Median wall time from `created_at` to `resolved_at` over
    /// resolved rows. `None` if no resolved rows have a timestamp.
    pub median_latency_micros: Option<i64>,
}

/// One row in a top-N projection.
#[derive(Debug, Clone, Serialize)]
pub struct TopRow {
    /// Group key (worker id, destination label, or `expected_outcome`).
    pub key: String,
    /// Row count for this key in the window.
    pub count: u64,
}

/// One bucket in the histogram readout.
#[derive(Debug, Clone, Serialize)]
pub struct VolumeBucket {
    /// Bucket start, micros UTC.
    pub bucket_start_micros: i64,
    /// Number of handoffs whose `created_at` falls in this bucket.
    pub count: u64,
}

/// Axis for the per-key breakdown projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BreakdownAxis {
    /// Group by `from_worker_id`.
    Worker,
    /// Group by `expected_outcome` (the closest skill proxy).
    Skill,
    /// Group by destination label (worker id or `human:<addr>`).
    Destination,
}

/// One row in a per-key breakdown projection.
#[derive(Debug, Clone, Serialize)]
pub struct BreakdownRow {
    /// Group key.
    pub key: String,
    /// Row count.
    pub count: u64,
    /// Resolved count.
    pub resolved: u64,
    /// Escalated count.
    pub escalated: u64,
    /// Abandoned count.
    pub abandoned: u64,
    /// Pending count.
    pub pending: u64,
    /// `resolved / (resolved + abandoned)` for this key.
    pub success_rate: Option<f64>,
}

/// Project the orchestrator's `EscalationStep` JSON to a stable
/// group key for top-N / breakdown projections.
///
/// ```
/// use argo_store::handoff_stats_query::destination_label;
/// assert_eq!(destination_label(r#"{"kind":"worker","worker_id":"w-1"}"#), "w-1");
/// assert_eq!(destination_label(r#"{"kind":"human","addr":"ops@x"}"#), "human:ops@x");
/// assert_eq!(destination_label("not-json"), "(unknown)");
/// ```
#[must_use]
pub fn destination_label(to_json: &str) -> String {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(to_json) else {
        return "(unknown)".into();
    };
    match v.get("kind").and_then(serde_json::Value::as_str) {
        Some("worker") => v
            .get("worker_id")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("(unknown)")
            .to_string(),
        Some("human") => format!(
            "human:{}",
            v.get("addr")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("(unknown)")
        ),
        _ => "(unknown)".into(),
    }
}

/// Read every handoff row inside `[since_micros, until_micros)` for
/// `tenant`. Caller may then fold into any of the projections below.
pub async fn fetch_window(
    store: &SessionStore,
    tenant: TenantId,
    since_micros: i64,
    until_micros: i64,
) -> ArgoResult<Vec<HandoffWindowRow>> {
    let rows = sqlx::query(
        "SELECT h.id, h.session_id, h.from_worker_id, h.\"to\", h.expected_outcome, \
         h.status, h.created_at, h.resolved_at \
         FROM handoff h JOIN session s ON s.id = h.session_id \
         WHERE s.tenant_id = ? AND s.provider = 'orchestrator' \
         AND h.created_at >= ? AND h.created_at < ? \
         ORDER BY h.created_at ASC",
    )
    .bind(tenant.0.as_bytes().as_slice())
    .bind(since_micros)
    .bind(until_micros)
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("handoff_stats fetch_window: {e}")))?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let session_blob: Vec<u8> = row.get("session_id");
        let session_id = uuid::Uuid::from_slice(&session_blob)
            .map_err(|e| ArgoError::Provider(format!("session_id decode: {e}")))?
            .to_string();
        out.push(HandoffWindowRow {
            handoff_id: row.get("id"),
            session_id,
            from_worker_id: row.get("from_worker_id"),
            to_json: row.get("to"),
            expected_outcome: row.get("expected_outcome"),
            status: row.get("status"),
            created_at: row.get("created_at"),
            resolved_at: row.try_get::<Option<i64>, _>("resolved_at").unwrap_or(None),
        });
    }
    Ok(out)
}

/// Classify a window row's outcome. `later_in_session` is `true` when
/// the same session_id has a later `created_at` row in the window;
/// such rows are reported as [`Outcome::Escalated`].
#[must_use]
pub fn classify(row: &HandoffWindowRow, later_in_session: bool) -> Outcome {
    match row.status.as_str() {
        "settled" => Outcome::Resolved,
        "expired" | "cancelled" => Outcome::Abandoned,
        _ if later_in_session => Outcome::Escalated,
        _ => Outcome::Pending,
    }
}

/// Build the overview block from a window slice.
#[must_use]
pub fn overview(rows: &[HandoffWindowRow]) -> OverviewStats {
    let later_set = sessions_with_later_rows(rows);
    let mut resolved = 0u64;
    let mut escalated = 0u64;
    let mut abandoned = 0u64;
    let mut pending = 0u64;
    let mut latencies: Vec<i64> = Vec::new();
    let mut sessions_seen = std::collections::HashSet::new();
    let mut sessions_multi: std::collections::HashMap<&str, u32> = std::collections::HashMap::new();
    for row in rows {
        sessions_seen.insert(row.session_id.as_str());
        *sessions_multi.entry(row.session_id.as_str()).or_default() += 1;
        let later = later_set.contains(&(row.session_id.as_str(), row.created_at));
        match classify(row, later) {
            Outcome::Resolved => resolved += 1,
            Outcome::Escalated => escalated += 1,
            Outcome::Abandoned => abandoned += 1,
            Outcome::Pending => pending += 1,
        }
        if row.status == "settled" {
            if let Some(end) = row.resolved_at {
                if end >= row.created_at {
                    latencies.push(end - row.created_at);
                }
            }
        }
    }
    let total = rows.len() as u64;
    let terminal = resolved + abandoned;
    let success_rate = (terminal > 0).then(|| resolved as f64 / terminal as f64);
    let multi_sessions = sessions_multi.values().filter(|c| **c > 1).count() as u64;
    let session_count = sessions_seen.len() as u64;
    let escalation_rate = (session_count > 0).then(|| multi_sessions as f64 / session_count as f64);
    OverviewStats {
        total,
        resolved,
        escalated,
        abandoned,
        pending,
        success_rate,
        escalation_rate,
        median_latency_micros: median(&mut latencies),
    }
}

/// Top-N handoff originators by raw count, desc. Ties broken by key
/// for reproducibility.
#[must_use]
pub fn top_n_from(rows: &[HandoffWindowRow], n: usize) -> Vec<TopRow> {
    top_by(rows, n, |r| r.from_worker_id.clone())
}

/// Top-N handoff destinations by raw count, desc. See
/// [`destination_label`] for the projection rule.
#[must_use]
pub fn top_n_to(rows: &[HandoffWindowRow], n: usize) -> Vec<TopRow> {
    top_by(rows, n, |r| destination_label(&r.to_json))
}

/// Histogram of handoff volume: count per fixed-width
/// `bucket_secs` bucket inside `[since_micros, until_micros)`.
/// `bucket_secs` is clamped to `>= 1`.
#[must_use]
pub fn volume_per_bucket(
    rows: &[HandoffWindowRow],
    since_micros: i64,
    until_micros: i64,
    bucket_secs: i64,
) -> Vec<VolumeBucket> {
    let bucket_us = bucket_secs.max(1).saturating_mul(1_000_000);
    if until_micros <= since_micros {
        return Vec::new();
    }
    let span = until_micros.saturating_sub(since_micros);
    let count = ((span + bucket_us - 1) / bucket_us).max(1) as usize;
    let mut buckets: Vec<VolumeBucket> = (0..count)
        .map(|i| VolumeBucket {
            bucket_start_micros: since_micros + i as i64 * bucket_us,
            count: 0,
        })
        .collect();
    for row in rows {
        if row.created_at < since_micros || row.created_at >= until_micros {
            continue;
        }
        let idx = ((row.created_at - since_micros) / bucket_us) as usize;
        if let Some(b) = buckets.get_mut(idx) {
            b.count += 1;
        }
    }
    buckets
}

/// Per-key breakdown along [`BreakdownAxis`]. Returns rows sorted by
/// count desc, key asc.
#[must_use]
pub fn breakdown_by(rows: &[HandoffWindowRow], axis: BreakdownAxis) -> Vec<BreakdownRow> {
    let later_set = sessions_with_later_rows(rows);
    let mut acc: std::collections::HashMap<String, BreakdownRow> = std::collections::HashMap::new();
    for row in rows {
        let key = match axis {
            BreakdownAxis::Worker => row.from_worker_id.clone(),
            BreakdownAxis::Skill => row.expected_outcome.clone(),
            BreakdownAxis::Destination => destination_label(&row.to_json),
        };
        let entry = acc.entry(key.clone()).or_insert_with(|| BreakdownRow {
            key,
            count: 0,
            resolved: 0,
            escalated: 0,
            abandoned: 0,
            pending: 0,
            success_rate: None,
        });
        entry.count += 1;
        let later = later_set.contains(&(row.session_id.as_str(), row.created_at));
        match classify(row, later) {
            Outcome::Resolved => entry.resolved += 1,
            Outcome::Escalated => entry.escalated += 1,
            Outcome::Abandoned => entry.abandoned += 1,
            Outcome::Pending => entry.pending += 1,
        }
    }
    let mut out: Vec<BreakdownRow> = acc.into_values().collect();
    for r in &mut out {
        let terminal = r.resolved + r.abandoned;
        r.success_rate = (terminal > 0).then(|| r.resolved as f64 / terminal as f64);
    }
    out.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.key.cmp(&b.key)));
    out
}

fn top_by<F: Fn(&HandoffWindowRow) -> String>(
    rows: &[HandoffWindowRow],
    n: usize,
    key_of: F,
) -> Vec<TopRow> {
    let mut counts: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
    for row in rows {
        *counts.entry(key_of(row)).or_default() += 1;
    }
    let mut out: Vec<TopRow> = counts
        .into_iter()
        .map(|(key, count)| TopRow { key, count })
        .collect();
    out.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.key.cmp(&b.key)));
    out.truncate(n);
    out
}

fn sessions_with_later_rows(
    rows: &[HandoffWindowRow],
) -> std::collections::HashSet<(&str, i64)> {
    let mut latest: std::collections::HashMap<&str, i64> = std::collections::HashMap::new();
    for row in rows {
        latest
            .entry(row.session_id.as_str())
            .and_modify(|t| {
                if row.created_at > *t {
                    *t = row.created_at;
                }
            })
            .or_insert(row.created_at);
    }
    rows.iter()
        .filter_map(|r| {
            let last = latest.get(r.session_id.as_str()).copied()?;
            (r.created_at < last).then_some((r.session_id.as_str(), r.created_at))
        })
        .collect()
}

fn median(values: &mut [i64]) -> Option<i64> {
    if values.is_empty() {
        return None;
    }
    values.sort_unstable();
    let mid = values.len() / 2;
    Some(if values.len().is_multiple_of(2) {
        (values[mid - 1] + values[mid]) / 2
    } else {
        values[mid]
    })
}

// Tests live in `handoff_stats_query_tests.rs` (split out under the
// 500-LOC per-file ceiling).
