//! Last-24-hour audit-record aggregation per worker.
//!
//! v1 reads counts of three audit-kind buckets:
//!
//! - `assistant_message` — message-shaped activity (replies sent,
//!   emails sent, calls handled).
//! - `tool_call` — action-shaped activity (tickets resolved,
//!   meetings booked, documents processed).
//! - `escalation.required` minus `approval.granted` +
//!   `approval.consumed` — net "pending" work.
//!
//! The query is a single CTE that joins `audit_record` to
//! `session` (which carries `worker_id`) and aggregates the
//! buckets in one round-trip.

use std::sync::Arc;

use argo_identity::WorkerId;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use chrono::{DateTime, Duration, Utc};
use sqlx::Row;

/// Per-worker activity rollup over the last 24 hours.
///
/// Fields are independent counts — the generator's bullet
/// formatters pick the field that best matches each KPI in the
/// vertical's [`crate::bullets_for_vertical`] table.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ActivityCounts {
    /// Rows where `audit_record.kind = 'assistant_message'`.
    pub assistant_messages: i64,
    /// Rows where `audit_record.kind = 'tool_call'`.
    pub tool_calls: i64,
    /// `escalation.required` rows minus resolved rows
    /// (`approval.granted` + `approval.consumed`). Floored at 0.
    pub pending_escalations: i64,
}

/// 24-hour activity window used by the headline generator. Exposed
/// for tests + sibling team-list endpoint that wants the same
/// window semantics.
pub const LOOKBACK: Duration = Duration::hours(24);

/// Query `audit_record` joined on `session.worker_id`, grouping
/// counts for the buckets the v1 bullet formatters consume.
///
/// `now` is injected so tests can pin a deterministic window;
/// production callers pass `Utc::now()` via
/// [`crate::generate_team_headline`].
pub async fn activity_counts_for_worker(
    store: &Arc<SessionStore>,
    worker_id: WorkerId,
    now: DateTime<Utc>,
) -> ArgoResult<ActivityCounts> {
    let since_micros = (now - LOOKBACK).timestamp_micros();
    let worker_blob = worker_id.0.as_bytes().to_vec();
    let row = sqlx::query(
        "SELECT \
            SUM(CASE WHEN a.kind = 'assistant_message' THEN 1 ELSE 0 END) AS assistant_messages, \
            SUM(CASE WHEN a.kind = 'tool_call' THEN 1 ELSE 0 END) AS tool_calls, \
            SUM(CASE WHEN a.kind = 'escalation.required' THEN 1 ELSE 0 END) AS escalations, \
            SUM(CASE WHEN a.kind IN ('approval.granted', 'approval.consumed') THEN 1 ELSE 0 END) AS approvals \
         FROM audit_record a \
         JOIN session s ON s.id = a.session_id \
         WHERE s.worker_id = ? \
           AND a.created_at >= ?",
    )
    .bind(worker_blob)
    .bind(since_micros)
    .fetch_one(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("team-headline counts: {e}")))?;

    let assistant_messages: i64 = row.try_get("assistant_messages").unwrap_or(0);
    let tool_calls: i64 = row.try_get("tool_calls").unwrap_or(0);
    let escalations: i64 = row.try_get("escalations").unwrap_or(0);
    let approvals: i64 = row.try_get("approvals").unwrap_or(0);
    let pending_escalations = (escalations - approvals).max(0);

    Ok(ActivityCounts {
        assistant_messages,
        tool_calls,
        pending_escalations,
    })
}
