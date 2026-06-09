//! Outbound transactional-email provider for the magic-link sign-in
//! flow (mission-magic-link-v1x-hardening).
//!
//! The default [`EmailOutboundProvider::Noop`] preserves the v1.0
//! behavior where the gateway logs the magic-link URL via
//! `tracing::warn!`. Operators flip to `postmark`, `sendgrid`, or
//! `smtp` in v1.x by editing `[email.outbound]` in `argo.toml`.
//!
//! # Brief deviation
//!
//! The mission brief (`mission-magic-link-v1x-hardening`) lists this
//! module at `crates/argo-channels-email/src/outbound.rs (NEW or
//! existing)`. No `argo-channels-email` crate exists on
//! `origin/main`; creating one would have required a workspace-
//! member add + `Cargo.lock` regen + dep wiring well outside the
//! brief's may-touch budget. The only consumer is the gateway's
//! magic-link handler, so the module lives alongside that handler.
//! The provider trait + impls match the brief's intent.

use argo_config::{EmailOutboundConfig, EmailOutboundProvider};
use argo_email_templates::RenderedEmail;

/// One outbound message to one recipient.
pub(super) struct OutboundMessage<'a> {
    /// Recipient email.
    pub to: &'a str,
    /// "From" address resolved from `[email.outbound] from_address`.
    pub from: &'a str,
    /// Rendered template — subject + HTML + plaintext.
    pub rendered: &'a RenderedEmail,
}

/// Send `msg` via the provider selected by `cfg`. Returns `Ok(())` on
/// successful dispatch or an opaque string reason on failure; the
/// caller logs and swallows errors so the signup flow always returns
/// 200 (the brief's "no retry, silent fail" stance).
pub(super) async fn send(cfg: &EmailOutboundConfig, msg: OutboundMessage<'_>) -> Result<(), String> {
    match cfg.provider {
        EmailOutboundProvider::Noop => send_noop(msg),
        EmailOutboundProvider::Postmark => {
            send_postmark(cfg.postmark_token.expose(), &cfg.from_address, msg).await
        }
        EmailOutboundProvider::Sendgrid => {
            send_sendgrid(cfg.sendgrid_api_key.expose(), &cfg.from_address, msg).await
        }
        EmailOutboundProvider::Smtp => {
            send_smtp(
                &cfg.smtp_host,
                if cfg.smtp_port == 0 { 587 } else { cfg.smtp_port },
                &cfg.smtp_username,
                cfg.smtp_password.expose(),
                msg,
            )
            .await
        }
    }
}

fn send_noop(msg: OutboundMessage<'_>) -> Result<(), String> {
    tracing::warn!(
        target: "gateway::email_outbound",
        provider = "noop",
        recipient = %msg.to,
        subject = %msg.rendered.subject,
        "outbound email skipped (provider=noop); operator must copy link from prior magic-link log"
    );
    Ok(())
}

async fn send_postmark(
    token: &str,
    from: &str,
    msg: OutboundMessage<'_>,
) -> Result<(), String> {
    if token.is_empty() {
        return Err("postmark token missing".into());
    }
    if from.is_empty() {
        return Err("from_address missing".into());
    }
    let body = serde_json::json!({
        "From": from,
        "To": msg.to,
        "Subject": msg.rendered.subject,
        "HtmlBody": msg.rendered.html,
        "TextBody": msg.rendered.plaintext,
        "MessageStream": "outbound",
    });
    post_json(
        "https://api.postmarkapp.com/email",
        &[
            ("Accept", "application/json"),
            ("X-Postmark-Server-Token", token),
        ],
        &body,
        "postmark",
        msg.to,
    )
    .await
}

async fn send_sendgrid(
    api_key: &str,
    from: &str,
    msg: OutboundMessage<'_>,
) -> Result<(), String> {
    if api_key.is_empty() {
        return Err("sendgrid api key missing".into());
    }
    if from.is_empty() {
        return Err("from_address missing".into());
    }
    let body = serde_json::json!({
        "personalizations": [{ "to": [{ "email": msg.to }] }],
        "from": { "email": from },
        "subject": msg.rendered.subject,
        "content": [
            { "type": "text/plain", "value": msg.rendered.plaintext },
            { "type": "text/html",  "value": msg.rendered.html },
        ],
    });
    let auth = format!("Bearer {api_key}");
    post_json(
        "https://api.sendgrid.com/v3/mail/send",
        &[("Authorization", auth.as_str())],
        &body,
        "sendgrid",
        msg.to,
    )
    .await
}

