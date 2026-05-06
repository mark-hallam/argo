//! Anomaly-detection hook (Stream `claude-anomaly-detector`).
//!
//! Sibling of `worker_actions_dispatch.rs`. Lifted out so that
//! file stays under the per-file LOC budget.
//!
//! Called from `dispatch::complete()` AFTER the transition row
//! lands. Failures are swallowed by the caller — anomaly
//! detection is observability, not correctness, and should never
//! fail an action.
use super::worker_actions::ActionRecord;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use chrono::{DateTime, Utc};
use std::sync::Arc;

/// Fold this completed action into the per-(worker, kind)
/// Welford baseline + emit anomaly events when |z| crosses the
/// configured threshold.
pub(crate) async fn check_action_anomaly(
    store: &Arc<SessionStore>,
    rec: &ActionRecord,
) -> ArgoResult<()> {
    let latency_ms = match (rec.approved_at.as_deref(), rec.executed_at.as_deref()) {
        (Some(start), Some(end)) => duration_ms(start, end).unwrap_or(0),
        // Pending → executing without an approval gate (autonomous-safe).
        (None, Some(end)) => duration_ms(&rec.created_at, end).unwrap_or(0),
        _ => 0,
    };
    let retry_count = rec
        .evidence
        .iter()
        .filter(|e| e.starts_with("argo://action/"))
        .count() as u32;
    let sample = argo_store::anomaly::BehaviorSample {
        latency_ms,
        retry_count,
    };
    let threshold = argo_store::anomaly::configured_threshold();
    let _emitted = argo_store::anomaly::record_sample(
        store.pool(),
        &rec.worker_id.to_string(),
        &rec.action_id,
        &rec.action_type,
        sample,
        threshold,
        Utc::now(),
    )
    .await
    .map_err(|e| ArgoError::Provider(format!("anomaly record_sample: {e}")))?;
    Ok(())
}

fn duration_ms(start: &str, end: &str) -> Option<i64> {
    let s = start.parse::<DateTime<Utc>>().ok()?;
    let e = end.parse::<DateTime<Utc>>().ok()?;
    Some(e.signed_duration_since(s).num_milliseconds().max(0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duration_ms_returns_positive_delta() {
        let d = duration_ms("2026-05-07T10:00:00Z", "2026-05-07T10:00:01Z").unwrap();
        assert_eq!(d, 1_000);
    }

    #[test]
    fn duration_ms_clamps_negatives_to_zero() {
        let d = duration_ms("2026-05-07T10:00:01Z", "2026-05-07T10:00:00Z").unwrap();
        assert_eq!(d, 0);
    }

    #[test]
    fn duration_ms_returns_none_on_garbage() {
        assert!(duration_ms("not-a-date", "2026-05-07T10:00:00Z").is_none());
    }
}
