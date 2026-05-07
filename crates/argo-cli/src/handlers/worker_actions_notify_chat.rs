use super::{approval_payload, post_json, required_env};
use argo_channels::Notification;

pub(super) async fn send_chat(n: &Notification) -> Result<(), String> {
    let url = required_env("chat", "CHAT_WEBHOOK_URL")?;
    post_json("chat", &url, approval_payload("chat", n)).await
}
