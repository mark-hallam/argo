//! Stream `mission-wire-outbound-notify-on-ask` — the load-bearing
//! wire from `Decision::Ask` to outbound `NotifyChannel.send()`.
//!
//! Today the auto-approval gate (`worker_actions_auto_approval.rs`)
//! returns `Decision::Ask` and the action stays `Pending` in the
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
//!      the auto-approval gate's `Decision` is the source of
//!      truth; notify is best-effort.
//!
//! Tests under `worker_actions_notify_tests.rs` exercise the
//! fan-out with a stub sender that records calls.
use crate::handlers::worker_actions::ActionRecord;
use argo_channels::{Notification, OperatorTarget, RiskLevel};
use argo_identity::NotifyChannelRef;
use chrono::{Duration as ChronoDuration, Utc};
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::Path;

#[path = "worker_actions_notify_chat.rs"]
mod notify_chat;
#[path = "worker_actions_notify_discord.rs"]
mod notify_discord;
#[path = "worker_actions_notify_email.rs"]
mod notify_email;
#[path = "worker_actions_notify_slack.rs"]
mod notify_slack;
#[path = "worker_actions_notify_sms.rs"]
mod notify_sms;
#[path = "worker_actions_notify_sse.rs"]
mod notify_sse;

/// `.skycot/notify.toml` — workspace-level approval-notification
/// config. Loaded once per `Decision::Ask` dispatch.
const NOTIFY_RELPATH: &str = ".skycot/notify.toml";

/// Workspace-level approval-notification config. Operators
/// declare the channels here; absent file == no channels == no-op.
#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
pub struct NotifyConfig {
    /// Channel preferences. Each entry is a [`NotifyChannelRef`]:
    /// `{ channel = "push", target = "operator-iphone",
    ///    risk_level = "high" }`.
    #[serde(default)]
    pub channels: Vec<NotifyChannelRef>,
}

impl NotifyConfig {
    /// Read `<cwd>/.skycot/notify.toml`. Missing file or invalid
    /// TOML returns the default (empty channel list — no-op).
    /// Soft-fails so a single bad file never breaks dispatch
    /// (consistent with `policy_files` in the auto-approval gate).
    #[must_use]
    pub fn load(cwd: &Path) -> Self {
        let path = cwd.join(NOTIFY_RELPATH);
        if !path.exists() {
            return Self::default();
        }
        match std::fs::read_to_string(&path) {
            Ok(body) => match toml::from_str::<Self>(&body) {
                Ok(cfg) => cfg,
                Err(e) => {
                    tracing::warn!(
                        path = %path.display(),
                        error = %e,
                        "notify.toml parse failed; ignoring",
                    );
                    Self::default()
                }
            },
            Err(e) => {
                tracing::warn!(
                    path = %path.display(),
                    error = %e,
                    "notify.toml read failed; ignoring",
                );
                Self::default()
            }
        }
    }
}

/// Outcome of one channel send. `success=false` means the channel
/// returned `Err` and was logged via `tracing::warn!`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispatchResult {
    /// Channel kind from the ref (e.g. `push`, `email`).
    pub channel: String,
    /// Channel-specific target from the ref.
    pub target: String,
    /// `true` when the channel's `send()` returned Ok.
    pub success: bool,
    /// Compact error string when `success=false`.
    pub error: Option<String>,
}

/// Test seam — production code uses [`EnvChannelSender`].
#[async_trait::async_trait]
pub trait NotifySender: Send + Sync {
    /// Send `notification` via the channel referenced by `ref_`.
    /// Errors are surfaced verbatim and logged by [`dispatch`].
    async fn send_via(
        &self,
        ref_: &NotifyChannelRef,
        notification: &Notification,
    ) -> Result<(), String>;
}

/// `true` when the channel's optional risk filter accepts an
/// action with the given `risk` (case-insensitive). `None` filter
/// = accept any risk. The `low` / `medium` / `high` vocabulary
/// matches `argo_channels::RiskLevel`.
#[must_use]
pub fn matches_risk(ch: &NotifyChannelRef, risk: &str) -> bool {
    match &ch.risk_level {
        None => true,
        Some(filter) => filter.eq_ignore_ascii_case(risk),
    }
}

/// Build the [`Notification`] envelope for an action awaiting
/// approval. Pure function — no I/O, no env access except
/// `ARGO_NOTIFY_DEFAULT_EMAIL` (which keeps the OperatorTarget
/// stable across channels that need an email surface).
#[must_use]
pub fn build_notification(action: &ActionRecord) -> Notification {
    let exp = Utc::now() + ChronoDuration::hours(24);
    Notification {
        target: OperatorTarget {
            operator_id: "operator".into(),
            email: std::env::var("ARGO_NOTIFY_DEFAULT_EMAIL")
                .ok()
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "operator@skycot.local".into()),
        },
        action_id: action.action_id.clone(),
        approval_id: action.action_id.clone(),
        risk_level: parse_risk(&action.risk_level),
        prompt_body: format!(
            "Action `{}` (type: {}, risk: {}) needs operator approval.",
            action.action_id, action.action_type, action.risk_level
        ),
        approve_url: format!("argo://approve/{}", action.action_id),
        reject_url: format!("argo://reject/{}", action.action_id),
        expiry: exp,
    }
}

fn parse_risk(s: &str) -> RiskLevel {
    match s.to_ascii_lowercase().as_str() {
        "low" => RiskLevel::Low,
        "medium" => RiskLevel::Medium,
        _ => RiskLevel::High,
    }
}

