//! Messaging-channel config sections: `[channels]`,
//! `[channels.teams]`, `[channels.matrix]`, `[channels.voice]`. Extracted from
//! [`crate::sections`] to keep that module under the 500-LOC ceiling.
//! Re-exported through `sections` so the public API is unchanged.
use crate::defaults;
use serde::{Deserialize, Serialize};

/// `[channels]` — parent for per-channel TOML config blocks.
/// Today `[channels.teams]`, `[channels.matrix]`, `[channels.voice]`;
/// sibling blocks land here as other channels (slack, discord,
/// whatsapp, etc.) move off env-only configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ChannelsConfig {
    /// `[channels.teams]`.
    pub teams: TeamsTomlConfig,
    /// `[channels.matrix]`.
    pub matrix: MatrixTomlConfig,
    /// `[channels.webinbox]`.
    pub webinbox: WebInboxTomlConfig,
    /// `[channels.voice]`.
    pub voice: VoiceTomlConfig,
}

/// `[channels.webinbox]` — browser inbox defaults.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct WebInboxTomlConfig {
    /// Tenant id used by gateway web-inbox routes when the request
    /// does not carry an explicit tenant.
    pub tenant_id: Option<String>,
}

/// `[channels.teams]` — Microsoft Teams (Graph API app-only auth).
///
/// The client secret is loaded indirectly from the env var named by
/// `client_secret_env` (default `ARGO_TEAMS_CLIENT_SECRET`) so the
/// TOML never carries plaintext credentials and operators can rotate
/// without restart-and-edit-config.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TeamsTomlConfig {
    /// Master enable. Defaults to `false` — opt-in.
    pub enabled: bool,
    /// Azure-AD tenant id.
    pub tenant_id: String,
    /// Azure-AD app (client) id.
    pub client_id: String,
    /// Env var name holding the client secret.
    pub client_secret_env: String,
    /// Default team id used when no explicit team is supplied.
    pub default_team_id: Option<String>,
    /// Default channel id used when no explicit channel is supplied.
    pub default_channel_id: Option<String>,
    /// When `true`, body markdown is converted to HTML before send.
    pub markdown_to_html: bool,
}

impl Default for TeamsTomlConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            tenant_id: String::new(),
            client_id: String::new(),
            client_secret_env: defaults::DEFAULT_TEAMS_CLIENT_SECRET_ENV.into(),
            default_team_id: None,
            default_channel_id: None,
            markdown_to_html: true,
        }
    }
}

/// `[channels.matrix]` — Matrix Client-Server API r0.6.1 cleartext
/// send.
///
/// The access token is loaded indirectly from the env var named by
/// `access_token_env` (default `ARGO_MATRIX_ACCESS_TOKEN`). End-to-
/// end encryption (Olm/Megolm) is out of scope for v1; see the
/// follow-up brief for the encrypted path.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MatrixTomlConfig {
    /// Master enable. Defaults to `false` — opt-in.
    pub enabled: bool,
    /// Homeserver base URL (e.g. `https://matrix.org`).
    pub homeserver_url: String,
    /// Bot user id, `@local:domain` form.
    pub user_id: String,
    /// Env var name holding the long-lived access token.
    pub access_token_env: String,
    /// Default room id (`!opaque:domain`); explicit override wins.
    pub default_room_id: Option<String>,
}

impl Default for MatrixTomlConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            homeserver_url: defaults::DEFAULT_MATRIX_HOMESERVER_URL.into(),
            user_id: String::new(),
            access_token_env: defaults::DEFAULT_MATRIX_ACCESS_TOKEN_ENV.into(),
            default_room_id: None,
        }
    }
}

/// `[channels.voice]` — Twilio (or Plivo, in v1.x) outbound voice
/// approval calls. Mirrors the `[channels.teams]` env-indirect
/// pattern: the TOML carries non-secret identifiers (account SID,
/// from-number, provider name) and a `*_env` field naming the env
/// var that holds the actual auth token. Operators rotate secrets
/// without editing config.
///
/// Today the dispatcher (`worker_actions_notify_voice`) still reads
/// the Twilio credentials directly from env so that the same
/// `TWILIO_*` setup that backs SMS continues to work; this block
/// exists so the auto-approval gate (and future config-driven
/// channel registry) can discover that voice is enabled and which
/// env var to consult.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct VoiceTomlConfig {
    /// Master enable. Defaults to `false` — opt-in.
    pub enabled: bool,
    /// Provider id (`twilio` in v1; `plivo` and `vonage` are
    /// DEFERRED-v1.x).
    pub provider: String,
    /// Twilio Account SID (`AC…`). Non-secret; safe to commit.
    pub account_sid: String,
    /// Env var name holding the provider auth token.
    pub auth_token_env: String,
    /// Twilio caller phone number (E.164, e.g. `+15551234567`).
    pub from_number: String,
}

impl Default for VoiceTomlConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: defaults::DEFAULT_VOICE_PROVIDER.into(),
            account_sid: String::new(),
            auth_token_env: defaults::DEFAULT_VOICE_AUTH_TOKEN_ENV.into(),
            from_number: String::new(),
        }
    }
}

#[cfg(test)]
mod voice_config_tests {
    #[test]
    fn voice_toml_config_parses_with_defaults() {
        let cfg = crate::ArgoConfig::from_toml(
            "[channels.voice]\n\
             enabled = true\n\
             account_sid = \"AC123\"\n\
             from_number = \"+15550000000\"\n",
        )
        .unwrap();

        assert!(cfg.channels.voice.enabled);
        assert_eq!(cfg.channels.voice.provider, "twilio");
        assert_eq!(cfg.channels.voice.account_sid, "AC123");
        assert_eq!(cfg.channels.voice.auth_token_env, "TWILIO_AUTH_TOKEN");
        assert_eq!(cfg.channels.voice.from_number, "+15550000000");
    }
}
