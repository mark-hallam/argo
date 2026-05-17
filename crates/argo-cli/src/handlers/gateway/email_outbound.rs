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
use base64::Engine;

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

/// Minimal SMTP send: connect, EHLO, AUTH PLAIN, MAIL FROM, RCPT TO,
/// DATA, `.`, QUIT. Plain TCP only — operators wanting TLS deploy
/// behind a smarthost or wait for the v1.y `lettre` follow-up.
async fn send_smtp(
    host: &str,
    port: u16,
    username: &str,
    password: &str,
    msg: OutboundMessage<'_>,
) -> Result<(), String> {
    use tokio::io::{AsyncWriteExt, BufReader};
    use tokio::net::TcpStream;

    if host.is_empty() {
        return Err("smtp host missing".into());
    }
    let stream = TcpStream::connect((host, port))
        .await
        .map_err(|e| format!("smtp connect {host}:{port}: {e}"))?;
    let (read, mut write) = stream.into_split();
    let mut reader = BufReader::new(read);
    expect_smtp_code(&mut reader, "220").await?;
    write_smtp_line(&mut write, "EHLO skycot.local").await?;
    drain_smtp_until(&mut reader, "250").await?;
    if !username.is_empty() {
        let plain = base64::engine::general_purpose::STANDARD
            .encode(format!("\0{username}\0{password}"));
        write_smtp_line(&mut write, &format!("AUTH PLAIN {plain}")).await?;
        expect_smtp_code(&mut reader, "235").await?;
    }
    write_smtp_line(&mut write, &format!("MAIL FROM:<{}>", msg.from)).await?;
    expect_smtp_code(&mut reader, "250").await?;
    write_smtp_line(&mut write, &format!("RCPT TO:<{}>", msg.to)).await?;
    expect_smtp_code(&mut reader, "250").await?;
    write_smtp_line(&mut write, "DATA").await?;
    expect_smtp_code(&mut reader, "354").await?;
    let data = format!(
        "From: {}\r\nTo: {}\r\nSubject: {}\r\nMIME-Version: 1.0\r\nContent-Type: text/plain; charset=utf-8\r\n\r\n{}\r\n.\r\n",
        msg.from, msg.to, msg.rendered.subject, msg.rendered.plaintext
    );
    write.write_all(data.as_bytes()).await.map_err(|e| format!("smtp data: {e}"))?;
    expect_smtp_code(&mut reader, "250").await?;
    write_smtp_line(&mut write, "QUIT").await?;
    tracing::info!(
        target: "gateway::email_outbound",
        provider = "smtp",
        recipient = %msg.to,
        "outbound email dispatched"
    );
    Ok(())
}

async fn write_smtp_line(
    write: &mut tokio::net::tcp::OwnedWriteHalf,
    line: &str,
) -> Result<(), String> {
    use tokio::io::AsyncWriteExt;
    write
        .write_all(format!("{line}\r\n").as_bytes())
        .await
        .map_err(|e| format!("smtp write: {e}"))
}

async fn expect_smtp_code<R: tokio::io::AsyncBufRead + Unpin>(
    reader: &mut R,
    code: &str,
) -> Result<(), String> {
    let mut line = String::new();
    use tokio::io::AsyncBufReadExt;
    reader
        .read_line(&mut line)
        .await
        .map_err(|e| format!("smtp read: {e}"))?;
    if !line.starts_with(code) {
        return Err(format!("smtp expected {code}, got {}", line.trim()));
    }
    Ok(())
}

async fn drain_smtp_until<R: tokio::io::AsyncBufRead + Unpin>(
    reader: &mut R,
    code: &str,
) -> Result<(), String> {
    use tokio::io::AsyncBufReadExt;
    loop {
        let mut line = String::new();
        reader
            .read_line(&mut line)
            .await
            .map_err(|e| format!("smtp read: {e}"))?;
        if !line.starts_with(code) {
            return Err(format!("smtp expected {code}, got {}", line.trim()));
        }
        // Multi-line replies use `code-` for continuation, `code ` for terminator.
        let after_code = line.chars().nth(code.len()).unwrap_or(' ');
        if after_code != '-' {
            return Ok(());
        }
    }
}

#[cfg(test)]
#[path = "email_outbound_tests.rs"]
mod tests;
