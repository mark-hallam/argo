use argo_channels::voice::{VoiceApprovalChannel, VoiceApprovalChannelConfig};
use argo_channels::{ChannelId, Notification, NotifyChannel, OperatorTarget, RiskLevel};
use chrono::{Duration, Utc};
use std::collections::BTreeMap;

#[tokio::test]
async fn twilio_voice_integration_gated_on_env() {
    let from = match std::env::var("TWILIO_VOICE_FROM_NUMBER") {
        Ok(v) if !v.is_empty() => v,
        _ => return,
    };
    let sid = match std::env::var("TWILIO_TEST_ACCOUNT_SID")
        .or_else(|_| std::env::var("TWILIO_ACCOUNT_SID"))
    {
        Ok(v) if !v.is_empty() => v,
        _ => return,
    };
    let token = match std::env::var("TWILIO_TEST_AUTH_TOKEN")
        .or_else(|_| std::env::var("TWILIO_AUTH_TOKEN"))
    {
        Ok(v) if !v.is_empty() => v,
        _ => return,
    };
    let to = match std::env::var("TWILIO_VOICE_TO_NUMBER")
        .or_else(|_| std::env::var("TWILIO_TEST_TO"))
    {
        Ok(v) if !v.is_empty() => v,
        _ => return,
    };
    let mut phones = BTreeMap::new();
    phones.insert("op-1".into(), to);
    let channel = VoiceApprovalChannel::new(VoiceApprovalChannelConfig {
        channel_id: ChannelId::new("voice"),
        endpoint: format!("https://api.twilio.com/2010-04-01/Accounts/{sid}/Calls.json"),
        account_sid: sid,
        auth_token: token,
        from_number: from,
        decision_callback_base_url: std::env::var("ARGO_TWILIO_VOICE_DECISION_URL")
            .unwrap_or_else(|_| "https://example.com/twilio/voice/decision".into()),
        hmac_secret: std::env::var("ARGO_CHANNEL_HMAC_SECRET")
            .unwrap_or_else(|_| "voice-smoke-test-secret".into()),
        operator_phone_numbers: phones,
    });
    let receipt = channel
        .send(Notification {
            target: OperatorTarget {
                operator_id: "op-1".into(),
                email: "operator@example.com".into(),
            },
            action_id: "voice-smoke-action".into(),
            approval_id: "voice-smoke-action".into(),
            risk_level: RiskLevel::High,
            prompt_body: "Argo voice approval smoke test".into(),
            approve_url: "https://example.com/approve".into(),
            reject_url: "https://example.com/reject".into(),
            expiry: Utc::now() + Duration::minutes(5),
        })
        .await
        .unwrap();
    assert_eq!(receipt.status, argo_channels::DeliveryStatus::Sent);
}
