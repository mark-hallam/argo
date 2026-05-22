//! Stream `mission-wire-outbound-notify-on-ask` — the load-bearing
//! wire from `AutoApprovalDecision::Ask` to outbound `NotifyChannel.send()`.
//!
//! Today the auto-approval gate (`worker_actions_auto_approval.rs`)
//! returns `AutoApprovalDecision::Ask` and the action stays `Pending` in the
//! queue forever — operators get nothing in Slack/email/SMS. This
//! module wires the missing fan-out.
//!
//! ## Where the channel list lives
//!
//! The brief (`docs/build-plan/streams/mission-wire-outbound-notify-on-ask.md`)
//! refers to a `[action.<name>.approval]` manifest section, but
//! in the actual codebase channel preferences live on the worker
//! (`argo_identity::Worker.channels: Vec<NotifyChannelRef>`) and
//! a workspace-level fallback at `.skycot/notify.toml`. v1 reads
//! the workspace file — it's the operator-friendly knob and lands
//! the wire without dragging a Worker lookup into the dispatch
//! path. A follow-up brief swaps in worker-scoped channels once
//! the gate has access to the resolved [`Worker`].
//!
//! ## Fan-out idiom
//!
//! For each `NotifyChannelRef`:
//!   1. Filter on `risk_level` (channel may opt out of risks
//!      lower than the action — e.g. SMS for high only).
//!   2. Hand the ref to the [`NotifySender`] trait, which v1
//!      implements via `EnvChannelSender` for the env-config
//!      channels that are already wired (push, webinbox).
//!   3. Errors surface as `tracing::warn!` and never propagate —
//!      the auto-approval gate's `AutoApprovalDecision` is the source of
//!      truth; notify is best-effort.
//!
//! Tests under `worker_actions_notify_tests.rs` exercise the
//! fan-out with a stub sender that records calls.
use crate::handlers::worker_actions::ActionRecord;
use crate::handlers::worker_actions_notify_fallback;
use argo_audit::AuditSink;
use argo_channels::operator_preferences::{
    OperatorId, OperatorPreferencesResolver, TomlPreferencesSource,
};
use argo_channels::{Notification, QuietHoursResolver};
#[cfg(test)]
use argo_channels::RiskLevel;
use chrono::Utc;
use std::path::Path;
use std::sync::Arc;

#[path = "worker_actions_notify_audit.rs"]
mod notify_audit;
#[path = "worker_actions_notify_chat.rs"]
mod notify_chat;
#[path = "worker_actions_notify_discord.rs"]
mod notify_discord;
#[path = "worker_actions_notify_dispatch.rs"]
mod notify_dispatch;
#[path = "worker_actions_notify_email.rs"]
mod notify_email;
#[path = "worker_actions_notify_env.rs"]
mod notify_env;
#[path = "worker_actions_notify_slack.rs"]
mod notify_slack;
#[path = "worker_actions_notify_sms.rs"]
mod notify_sms;
#[path = "worker_actions_notify_sse.rs"]
mod notify_sse;
#[path = "worker_actions_notify_voice.rs"]
mod notify_voice;

use notify_audit::emit_approval_asked;
pub use notify_dispatch::{build_notification, dispatch, matches_risk, NotifyConfig, NotifySender};
pub use notify_env::EnvChannelSender;
use notify_env::{approval_payload, post_json, required_env, send_with_rate_limit_retry};

