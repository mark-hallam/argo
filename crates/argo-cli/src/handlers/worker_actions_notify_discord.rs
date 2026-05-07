use super::{approval_payload, post_json, required_env};
use argo_channels::Notification;
use serde_json::json;

pub(super) async fn send_discord(n: &Notification) -> Result<(), String> {
    let url = required_env("discord", "DISCORD_WEBHOOK_URL")?;
    let payload = json!({
        "content": format!("Approval needed for `{}`", n.action_id),
        "embeds": [{
            "title": "Approval needed",
            "description": n.prompt_body,
            "fields": [
                {"name": "Action", "value": n.action_id, "inline": true},
                {"name": "Risk", "value": n.risk_level.to_string(), "inline": true},
                {"name": "Approve", "value": n.approve_url},
                {"name": "Reject", "value": n.reject_url}
            ],
        }],
        "skycot": approval_payload("discord", n),
    });
    post_json("discord", &url, payload).await
}
