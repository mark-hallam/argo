//! Blue Bubbles-backed iMessage outbound arm of [`super::EnvChannelSender`].
//!
//! Mirrors the voice/SMS pattern: the per-operator chat GUID (or
//! E.164 / iMessage handle that Blue Bubbles resolves to a chat)
//! rides on the [`NotifyChannelRef::target`]; the Blue Bubbles
//! server URL + password + approval-token signing secret come from
//! env via [`super::required_env`]; the channel itself is implemented
//! in `argo_channels::imessage::IMessageChannel`.
//!
//! The `ARGO_CHANNEL_HMAC_SECRET` reused here is the same secret the
//! inbound webhook at `gateway/routes_channels_imessage.rs` verifies
//! against, so a self-consistent round-trip is possible.

use super::{required_env, send_with_rate_limit_retry};
use argo_channels::imessage::{IMessageChannel, IMessageChannelConfig};
use argo_channels::{ChannelId, Notification, RateLimitConfig};
use argo_identity::NotifyChannelRef;
use std::collections::BTreeMap;

pub(super) async fn send_imessage(
    ref_: &NotifyChannelRef,
    n: &Notification,
) -> Result<(), String> {
    let route = ref_.target.trim();
    if route.is_empty() {
        return Err(
            "imessage not configured: channel target must be the operator chat GUID or handle"
                .into(),
        );
    }
    let server_url = required_env("imessage", "BLUE_BUBBLES_URL")?;
    let password = required_env("imessage", "BLUE_BUBBLES_PASSWORD")?;
    let approval_secret = required_env("imessage", "ARGO_CHANNEL_HMAC_SECRET")?;
    // tenant-unscoped: worker action handler called from queue without tenant context yet (follow-up: pass via WorkerAction envelope)
    let webhook_secret = argo_config::cred::get_unscoped("BLUE_BUBBLES_WEBHOOK_SECRET")
        .ok()
        .filter(|s| !s.trim().is_empty());
    let mut operator_routes = BTreeMap::new();
    operator_routes.insert(n.target.operator_id.clone(), route.to_string());
    let cfg = IMessageChannelConfig {
        channel_id: ChannelId::new("imessage"),
        server_url,
        password,
        approval_secret,
        webhook_secret,
        operator_routes,
        rate_limit: RateLimitConfig::default(),
        quiet_hours: None,
    };
    let channel = IMessageChannel::new(cfg).map_err(|e| e.to_string())?;
    send_with_rate_limit_retry("imessage", &channel, n).await
}
