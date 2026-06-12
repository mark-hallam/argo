use super::super::email_outbound::{self, OutboundMessage};
use argo_config::{EmailOutboundConfig, EmailOutboundProvider};
use argo_email_templates::MagicLinkEmail;
use argo_types::Redacted;
use std::sync::OnceLock;

/// Outcome of one magic-link delivery attempt, threaded into the
/// signup response so the SPA can show an honest state instead of an
/// unconditional "Check your inbox".
///
/// `launch-email-delivery-honesty-and-public-base-url`: the previous
/// flow swallowed every send failure into a warn log and always told
/// the stranger "sent". A user whose email never goes out then waits
/// forever for a link that was never delivered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DeliveryStatus {
    /// A real provider accepted the message for delivery.
    Sent,
    /// A real provider was configured but the send failed (network,
    /// auth, provider 4xx/5xx). The provider error detail is logged +
    /// auditable; it is NEVER returned to the client.
    DeliveryFailed,
    /// No real outbound provider is configured (`provider = "noop"`):
    /// the link was only written to the trace log for an operator to
    /// copy. The honest state for a self-serve deploy that forgot to
    /// wire `[email.outbound]` / the SES env vars.
    NotConfigured,
}

impl DeliveryStatus {
    /// Client-facing status string for the signup JSON response. The
    /// SPA's check-email page keys its honest copy on these literals.
    pub(crate) fn as_response_status(self) -> &'static str {
        match self {
            DeliveryStatus::Sent => "sent",
            DeliveryStatus::DeliveryFailed => "delivery_failed",
            DeliveryStatus::NotConfigured => "not_configured",
        }
    }
}

/// Public base URL used to mint links. Honors the first-class deploy
/// var `SKYCOT_PUBLIC_BASE_URL`; falls back to the historical default
/// so dev / demo runs are unchanged.
pub(crate) fn public_base_url() -> String {
    base_url_from(std::env::var("SKYCOT_PUBLIC_BASE_URL").ok())
}

/// Pure resolution of the public base URL from a raw env value, split
/// out so it is unit-testable without mutating the process environment.
/// Whitespace-only values are treated as unset and a trailing slash is
/// trimmed.
pub(crate) fn base_url_from(raw: Option<String>) -> String {
    raw.map(|v| v.trim().trim_end_matches('/').to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "https://app.skycot.com".to_string())
}

pub(crate) async fn deliver_magic_link(email: &str, raw_token: &str) -> DeliveryStatus {
    deliver_with_config(outbound_config(), email, raw_token).await
}

/// Core delivery path, parameterized on the resolved config so it is
/// unit-testable per provider without touching the process-global
/// [`outbound_config`] cache.
pub(crate) async fn deliver_with_config(
    cfg: &EmailOutboundConfig,
    email: &str,
    raw_token: &str,
) -> DeliveryStatus {
    let magic_url = format!("{}/auth/verify?token={raw_token}", public_base_url());
    let rendered = MagicLinkEmail {
        magic_url: magic_url.clone(),
        email: email.to_string(),
        expires_in: "15 minutes".to_string(),
    }
    .render();
    tracing::warn!(
        target: "gateway::auth_magic_link",
        recipient = %email,
        subject = %rendered.subject,
        link = %magic_url,
        "magic link issued",
    );
    let from = if cfg.from_address.is_empty() {
        "noreply@skycot.com"
    } else {
        cfg.from_address.as_str()
    };
    let msg = OutboundMessage {
        to: email,
        from,
        rendered: &rendered,
    };
    // provider=noop never actually sends — be honest about it rather
    // than reporting "sent". The link is still in the trace log above
    // for an operator to copy.
    if cfg.provider == EmailOutboundProvider::Noop {
        let _ = email_outbound::send(cfg, msg).await;
        return DeliveryStatus::NotConfigured;
    }
    match email_outbound::send(cfg, msg).await {
        Ok(()) => DeliveryStatus::Sent,
        Err(reason) => {
            // Provider detail stays server-side (log + audit); the
            // client only ever sees the generic `delivery_failed`.
            tracing::warn!(
                target: "gateway::auth_magic_link",
                recipient = %email,
                provider = cfg.provider.as_str(),
                reason = %reason,
                "outbound email failed; magic link still in trace log for operator copy",
            );
            DeliveryStatus::DeliveryFailed
        }
    }
}

/// Load `[email.outbound]` lazily on first signup and cache it for the
/// lifetime of the process. Env-var overrides (the first-class deploy
/// surface — a production container has no `argo.toml`) take precedence
/// over the loaded TOML so a by-the-book AWS/SES deploy wires real
/// delivery without an in-image config file.
fn outbound_config() -> &'static EmailOutboundConfig {
    static CFG: OnceLock<EmailOutboundConfig> = OnceLock::new();
    CFG.get_or_init(|| {
        let mut cfg = argo_config::load().unwrap_or_default().email.outbound;
        apply_env_overrides(&mut cfg);
        cfg
    })
}

/// Override `[email.outbound]` fields from `SKYCOT_EMAIL_*` /
/// `SKYCOT_SMTP_*` env vars when set + non-empty. Mirrors the names in
/// `deploy/secrets/skycot-production.env.example` and the ECS task env.
fn apply_env_overrides(cfg: &mut EmailOutboundConfig) {
    if let Some(p) = env_nonempty("SKYCOT_EMAIL_PROVIDER") {
        if let Some(provider) = parse_provider(&p) {
            cfg.provider = provider;
        }
    }
    if let Some(v) = env_nonempty("SKYCOT_EMAIL_FROM_ADDRESS") {
        cfg.from_address = v;
    }
    if let Some(v) = env_nonempty("SKYCOT_SMTP_HOST") {
        cfg.smtp_host = v;
    }
    if let Some(v) = env_nonempty("SKYCOT_SMTP_PORT") {
        if let Ok(port) = v.parse::<u16>() {
            cfg.smtp_port = port;
        }
    }
    if let Some(v) = env_nonempty("SKYCOT_SMTP_USERNAME") {
        cfg.smtp_username = v;
    }
    if let Some(v) = env_nonempty("SKYCOT_SMTP_PASSWORD") {
        cfg.smtp_password = Redacted::new(v);
    }
    if let Some(v) = env_nonempty("SKYCOT_POSTMARK_TOKEN") {
        cfg.postmark_token = Redacted::new(v);
    }
    if let Some(v) = env_nonempty("SKYCOT_SENDGRID_API_KEY") {
        cfg.sendgrid_api_key = Redacted::new(v);
    }
}

fn env_nonempty(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn parse_provider(s: &str) -> Option<EmailOutboundProvider> {
    match s.trim().to_ascii_lowercase().as_str() {
        "noop" => Some(EmailOutboundProvider::Noop),
        "postmark" => Some(EmailOutboundProvider::Postmark),
        "sendgrid" => Some(EmailOutboundProvider::Sendgrid),
        "smtp" => Some(EmailOutboundProvider::Smtp),
        _ => None,
    }
}

pub(crate) fn looks_like_email(s: &str) -> bool {
    let Some((local, domain)) = s.split_once('@') else {
        return false;
    };
    s.len() >= 3
        && s.len() <= 254
        && !local.is_empty()
        && domain.contains('.')
        && !domain.starts_with('.')
}