/// Best-effort entry point called from the auto-approval gate's
/// `AutoApprovalDecision::Ask` arm. Loads `<cwd>/.skycot/notify.toml`, builds
/// the envelope, then either:
///
/// 1. Routes through [`worker_actions_notify_fallback::dispatch_with_fallback`]
///    when `[notify_channels.fallback_chain]` is configured — first
///    successful channel wins, subsequent channels not tried, and a
///    [`DeliveryReport`] is appended to the JSONL log read by
///    `argo doctor channels`. (V1.4)
/// 2. Falls back to the v0 per-channel fan-out via
///    [`EnvChannelSender`] otherwise — preserves the no-config
///    default behaviour.
///
/// When `audit` is `Some`, every successful per-channel send writes
/// a `AuditKind::ApprovalAsked` row carrying
/// `{ action_id, worker_id, channel, target, risk_level,
///   requested_at, expiry_ts }` (audit finding P0-2). Failed sends
/// do NOT emit so the chain stays free of phantom asks.
///
/// Returns the count of successful sends — callers ignore it; the
/// value is surfaced for tests + future telemetry.
///
/// [`DeliveryReport`]: argo_channels::fallback::DeliveryReport
pub async fn try_dispatch_on_ask(
    action: &ActionRecord,
    cwd: &Path,
    audit: Option<&AuditSink>,
) -> usize {
    let config = NotifyConfig::load(cwd);
    if config.channels.is_empty() {
        return 0;
    }
    let sender = EnvChannelSender;
    if let Some(chain) = config.fallback_chain() {
        let notification = build_notification(action);
        // mission-channels-operator-preferred-fallback: resolve the
        // operator's preferred channel (Slack-first, SMS-never, etc.)
        // and promote it to position 0 of the fallback chain. The
        // resolver lazily reads `<cwd>/argo.toml` and falls through
        // to manifest order when no `[operator.<id>.preferences]`
        // block matches — covers the no-config dev path safely.
        let preferred = resolve_preferred_channel(cwd, &notification).await;
        let chain = worker_actions_notify_fallback::promote_preferred(chain, preferred.as_deref());
        // mission-quiet-hours-channels: workspace `[operator.quiet_hours]`
        // filter — strip noisy channel kinds (SMS, voice, WhatsApp,
        // iMessage, push by default) inside the operator's quiet window
        // so dispatch falls through to a non-noisy step (typically
        // WebInbox). Disabled-by-default; absent config leaves the chain
        // untouched.
        let chain = QuietHoursResolver::from_toml_path(&cwd.join("argo.toml"))
            .filter_chain(chain, chrono::Utc::now());
        let report = worker_actions_notify_fallback::dispatch_with_fallback(
            &notification,
            &chain,
            &config.channels,
            &sender,
        )
        .await;
        if let (Some(sink), Some(channel)) = (audit, report.succeeded_via.as_deref()) {
            // First-success-wins: emit one `ApprovalAsked` row for
            // the channel that delivered. Late attempts are
            // reported in the JSONL log but not the audit chain
            // (they never reached the operator).
            let target = config
                .channels
                .iter()
                .find(|c| c.channel == channel)
                .map(|c| c.target.clone())
                .unwrap_or_default();
            emit_approval_asked(sink, action, &notification, channel, &target).await;
        }
        worker_actions_notify_fallback::persist_report(cwd, &report);
        return usize::from(report.ok());
    }
    let notification = build_notification(action);
    let results = dispatch(&config, action, &sender).await;
    if let Some(sink) = audit {
        for r in results.iter().filter(|r| r.success) {
            emit_approval_asked(sink, action, &notification, &r.channel, &r.target).await;
        }
    }
    results.iter().filter(|r| r.success).count()
}

/// Resolve the operator-preferred channel kind for `notification`
/// against the `[operator.<id>.preferences]` block (if any) in
/// `<cwd>/argo.toml`. Returns the channel kind string suitable for
/// matching against `FallbackChain.order` entries, or `None` when:
///
/// - the operator has no preferences row,
/// - the file is missing/unparseable,
/// - or the resolver errors for any other reason.
///
/// All failure modes return `None` so the dispatcher falls back to
/// the manifest-defined order — operator preferences are an
/// override, not a hard requirement.
async fn resolve_preferred_channel(cwd: &Path, notification: &Notification) -> Option<String> {
    let path = cwd.join("argo.toml");
    let source = Arc::new(TomlPreferencesSource::new(path));
    let resolver = OperatorPreferencesResolver::new(source);
    let op = OperatorId::new(&notification.target.operator_id);
    resolver
        .resolve_initial(&op, notification.risk_level, Utc::now())
        .await
        .ok()
        .map(|channel| channel.0)
}

#[cfg(test)]
#[path = "worker_actions_notify_tests.rs"]
mod tests;
