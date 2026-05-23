//! Twilio Voice outbound arm of [`super::EnvChannelSender`].
//!
//! Mirrors the SMS pattern: the per-operator phone number rides on the
//! [`NotifyChannelRef::target`] (E.164), credentials come from env via
//! [`super::required_env`], and the channel itself is implemented in
//! `argo-channels::voice::VoiceApprovalChannel`. The HMAC secret used to
//! sign the DTMF callback URL is `ARGO_CHANNEL_HMAC_SECRET` — the same
//! secret the inbound webhook at `gateway/routes_channels_voice.rs`
//! verifies against, so a self-consistent round-trip is possible.

use super::{required_env, send_with_rate_limit_retry};
use argo_channels::voice::{VoiceApprovalChannel, VoiceApprovalChannelConfig};
use argo_channels::{ChannelId, Notification, RateLimitConfig};
use argo_identity::NotifyChannelRef;
use std::collections::BTreeMap;

pub(super) async fn send_voice(ref_: &NotifyChannelRef, n: &Notification) -> Result<(), String> {
    let to = ref_.target.trim();
    if to.is_empty() {
        return Err(
            "voice not configured: channel target must be the operator E.164 number".into(),
        );
    }
    let account_sid = required_env("voice", "TWILIO_ACCOUNT_SID")?;
    let auth_token = required_env("voice", "TWILIO_AUTH_TOKEN")?;
    let from_number = first_env("voice", &["TWILIO_VOICE_FROM_NUMBER", "TWILIO_FROM_NUMBER"])?;
    let decision_callback_base_url = required_env("voice", "ARGO_TWILIO_VOICE_DECISION_URL")?;
    let hmac_secret = required_env("voice", "ARGO_CHANNEL_HMAC_SECRET")?;
    // tenant-unscoped: worker action handler called from queue without tenant context yet (follow-up: pass via WorkerAction envelope)
    let base = argo_config::cred::get_unscoped("TWILIO_API_BASE_URL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "https://api.twilio.com".into());
    let endpoint = format!(
        "{}/2010-04-01/Accounts/{}/Calls.json",
        base.trim_end_matches('/'),
        account_sid
    );
    let mut operator_phone_numbers = BTreeMap::new();
    operator_phone_numbers.insert(n.target.operator_id.clone(), to.to_string());
    let cfg = VoiceApprovalChannelConfig {
        channel_id: ChannelId::new("voice"),
        account_sid,
        auth_token,
        from_number,
        endpoint,
        decision_callback_base_url,
        hmac_secret,
        operator_phone_numbers,
        rate_limit: RateLimitConfig::default(),
        quiet_hours: None,
    };
    let channel = VoiceApprovalChannel::new(cfg);
    send_with_rate_limit_retry("voice", &channel, n).await
}

fn first_env(channel: &str, names: &[&str]) -> Result<String, String> {
    for name in names {
        // tenant-unscoped: worker action handler called from queue without tenant context yet (follow-up: pass via WorkerAction envelope)
        if let Ok(v) = argo_config::cred::get_unscoped(name) {
            if !v.trim().is_empty() {
                return Ok(v);
            }
        }
    }
    Err(format!(
        "{channel} not configured: set {}",
        names.join(" or ")
    ))
}
