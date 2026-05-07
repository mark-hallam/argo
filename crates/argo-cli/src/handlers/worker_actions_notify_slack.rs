use super::{approval_payload, post_json, required_env};
use argo_channels::Notification;
use serde_json::json;

pub(super) async fn send_slack(n: &Notification) -> Result<(), String> {
    let url = required_env("slack", "SLACK_WEBHOOK_URL")?;
    let payload = json!({
        "text": format!("Approval needed for {}", n.action_id),
        "blocks": [
            {
                "type": "section",
                "text": {
                    "type": "mrkdwn",
                    "text": format!(
                        "*Approval needed*\n{}\n*Risk:* `{}`\n*Action:* `{}`",
                        n.prompt_body, n.risk_level, n.action_id
                    )
                }
            },
            {
                "type": "context",
                "elements": [{
                    "type": "mrkdwn",
                    "text": format!("approval_id `{}` expires `{}`", n.approval_id, n.expiry)
                }]
            },
            {
                "type": "actions",
                "elements": [
                    slack_button("Approve", "primary", &n.approve_url),
                    slack_button("Reject", "danger", &n.reject_url)
                ]
            }
        ],
        "skycot": approval_payload("slack", n),
    });
    post_json("slack", &url, payload).await
}

fn slack_button(text: &str, style: &str, url: &str) -> serde_json::Value {
    json!({
        "type": "button",
        "text": {"type": "plain_text", "text": text},
        "style": style,
        "url": url,
    })
}
