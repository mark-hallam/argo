//! Channel manifest parsing.

use crate::email::QuietHours;
use crate::fallback::FallbackChainManifest;
use crate::rate_limit::RateLimitConfig;
use crate::{ChannelError, ChannelResult};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// `manifests/channels/email.toml` shape.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct EmailChannelManifest {
    /// Channel provider configuration.
    pub channel: EmailProviderManifest,
    /// HMAC token configuration.
    pub hmac: HmacManifest,
    /// Operator preference defaults.
    pub operator: OperatorManifest,
    /// Per-action approval channel declarations.
    pub action: BTreeMap<String, ActionManifest>,
    /// Operator override for the per-channel rate-limit budget.
    /// Missing fields fall back to the channel's compiled-in
    /// default (email is 5/sec, 30/min). See
    /// [`crate::rate_limit`] for the full default table.
    pub rate_limit: RateLimitConfig,
}

impl EmailChannelManifest {
    /// Parse an email channel manifest from TOML.
    pub fn parse(src: &str) -> ChannelResult<Self> {
        toml::from_str(src).map_err(|e| ChannelError::Parse(format!("email channel manifest: {e}")))
    }

    /// Return approval channels declared for an action id.
    #[must_use]
    pub fn approval_channels_for(&self, action_id: &str) -> Vec<String> {
        self.action
            .get(action_id)
            .and_then(|a| a.approval.as_ref())
            .map(|a| a.channels.clone())
            .unwrap_or_default()
    }
}

/// `[channel]` provider settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct EmailProviderManifest {
    /// Channel id.
    pub id: String,
    /// Provider name.
    pub provider: String,
    /// Environment variable holding the Postmark server token.
    pub postmark_token_env: String,
    /// Environment variable holding the from address.
    pub from_address_env: String,
    /// Subject template.
    pub subject_template: String,
    /// Inbound webhook path.
    pub webhook_path: String,
    /// Postmark API endpoint.
    pub endpoint: String,
}

impl Default for EmailProviderManifest {
    fn default() -> Self {
        Self {
            id: "email".into(),
            provider: "postmark".into(),
            postmark_token_env: "POSTMARK_SERVER_TOKEN".into(),
            from_address_env: "ARGO_CHANNEL_EMAIL_FROM".into(),
            subject_template: "[{{risk_level}}] Approval needed: {{action_id}}".into(),
            webhook_path: "/channels/email/webhook".into(),
            endpoint: "https://api.postmarkapp.com/email".into(),
        }
    }
}

/// `[hmac]` token settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct HmacManifest {
    /// Environment variable holding the HMAC secret.
    pub secret_env: String,
    /// Token time-to-live in seconds.
    pub token_ttl_seconds: i64,
}

impl Default for HmacManifest {
    fn default() -> Self {
        Self {
            secret_env: "ARGO_CHANNEL_HMAC_SECRET".into(),
            token_ttl_seconds: 86_400,
        }
    }
}

/// `[operator]` preference settings.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct OperatorManifest {
    /// Quiet hours table.
    pub quiet_hours: QuietHours,
}

/// `[action.<name>]` settings.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct ActionManifest {
    /// Approval channel settings for one action.
    pub approval: Option<ActionApprovalManifest>,
}

/// `[action.<name>.approval]` settings.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct ActionApprovalManifest {
    /// Channel ids to notify for this action.
    pub channels: Vec<String>,
}

/// `[notify_channels]` umbrella in `.skycot/notify.toml`. Parses
/// only the `fallback_chain` sub-block today; other sub-blocks
/// (parallel fan-out, per-action overrides) are out of scope per
/// the V1.4 brief.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct NotifyChannelsManifest {
    /// Optional `[notify_channels.fallback_chain]` block.
    pub fallback_chain: Option<FallbackChainManifest>,
}

impl NotifyChannelsManifest {
    /// Parse the `[notify_channels]` block from a TOML body. Body
    /// must be the inner table, e.g. the value of `notify_channels`
    /// as a top-level key.
    pub fn parse(src: &str) -> ChannelResult<Self> {
        toml::from_str(src)
            .map_err(|e| ChannelError::Parse(format!("notify_channels manifest: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_action_approval_channels() {
        let src = r#"
[channel]
postmark_token_env = "POSTMARK_TOKEN"
from_address_env = "FROM"

[action.post_listing.approval]
channels = ["email"]
"#;
        let parsed = EmailChannelManifest::parse(src).expect("parse");
        assert_eq!(parsed.approval_channels_for("post_listing"), vec!["email"]);
        assert!(parsed.approval_channels_for("draft").is_empty());
    }

    #[test]
    fn parses_notify_channels_fallback_chain() {
        let src = r#"
[fallback_chain]
order = ["slack", "email", "webinbox"]
per_step_timeout_secs = 12
"#;
        let parsed = NotifyChannelsManifest::parse(src).expect("parse");
        let chain = parsed.fallback_chain.expect("fallback_chain present");
        assert_eq!(chain.order, vec!["slack", "email", "webinbox"]);
        assert_eq!(chain.per_step_timeout_secs, 12);
    }

    #[test]
    fn notify_channels_manifest_omits_fallback_chain_when_absent() {
        let parsed = NotifyChannelsManifest::parse("").expect("parse");
        assert!(parsed.fallback_chain.is_none());
    }

    #[test]
    fn parses_rate_limit_block_with_partial_override() {
        let src = r#"
[channel]
postmark_token_env = "POSTMARK_TOKEN"
from_address_env = "FROM"

[rate_limit]
per_sec = 20
"#;
        let parsed = EmailChannelManifest::parse(src).expect("parse");
        assert_eq!(parsed.rate_limit.per_sec, Some(20));
        assert_eq!(parsed.rate_limit.per_min, None);
        assert_eq!(parsed.rate_limit.per_hour, None);
    }

    #[test]
    fn omitted_rate_limit_block_defaults_to_unlimited_override() {
        let src = r#"
[channel]
postmark_token_env = "POSTMARK_TOKEN"
from_address_env = "FROM"
"#;
        let parsed = EmailChannelManifest::parse(src).expect("parse");
        assert_eq!(parsed.rate_limit, RateLimitConfig::unlimited());
    }
}
