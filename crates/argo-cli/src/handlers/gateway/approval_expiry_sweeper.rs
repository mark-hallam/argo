//! Periodic approval expiry sweeper.
//!
//! The store owns the state transition; this task only calls it on a
//! fixed cadence and writes one audit row when a sweep expires rows.
use argo_audit::{AuditKind, AuditSink};
use argo_store::{approvals, SessionStore};
use argo_types::ArgoResult;
use chrono::{DateTime, Utc};
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;

pub const DEFAULT_TICK_SECS: u64 = 30;
pub const MIN_TICK_SECS: u64 = 5;
pub const DECIDED_BY: &str = "system:approval-expiry-sweeper";

pub fn spawn_approval_expiry_sweeper(store: Arc<SessionStore>) -> Option<JoinHandle<()>> {
    let interval = Duration::from_secs(tick_secs_from_env());
    Some(tokio::spawn(async move {
        let mut ticker = tokio::time::interval(interval);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            if let Err(e) = sweep_once(&store, Utc::now()).await {
                tracing::warn!("approval expiry sweeper error: {e}");
            }
        }
    }))
}

pub(crate) fn tick_secs_from_env() -> u64 {
    let Ok(raw) = std::env::var("ARGO_APPROVAL_EXPIRY_TICK_SECS") else {
        return DEFAULT_TICK_SECS;
    };
    match raw.parse::<i64>() {
        Ok(n) if n >= MIN_TICK_SECS as i64 => n as u64,
        Ok(n) if n > 0 => MIN_TICK_SECS,
        _ => {
            tracing::warn!(
                "ignoring invalid ARGO_APPROVAL_EXPIRY_TICK_SECS={raw}; using {DEFAULT_TICK_SECS}s"
            );
            DEFAULT_TICK_SECS
        }
    }
}

pub(crate) async fn sweep_once(store: &Arc<SessionStore>, now: DateTime<Utc>) -> ArgoResult<u64> {
    let count = approvals::expire_overdue(store.pool(), now.timestamp_micros(), DECIDED_BY).await?;
    if count > 0 {
        let payload = json!({
            "kind": "approval.expired_by_timeout",
            "count": count,
            "swept_at_iso": now.to_rfc3339(),
        });
        AuditSink::new(Arc::clone(store))
            .write(None, AuditKind::ApprovalExpiredByTimeout, payload, None)
            .await?;
    }
    Ok(count)
}

#[cfg(test)]
#[path = "approval_expiry_sweeper_tests.rs"]
mod tests;
