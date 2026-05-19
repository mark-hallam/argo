//! Trust-streak rollup for the approvals-inbox "ratchet" card.
//!
//! Stream `mission-pm-trust-ratchet-ui` (P2 #11). Walks the
//! `trust_streak_event` view (filtered slice of `audit_record`) for
//! one worker and counts consecutive `approval.granted` decisions per
//! `(worker_id, tool, target_glob)` triple.
//!
//! Streak compute rules:
//! - Each `approval.granted` row increments the streak for its
//!   `(action_type, target_glob)` pair on that worker.
//! - An `approval.revoked` row resets that pair's streak to zero
//!   (operator changed their mind about a previously-granted grant).
//! - An `approval.replied` row whose payload reports a `"reject"`
//!   `reply_kind` resets that pair's streak to zero (channel-side
//!   denial — first reply wins per the channel-dispatch contract).
//! - `approval.consumed` rows are walked but do not change the
//!   count; they exist on the view so future briefs can correlate
//!   one-shot consumption with the streak.
//!
//! Sensible deviations from the spec:
//! - The brief calls for a `worker_action.failed` audit kind to
//!   reset the streak; that kind is not in `AuditKind` and the
//!   `audit_record` chain does not record per-action completion.
//!   `approval.revoked` is the closest in-tree signal and stands in
//!   for it. A follow-up brief can swap in `worker_action.failed`
//!   when that kind lands.
//! - The brief mentions a denial kind `approval.denied`; the
//!   in-tree signal is `approval.replied` with
//!   `reply_kind = "reject"`, so the aggregator reads that shape.
//! - `target_glob` is read from `payload_json` when present (the
//!   surface the in-flight `mission-pm-approval-target-scope` brief
//!   adds). When absent, the streak collapses to `(worker_id,
//!   tool)`. Once that brief lands, existing rows keep their
//!   tool-only key and new rows ratchet per-target.

use crate::{SessionStore, StoreResult};
use argo_types::ArgoError;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::collections::BTreeMap;

/// One row of the streak rollup. Returned by
/// [`streaks_for_worker`]; the read side wraps these in the
/// `trust-streaks` JSON envelope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrustStreak {
    /// Worker the streak belongs to.
    pub worker_id: String,
    /// Action / tool name (from `audit_record.action_type` or the
    /// payload's `tool` field).
    pub tool: String,
    /// Optional per-target scope. `None` when the underlying
    /// approvals have no `target_glob` in their payload.
    pub target_glob: Option<String>,
    /// Consecutive `approval.granted` rows for this triple since
    /// the last reset.
    pub streak_count: u32,
    /// Wall-clock micros of the most recent `approval.granted` row
    /// in the streak.
    pub last_decided_at_micros: i64,
}

/// Compute every `(tool, target_glob)` streak for `worker_id` and
/// return those whose count meets `min_threshold`. Streaks below
/// the threshold are filtered out so the route can serve the same
/// shape as the inbox renders.
///
/// # Errors
/// Returns [`ArgoError::Provider`] when SQLite rejects the query.
///
/// ```
/// # async fn demo(store: &argo_store::SessionStore) -> argo_store::StoreResult<()> {
/// let streaks = argo_store::trust_streak::streaks_for_worker(
///     store,
///     "00000000-0000-0000-0000-000000000000",
///     10,
/// ).await?;
/// assert!(streaks.iter().all(|s| s.streak_count >= 10));
/// # Ok(()) }
/// ```
pub async fn streaks_for_worker(
    store: &SessionStore,
    worker_id: &str,
    min_threshold: u32,
) -> StoreResult<Vec<TrustStreak>> {
    let rows = sqlx::query(
        "SELECT kind, action_type, payload_json, created_at \
         FROM trust_streak_event \
         WHERE worker_id = ? \
         ORDER BY created_at ASC, seq ASC",
    )
    .bind(worker_id)
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("trust_streak_event query: {e}")))?;

    let mut by_pair: BTreeMap<(String, Option<String>), StreakState> = BTreeMap::new();
    for row in rows {
        let kind: String = row
            .try_get("kind")
            .map_err(|e| ArgoError::Provider(format!("trust_streak_event kind: {e}")))?;
        let column_tool: Option<String> = row.try_get("action_type").ok();
        let payload_json: String = row
            .try_get("payload_json")
            .map_err(|e| ArgoError::Provider(format!("trust_streak_event payload: {e}")))?;
        let created_at: i64 = row
            .try_get("created_at")
            .map_err(|e| ArgoError::Provider(format!("trust_streak_event created_at: {e}")))?;
        let payload: serde_json::Value =
            serde_json::from_str(&payload_json).unwrap_or(serde_json::Value::Null);
        let Some((tool, target_glob)) = extract_triple(&column_tool, &payload) else {
            continue;
        };
        apply_row(
            &mut by_pair,
            (tool, target_glob),
            &kind,
            &payload,
            created_at,
        );
    }
    let mut streaks: Vec<TrustStreak> = by_pair
        .into_iter()
        .filter(|(_, s)| s.count >= min_threshold)
        .map(|((tool, target_glob), s)| TrustStreak {
            worker_id: worker_id.to_string(),
            tool,
            target_glob,
            streak_count: s.count,
            last_decided_at_micros: s.last_decided_at_micros,
        })
        .collect();
    // Stable order: longest streak first, ties by tool name.
    streaks.sort_by(|a, b| {
        b.streak_count
            .cmp(&a.streak_count)
            .then_with(|| a.tool.cmp(&b.tool))
    });
    Ok(streaks)
}

#[derive(Debug, Default, Clone, Copy)]
struct StreakState {
    count: u32,
    last_decided_at_micros: i64,
}

fn apply_row(
    by_pair: &mut BTreeMap<(String, Option<String>), StreakState>,
    pair: (String, Option<String>),
    kind: &str,
    payload: &serde_json::Value,
    created_at: i64,
) {
    let entry = by_pair.entry(pair).or_default();
    match kind {
        "approval.granted" => {
            entry.count = entry.count.saturating_add(1);
            entry.last_decided_at_micros = created_at;
        }
        "approval.revoked" => {
            *entry = StreakState::default();
        }
        "approval.replied" => {
            let reply_kind = payload
                .get("reply_kind")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if reply_kind == "reject" || reply_kind == "deny" {
                *entry = StreakState::default();
            }
        }
        _ => {}
    }
}

fn extract_triple(
    column_tool: &Option<String>,
    payload: &serde_json::Value,
) -> Option<(String, Option<String>)> {
    let tool = column_tool
        .clone()
        .or_else(|| {
            payload
                .get("tool")
                .and_then(|v| v.as_str())
                .map(String::from)
        })
        .or_else(|| {
            payload
                .get("action_type")
                .and_then(|v| v.as_str())
                .map(String::from)
        })?;
    let target_glob = payload
        .get("target_glob")
        .and_then(|v| v.as_str())
        .map(String::from);
    Some((tool, target_glob))
}

#[cfg(test)]
#[path = "trust_streak_tests.rs"]
mod tests;
