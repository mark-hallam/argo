//! Periodic approval no-reply reminder.
//!
//! The approval store records pending approval requests and the
//! expiry sweeper applies their timeout decision after `deadline_at`.
//! This task runs before that deadline and sends one reminder through
//! the same workspace-level NotifyChannel config used by the original
//! `Decision::Ask` notification path.
use crate::handlers::worker_actions_notify::{
    matches_risk, EnvChannelSender, NotifyConfig, NotifySender,
};
use argo_channels::{Notification, OperatorTarget, RiskLevel};
use argo_store::{approvals, approvals::ApprovalRequest, SessionStore};
use argo_types::ArgoResult;
use chrono::{DateTime, TimeZone, Utc};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;

pub const DEFAULT_INTERVAL_SECS: u64 = 300;
pub const MIN_INTERVAL_SECS: u64 = 5;
pub const DEFAULT_WINDOW_MINUTES: u64 = 30;
pub const MIN_WINDOW_MINUTES: u64 = 1;

pub(super) fn spawn_approval_reminder_loop(
    store: Arc<SessionStore>,
    cwd: PathBuf,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(interval_secs_from_env()));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            if let Err(e) = sweep_once(&store, &cwd, Utc::now()).await {
                tracing::warn!("approval reminder loop error: {e}");
            }
        }
    })
}

pub(crate) fn interval_secs_from_env() -> u64 {
    env_u64_with_floor(
        "ARGO_APPROVAL_REMINDER_INTERVAL_SECS",
        DEFAULT_INTERVAL_SECS,
        MIN_INTERVAL_SECS,
    )
}

pub(crate) fn window_from_env() -> Duration {
    Duration::from_secs(
        env_u64_with_floor(
            "ARGO_APPROVAL_REMINDER_WINDOW_MINUTES",
            DEFAULT_WINDOW_MINUTES,
            MIN_WINDOW_MINUTES,
        )
        .saturating_mul(60),
    )
}

pub(crate) async fn sweep_once(
    store: &Arc<SessionStore>,
    cwd: &Path,
    now: DateTime<Utc>,
) -> ArgoResult<usize> {
    let config = NotifyConfig::load(cwd);
    let sender = EnvChannelSender;
    sweep_once_with_config(store, &config, now, window_from_env(), &sender).await
}

pub(crate) async fn sweep_once_with_config(
    store: &Arc<SessionStore>,
    config: &NotifyConfig,
    now: DateTime<Utc>,
    window: Duration,
    sender: &dyn NotifySender,
) -> ArgoResult<usize> {
    let rows = approvals::list_pending_near_deadline(store.pool(), window, now).await?;
    let mut stamped = 0;
    for row in rows {
        let notification = build_reminder_notification(&row);
        if dispatch_reminder(config, &notification, sender).await {
            approvals::mark_escalation_sent(store.pool(), &row.id).await?;
            stamped += 1;
        }
    }
    Ok(stamped)
}

fn env_u64_with_floor(name: &str, default: u64, floor: u64) -> u64 {
    let Ok(raw) = std::env::var(name) else {
        return default;
    };
    match raw.parse::<u64>() {
        Ok(n) if n >= floor => n,
        Ok(n) if n > 0 => floor,
        _ => {
            tracing::warn!("ignoring invalid {name}={raw}; using {default}");
            default
        }
    }
}

async fn dispatch_reminder(
    config: &NotifyConfig,
    notification: &Notification,
    sender: &dyn NotifySender,
) -> bool {
    let mut attempted = false;
    let mut all_ok = true;
    for ch in &config.channels {
        if !matches_risk(ch, "high") {
            continue;
        }
        attempted = true;
        if let Err(e) = sender.send_via(ch, notification).await {
            all_ok = false;
            tracing::warn!(
                channel = %ch.channel,
                target = %ch.target,
                approval_id = %notification.approval_id,
                error = %e,
                "approval reminder send failed"
            );
        }
    }
    attempted && all_ok
}

fn build_reminder_notification(row: &ApprovalRequest) -> Notification {
    let expiry = Utc
        .timestamp_micros(row.deadline_at)
        .single()
        .unwrap_or_else(Utc::now);
    let action_id = action_label(&row.action_id);
    Notification {
        target: OperatorTarget {
            operator_id: row.requested_by.clone(),
            email: std::env::var("ARGO_NOTIFY_DEFAULT_EMAIL")
                .ok()
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "operator@skycot.local".into()),
        },
        action_id,
        approval_id: row.id.clone(),
        risk_level: RiskLevel::High,
        prompt_body: format!(
            "Approval reminder: request `{}` is still awaiting your decision before {}.",
            row.id, expiry
        ),
        approve_url: format!("argo://approve/{}", row.id),
        reject_url: format!("argo://reject/{}", row.id),
        expiry,
    }
}

fn action_label(bytes: &[u8]) -> String {
    uuid::Uuid::from_slice(bytes)
        .map(|u| u.to_string())
        .unwrap_or_else(|_| hex::encode(bytes))
}

#[cfg(test)]
#[path = "approval_reminder_tests.rs"]
mod tests;
