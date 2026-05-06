//! Channel manifest parsing.

use crate::email::QuietHours;
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
}
