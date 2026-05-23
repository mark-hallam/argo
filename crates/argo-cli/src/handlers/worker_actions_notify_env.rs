use super::notify_dispatch::NotifySender;
use super::{
    notify_chat, notify_discord, notify_email, notify_imessage, notify_slack, notify_sms,
    notify_sse, notify_voice,
};
use argo_channels::{ChannelError, Notification, NotifyChannel};
use argo_identity::NotifyChannelRef;
use serde_json::{json, Value};
use std::time::Duration;

/// Production [`NotifySender`] backed by process env / workspace
/// credentials.
pub struct EnvChannelSender;

#[async_trait::async_trait]
impl NotifySender for EnvChannelSender {
    async fn send_via(&self, ref_: &NotifyChannelRef, n: &Notification) -> Result<(), String> {
        match ref_.channel.as_str() {
            "push" => send_push(n).await,
            "webinbox" => send_webinbox(n).await,
            "slack" => notify_slack::send_slack(n).await,
            "imessage" => notify_imessage::send_imessage(ref_, n).await,
            "email" => notify_email::send_email(ref_, n).await,
            "sms" => notify_sms::send_sms(ref_, n).await,
            "discord" => notify_discord::send_discord(n).await,
            "chat" => notify_chat::send_chat(n).await,
            "sse" => notify_sse::send_sse(n).await,
            "voice" => notify_voice::send_voice(ref_, n).await,
            other => Err(format!(
                "channel kind '{other}' is not supported by dispatch_notify \
                 (v1 channels: push, webinbox, slack, imessage, email, sms, discord, chat, sse, voice; \
                 DEFERRED-v1.x: whatsapp, teams, matrix)"
            )),
        }
    }
}

pub(super) fn required_env(channel: &str, name: &str) -> Result<String, String> {
    // tenant-unscoped: worker action handler called from queue without tenant context yet (follow-up: pass via WorkerAction envelope)
    let value = argo_config::cred::get_unscoped(name)
        .map_err(|_| format!("{channel} not configured: set {name}"))?;
    if value.trim().is_empty() {
        Err(format!("{channel} not configured: set {name}"))
    } else {
        Ok(value)
    }
}

pub(super) fn approval_payload(channel: &str, n: &Notification) -> Value {
    json!({
        "type": "notify_ask",
        "channel": channel,
        "action_id": n.action_id,
        "approval_id": n.approval_id,
        "risk_level": n.risk_level,
        "summary": n.prompt_body,
        "approve_url": n.approve_url,
        "reject_url": n.reject_url,
        "expires_at": n.expiry,
    })
}

pub(super) async fn post_json(channel: &str, url: &str, payload: Value) -> Result<(), String> {
    let resp = reqwest::Client::new()
        .post(url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("{channel} send failed: {e}"))?;
    let status = resp.status();
    let body = match resp.text().await {
        Ok(body) => body,
        Err(e) => format!("failed to read response body: {e}"),
    };
    if status.is_success() {
        Ok(())
    } else {
        Err(format!("{channel} returned {status}: {body}"))
    }
}

async fn send_push(n: &Notification) -> Result<(), String> {
    use argo_channels::push::{PushApprovalChannel, PushApprovalChannelConfig};
    let cfg = PushApprovalChannelConfig::from_env();
    let channel = PushApprovalChannel::new(cfg);
    send_with_rate_limit_retry("push", &channel, n).await
}

const RATE_LIMIT_MAX_BACKOFF: Duration = Duration::from_secs(60);
const RATE_LIMIT_MAX_RETRIES: usize = 1;

pub(super) async fn send_with_rate_limit_retry<C: NotifyChannel + Sync>(
    channel_label: &str,
    channel: &C,
    n: &Notification,
) -> Result<(), String> {
    for attempt in 0..=RATE_LIMIT_MAX_RETRIES {
        match channel.send(n.clone()).await {
            Ok(_) => return Ok(()),
            Err(ChannelError::RateLimited { retry_after }) if attempt < RATE_LIMIT_MAX_RETRIES => {
                let backoff = retry_after.min(RATE_LIMIT_MAX_BACKOFF);
                tracing::warn!(
                    channel = channel_label,
                    attempt,
                    retry_after_ms = u64::try_from(backoff.as_millis()).unwrap_or(u64::MAX),
                    "rate-limited; backing off before retry",
                );
                tokio::time::sleep(backoff).await;
            }
            Err(e) => return Err(e.to_string()),
        }
    }
    Err(format!(
        "{channel_label} rate-limited after {} retries",
        RATE_LIMIT_MAX_RETRIES
    ))
}

async fn send_webinbox(n: &Notification) -> Result<(), String> {
    use argo_channels::webinbox::{WebInboxChannel, WebInboxChannelConfig};
    let inbox_path = std::env::var("ARGO_WEBINBOX_PATH")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| ".skycot/webinbox.jsonl".to_string());
    let channel = WebInboxChannel::new(WebInboxChannelConfig::local(inbox_path));
    channel
        .send(n.clone())
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}