/// Fan out the notification across the configured channels.
/// Returns one [`DispatchResult`] per channel that passed the
/// risk filter (filtered-out channels do not appear in the
/// output). Errors are logged via `tracing::warn!` and recorded
/// in the result row's `error` field but never propagated — the
/// auto-approval gate's Decision is the source of truth.
pub async fn dispatch(
    config: &NotifyConfig,
    action: &ActionRecord,
    sender: &dyn NotifySender,
) -> Vec<DispatchResult> {
    if config.channels.is_empty() {
        return Vec::new();
    }
    let notification = build_notification(action);
    let mut out = Vec::with_capacity(config.channels.len());
    for ch in &config.channels {
        if !matches_risk(ch, &action.risk_level) {
            continue;
        }
        let result = match sender.send_via(ch, &notification).await {
            Ok(()) => DispatchResult {
                channel: ch.channel.clone(),
                target: ch.target.clone(),
                success: true,
                error: None,
            },
            Err(e) => {
                tracing::warn!(
                    channel = %ch.channel,
                    target = %ch.target,
                    action_id = %action.action_id,
                    error = %e,
                    "notify send failed",
                );
                DispatchResult {
                    channel: ch.channel.clone(),
                    target: ch.target.clone(),
                    success: false,
                    error: Some(e),
                }
            }
        };
        out.push(result);
    }
    out
}

/// Best-effort entry point called from the auto-approval gate's
/// `Decision::Ask` arm. Loads `<cwd>/.skycot/notify.toml`, builds
/// the envelope, fans out via [`EnvChannelSender`]. Returns the
/// count of successful sends — callers ignore it; the value is
/// surfaced for tests + future telemetry.
pub async fn try_dispatch_on_ask(action: &ActionRecord, cwd: &Path) -> usize {
    let config = NotifyConfig::load(cwd);
    if config.channels.is_empty() {
        return 0;
    }
    let sender = EnvChannelSender;
    let results = dispatch(&config, action, &sender).await;
    results.iter().filter(|r| r.success).count()
}

/// Production [`NotifySender`] — dispatches by `ref_.channel` to
/// the matching channel implementation, with config drawn from
/// process env / workspace credentials. Push and webinbox reuse
/// the `argo_channels` implementations; webhook-style channels
/// live in small sibling modules to keep this dispatcher under
/// the 500-LOC ceiling.
pub struct EnvChannelSender;

#[async_trait::async_trait]
impl NotifySender for EnvChannelSender {
    async fn send_via(&self, ref_: &NotifyChannelRef, n: &Notification) -> Result<(), String> {
        match ref_.channel.as_str() {
            "push" => send_push(n).await,
            "webinbox" => send_webinbox(n).await,
            "slack" => notify_slack::send_slack(n).await,
            "email" => notify_email::send_email(ref_, n).await,
            "sms" => notify_sms::send_sms(ref_, n).await,
            "discord" => notify_discord::send_discord(n).await,
            "chat" => notify_chat::send_chat(n).await,
            "sse" => notify_sse::send_sse(n).await,
            other => Err(format!(
                "channel kind '{other}' is not supported by dispatch_notify"
            )),
        }
    }
}

pub(super) fn required_env(channel: &str, name: &str) -> Result<String, String> {
    let value = argo_config::cred::get(name)
        .map_err(|_| format!("{channel} not configured: set {name}"))?;
    if value.trim().is_empty() {
        Err(format!("{channel} not configured: set {name}"))
    } else {
        Ok(value)
    }
}

pub(super) fn approval_payload(channel: &str, n: &Notification) -> Value {
    json!({
        "type": "notify_ask",
        "channel": channel,
        "action_id": n.action_id,
        "approval_id": n.approval_id,
        "risk_level": n.risk_level,
        "summary": n.prompt_body,
        "approve_url": n.approve_url,
        "reject_url": n.reject_url,
        "expires_at": n.expiry,
    })
}

pub(super) async fn post_json(channel: &str, url: &str, payload: Value) -> Result<(), String> {
    let resp = reqwest::Client::new()
        .post(url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("{channel} send failed: {e}"))?;
    let status = resp.status();
    let body = match resp.text().await {
        Ok(body) => body,
        Err(e) => format!("failed to read response body: {e}"),
    };
    if status.is_success() {
        Ok(())
    } else {
        Err(format!("{channel} returned {status}: {body}"))
    }
}

async fn send_push(n: &Notification) -> Result<(), String> {
    use argo_channels::push::{PushApprovalChannel, PushApprovalChannelConfig};
    use argo_channels::NotifyChannel;
    let cfg = PushApprovalChannelConfig::from_env();
    let channel = PushApprovalChannel::new(cfg);
    channel
        .send(n.clone())
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

async fn send_webinbox(n: &Notification) -> Result<(), String> {
    use argo_channels::webinbox::{WebInboxChannel, WebInboxChannelConfig};
    use argo_channels::NotifyChannel;
    let inbox_path = std::env::var("ARGO_WEBINBOX_PATH")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| ".skycot/webinbox.jsonl".to_string());
    let channel = WebInboxChannel::new(WebInboxChannelConfig::local(inbox_path));
    channel
        .send(n.clone())
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[cfg(test)]
#[path = "worker_actions_notify_tests.rs"]
mod tests;
