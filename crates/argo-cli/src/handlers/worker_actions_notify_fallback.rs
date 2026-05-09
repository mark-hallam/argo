//! Approval-channel fallback chain orchestrator (V1.4).
//!
//! Walks `FallbackChain.order` one channel at a time. Each step is
//! `tokio::time::timeout`-wrapped at `chain.per_step_timeout_secs`.
//! The first success terminates the walk; otherwise the chain
//! produces a [`DeliveryReport`] with one [`DeliveryAttempt`] per
//! step. WebInbox should always be the LAST entry — it's a local
//! sqlite/JSONL write, effectively never fails, and gives the
//! "floor" guarantee that no notification is ever fully lost.
//!
//! This module owns the orchestration only. Per-channel sends route
//! through the existing [`NotifySender`] trait used by
//! [`worker_actions_notify::dispatch`], so swapping channel impls
//! does not touch this file.
use crate::handlers::worker_actions_notify::NotifySender;
use argo_channels::fallback::{
    append_report, reports_path, AttemptOutcome, DeliveryAttempt, DeliveryReport, FallbackChain,
};
use argo_channels::Notification;
use argo_identity::NotifyChannelRef;
use chrono::Utc;
use std::path::Path;
use std::time::Duration;

/// Walk `chain.order`, returning a [`DeliveryReport`] of every
/// attempt. The walk terminates at the first
/// [`AttemptOutcome::Success`]. Channels named in the chain that
/// aren't configured in `channel_refs` are recorded as
/// [`AttemptOutcome::Unconfigured`] and the walk continues.
///
/// `channel_refs` is the operator's full `[[channels]]` list from
/// `.skycot/notify.toml`; the chain looks up by `ref_.channel`.
/// The first matching ref wins when multiple targets share a kind.
///
/// `notification` is sent verbatim to each channel — fan-out,
/// per-channel templating, and risk filtering happen in the caller
/// (`worker_actions_notify::dispatch`); the chain only sequences
/// the attempts.
pub async fn dispatch_with_fallback(
    notification: &Notification,
    chain: &FallbackChain,
    channel_refs: &[NotifyChannelRef],
    sender: &dyn NotifySender,
) -> DeliveryReport {
    let started_at = Utc::now();
    let mut attempts: Vec<DeliveryAttempt> = Vec::with_capacity(chain.order.len());
    let mut succeeded_via: Option<String> = None;
    let timeout = Duration::from_secs(chain.per_step_timeout_secs);

    for kind in &chain.order {
        let attempt_started = Utc::now();
        let outcome = run_one_attempt(kind, channel_refs, notification, sender, timeout).await;
        let terminal = outcome.is_terminal();
        attempts.push(DeliveryAttempt {
            channel: kind.clone(),
            started_at: attempt_started,
            outcome,
        });
        if terminal {
            succeeded_via = Some(kind.clone());
            break;
        }
    }

    DeliveryReport {
        action_id: notification.action_id.clone(),
        started_at,
        finished_at: Utc::now(),
        succeeded_via,
        attempts,
    }
}

async fn run_one_attempt(
    kind: &str,
    channel_refs: &[NotifyChannelRef],
    notification: &Notification,
    sender: &dyn NotifySender,
    timeout: Duration,
) -> AttemptOutcome {
    let Some(ref_) = channel_refs.iter().find(|c| c.channel == kind) else {
        tracing::info!(
            channel = kind,
            action_id = %notification.action_id,
            "fallback step skipped: no matching NotifyChannelRef configured",
        );
        return AttemptOutcome::Unconfigured;
    };
    let send_fut = sender.send_via(ref_, notification);
    match tokio::time::timeout(timeout, send_fut).await {
        Ok(Ok(())) => {
            tracing::info!(
                channel = %ref_.channel,
                target = %ref_.target,
                action_id = %notification.action_id,
                "fallback step success",
            );
            AttemptOutcome::Success
        }
        Ok(Err(e)) => {
            tracing::info!(
                channel = %ref_.channel,
                target = %ref_.target,
                action_id = %notification.action_id,
                error = %e,
                "fallback step failed; falling through",
            );
            AttemptOutcome::Failure { error: e }
        }
        Err(_elapsed) => {
            tracing::info!(
                channel = %ref_.channel,
                target = %ref_.target,
                action_id = %notification.action_id,
                timeout_secs = timeout.as_secs(),
                "fallback step timed out; falling through",
            );
            AttemptOutcome::Timeout
        }
    }
}

/// Best-effort persistence of a [`DeliveryReport`] to the JSONL
/// log read by `argo doctor channels`. Errors are logged at
/// `tracing::warn!` and swallowed — the chain's source of truth is
/// the in-memory return value plus the per-attempt `tracing::info!`
/// lines that audit-export consumes.
pub fn persist_report(cwd: &Path, report: &DeliveryReport) {
    let path = reports_path(cwd);
    if let Err(e) = append_report(&path, report) {
        tracing::warn!(
            path = %path.display(),
            error = %e,
            "delivery report persistence failed; in-memory report retained",
        );
    }
}

#[cfg(test)]
#[path = "worker_actions_notify_fallback_tests.rs"]
mod tests;
