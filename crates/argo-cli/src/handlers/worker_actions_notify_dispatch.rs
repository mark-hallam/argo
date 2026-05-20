use crate::handlers::worker_actions::ActionRecord;
use argo_channels::fallback::{FallbackChain, FallbackChainManifest};
use argo_channels::{Notification, OperatorTarget, RiskLevel};
use argo_identity::NotifyChannelRef;
use chrono::{Duration as ChronoDuration, Utc};
use serde::Deserialize;
use std::path::Path;

/// `.skycot/notify.toml` — workspace-level approval-notification
/// config. Loaded once per `AutoApprovalDecision::Ask` dispatch.
const NOTIFY_RELPATH: &str = ".skycot/notify.toml";

/// `[notify_channels]` umbrella block in `.skycot/notify.toml`.
#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
pub struct NotifyChannelsConfig {
    /// `[notify_channels.fallback_chain]` — channel order tried in
    /// sequence on each dispatch.
    #[serde(default)]
    pub fallback_chain: Option<FallbackChainManifest>,
}

/// Workspace-level approval-notification config. Operators
/// declare the channels here; absent file == no channels == no-op.
#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
pub struct NotifyConfig {
    /// Channel preferences. Each entry is a [`NotifyChannelRef`].
    #[serde(default)]
    pub channels: Vec<NotifyChannelRef>,
    /// Optional `[notify_channels]` block; today only carries the
    /// V1.4 fallback chain.
    #[serde(default)]
    pub notify_channels: NotifyChannelsConfig,
}

impl NotifyConfig {
    /// Validated [`FallbackChain`] derived from the parsed config,
    /// if the operator declared one.
    #[must_use]
    pub fn fallback_chain(&self) -> Option<FallbackChain> {
        self.notify_channels
            .fallback_chain
            .clone()
            .and_then(FallbackChainManifest::into_chain)
    }

    /// Read `<cwd>/.skycot/notify.toml`. Missing file or invalid
    /// TOML returns the default (empty channel list — no-op).
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

/// Outcome of one channel send.
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

/// Test seam — production code uses `EnvChannelSender`.
#[async_trait::async_trait]
pub trait NotifySender: Send + Sync {
    /// Send `notification` via the channel referenced by `ref_`.
    async fn send_via(
        &self,
        ref_: &NotifyChannelRef,
        notification: &Notification,
    ) -> Result<(), String>;
}

/// `true` when the channel's optional risk filter accepts an
/// action with the given `risk` (case-insensitive).
#[must_use]
pub fn matches_risk(ch: &NotifyChannelRef, risk: &str) -> bool {
    match &ch.risk_level {
        None => true,
        Some(filter) => filter.eq_ignore_ascii_case(risk),
    }
}

/// Build the [`Notification`] envelope for an action awaiting
/// approval.
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
