use super::{approval_payload, post_json, required_env};
use argo_channels::Notification;

pub(super) async fn send_sse(n: &Notification) -> Result<(), String> {
    let url = required_env("sse", "SSE_NOTIFY_URL")?;
    let mut payload = approval_payload("sse", n);
    if let Some(map) = payload.as_object_mut() {
        map.insert(
            "event".into(),
            serde_json::Value::String("notify_ask".into()),
        );
    }
    post_json("sse", &url, payload).await
}
