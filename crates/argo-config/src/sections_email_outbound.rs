//! `[email.outbound]` config block — operator picks the magic-link
//! delivery channel (mission-magic-link-v1x-hardening).
//!
//! Default is `provider = "noop"`, which preserves the v1.0 demo
//! behavior: the gateway logs the magic-link URL via `tracing::warn!`
//! so the operator can copy it from the terminal. Real outbound for
//! v1.x flips to `postmark`, `sendgrid`, or `smtp`.

use argo_types::Redacted;
use serde::{Deserialize, Serialize};

/// `[email]` — top-level email config (currently just outbound).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct EmailConfig {
    /// `[email.outbound]` — magic-link delivery provider.
    pub outbound: EmailOutboundConfig,
}

/// Outbound provider selector.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EmailOutboundProvider {
    /// Default. Renders the email and logs the magic-link URL via
    /// `tracing::warn!`; no network call. Preserves v1.0 demo flow.
    #[default]
    Noop,
    /// Postmark transactional API (`POST /email`).
    Postmark,
    /// SendGrid Mail Send v3 API (`POST /v3/mail/send`).
    Sendgrid,
    /// RFC 5321 SMTP send.
    Smtp,
}

/// `[email.outbound]` — provider + credentials + from-address.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct EmailOutboundConfig {
    /// Active provider. Defaults to [`EmailOutboundProvider::Noop`].
    pub provider: EmailOutboundProvider,
    /// "From" address stamped on outbound mail. Required when
    /// `provider != "noop"`; ignored otherwise.
    pub from_address: String,
    /// Postmark server token; `Redacted` so it never leaks into
    /// serialized config dumps. Used when `provider = "postmark"`.
    pub postmark_token: Redacted<String>,
    /// SendGrid API key. Used when `provider = "sendgrid"`.
    pub sendgrid_api_key: Redacted<String>,
    /// SMTP host. Used when `provider = "smtp"`.
    pub smtp_host: String,
    /// SMTP port (default 587 in operator config; this struct has no
    /// per-field default, the consumer falls back to 587).
    pub smtp_port: u16,
    /// SMTP username for AUTH PLAIN.
    pub smtp_username: String,
    /// SMTP password for AUTH PLAIN.
    pub smtp_password: Redacted<String>,
}

impl EmailOutboundProvider {
    /// Stable lowercase name used in tracing fields and metric labels.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Noop => "noop",
            Self::Postmark => "postmark",
            Self::Sendgrid => "sendgrid",
            Self::Smtp => "smtp",
        }
    }
}