async fn post_json(
    url: &str,
    headers: &[(&str, &str)],
    body: &serde_json::Value,
    provider: &'static str,
    recipient: &str,
) -> Result<(), String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("build client: {e}"))?;
    let mut req = client.post(url).json(body);
    for (name, value) in headers {
        req = req.header(*name, *value);
    }
    let resp = req
        .send()
        .await
        .map_err(|e| format!("{provider} send: {e}"))?;
    let status = resp.status();
    if !status.is_success() {
        let snippet = resp.text().await.unwrap_or_default();
        let trimmed: String = snippet.chars().take(200).collect();
        return Err(format!("{provider} {status}: {trimmed}"));
    }
    tracing::info!(
        target: "gateway::email_outbound",
        provider,
        recipient,
        "outbound email dispatched"
    );
    Ok(())
}

/// Send via SMTP with **mandatory TLS** (`lettre`, rustls backend). Real
/// transactional relays — AWS SES (`email-smtp.<region>.amazonaws.com`),
/// Gmail, etc. — reject plaintext sessions, so TLS is not optional: port
/// `465` uses implicit TLS (`relay`); every other port (587 by default,
/// resolved by the caller) uses STARTTLS (`starttls_relay`). The message is
/// sent as a multipart/alternative (plaintext + HTML) so the rendered
/// magic-link template arrives intact in every client. Credentials are
/// presented only when a username is configured — SES and Gmail always set
/// one (SES SMTP username/password generated in the SES console).
///
/// This replaces the previous hand-rolled plain-TCP SMTP, which could not
/// authenticate to any TLS-only relay (the documented reason the only working
/// magic-link delivery was `noop` + copy-from-log).
async fn send_smtp(
    host: &str,
    port: u16,
    username: &str,
    password: &str,
    msg: OutboundMessage<'_>,
) -> Result<(), String> {
    use lettre::message::{Mailbox, MultiPart};
    use lettre::transport::smtp::authentication::Credentials;
    use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

    if host.is_empty() {
        return Err("smtp host missing".into());
    }
    if msg.from.is_empty() {
        return Err("from_address missing".into());
    }
    let from: Mailbox = msg
        .from
        .parse()
        .map_err(|e| format!("smtp from_address invalid: {e}"))?;
    let to: Mailbox = msg
        .to
        .parse()
        .map_err(|e| format!("smtp recipient invalid: {e}"))?;
    let email = Message::builder()
        .from(from)
        .to(to)
        .subject(msg.rendered.subject.clone())
        .multipart(MultiPart::alternative_plain_html(
            msg.rendered.plaintext.clone(),
            msg.rendered.html.clone(),
        ))
        .map_err(|e| format!("smtp build message: {e}"))?;

    // TLS is mandatory: 465 = implicit TLS, everything else = STARTTLS.
    let port = if port == 0 { 587 } else { port };
    let mut builder = if port == 465 {
        AsyncSmtpTransport::<Tokio1Executor>::relay(host)
            .map_err(|e| format!("smtp relay({host}): {e}"))?
    } else {
        AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(host)
            .map_err(|e| format!("smtp starttls_relay({host}): {e}"))?
    }
    .port(port);
    if !username.is_empty() {
        builder = builder.credentials(Credentials::new(
            username.to_string(),
            password.to_string(),
        ));
    }
    builder
        .build()
        .send(email)
        .await
        .map(|_| ())
        .map_err(|e| format!("smtp send: {e}"))?;
    tracing::info!(
        target: "gateway::email_outbound",
        provider = "smtp",
        recipient = %msg.to,
        "outbound email dispatched (TLS)"
    );
    Ok(())
}

#[cfg(test)]
#[path = "email_outbound_tests.rs"]
mod tests;
