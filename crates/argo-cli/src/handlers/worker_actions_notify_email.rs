use super::required_env;
use argo_channels::Notification;
use argo_identity::NotifyChannelRef;
use lettre::message::{Mailbox, Message};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Tokio1Executor};

pub(super) async fn send_email(ref_: &NotifyChannelRef, n: &Notification) -> Result<(), String> {
    let host = required_env("email", "SMTP_HOST")?;
    let port = required_env("email", "SMTP_PORT")?
        .parse::<u16>()
        .map_err(|e| format!("email not configured: SMTP_PORT must be a port number: {e}"))?;
    let user = required_env("email", "SMTP_USER")?;
    let pass = required_env("email", "SMTP_PASS")?;
    let from = required_env("email", "SMTP_FROM")?;
    let to = if ref_.target.trim().is_empty() {
        n.target.email.as_str()
    } else {
        ref_.target.as_str()
    };
    let message = build_email(&from, to, n)?;
    let creds = Credentials::new(user, pass);
    let mailer = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&host)
        .port(port)
        .credentials(creds)
        .build();
    mailer
        .send(message)
        .await
        .map(|_| ())
        .map_err(|e| format!("email send failed: {e}"))
}

pub(super) fn build_email(from: &str, to: &str, n: &Notification) -> Result<Message, String> {
    let from = from
        .parse::<Mailbox>()
        .map_err(|e| format!("email not configured: SMTP_FROM invalid: {e}"))?;
    let to = to
        .parse::<Mailbox>()
        .map_err(|e| format!("email target invalid: {e}"))?;
    Message::builder()
        .from(from)
        .to(to)
        .subject(format!(
            "[{}] Approval needed: {}",
            n.risk_level, n.action_id
        ))
        .body(format!(
            "{}\n\nApprove: {}\nReject: {}\n\napproval_id: {}\nexpires: {}\n",
            n.prompt_body, n.approve_url, n.reject_url, n.approval_id, n.expiry
        ))
        .map_err(|e| format!("email build failed: {e}"))
}
