use super::super::email_outbound::{self, OutboundMessage};
use argo_config::EmailOutboundConfig;
use argo_email_templates::MagicLinkEmail;
use std::sync::OnceLock;

pub(crate) async fn deliver_magic_link(email: &str, raw_token: &str) {
    let base = std::env::var("SKYCOT_PUBLIC_BASE_URL")
        .unwrap_or_else(|_| "https://app.skycot.com".to_string());
    let magic_url = format!("{base}/auth/verify?token={raw_token}");
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
    let cfg = outbound_config();
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
    if let Err(reason) = email_outbound::send(cfg, msg).await {
        tracing::warn!(
            target: "gateway::auth_magic_link",
            recipient = %email,
            provider = cfg.provider.as_str(),
            reason = %reason,
            "outbound email failed; magic link still in trace log for operator copy",
        );
    }
}

/// Load `[email.outbound]` lazily on first signup and cache it for
/// the lifetime of the process.
fn outbound_config() -> &'static EmailOutboundConfig {
    static CFG: OnceLock<EmailOutboundConfig> = OnceLock::new();
    CFG.get_or_init(|| argo_config::load().unwrap_or_default().email.outbound)
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
