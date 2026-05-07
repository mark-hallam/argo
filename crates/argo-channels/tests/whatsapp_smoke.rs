use argo_channels::token::{sign_approval_token, ApprovalDecision};
use argo_channels::whatsapp::{WhatsAppApprovalChannel, WhatsAppApprovalChannelConfig};
use argo_channels::{
    ChannelId, DeliveryStatus, Notification, NotifyChannel, OperatorTarget, RiskLevel,
};
use chrono::{Duration, Utc};
use std::collections::BTreeMap;

#[tokio::test]
async fn twilio_whatsapp_integration_gated_on_env() {
    let from = match std::env::var("TWILIO_WHATSAPP_FROM_NUMBER") {
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
    let to = match std::env::var("TWILIO_WHATSAPP_TO_NUMBER")
        .or_else(|_| std::env::var("TWILIO_TEST_TO"))
    {
        Ok(v) if !v.is_empty() => v,
        _ => return,
    };
    let secret = std::env::var("ARGO_CHANNEL_HMAC_SECRET")
        .unwrap_or_else(|_| "whatsapp-smoke-test-secret".into());
    let exp = Utc::now() + Duration::minutes(5);
    let approve = sign_approval_token(
        "whatsapp-smoke-action",
        ApprovalDecision::Approve,
        exp.timestamp(),
        &secret,
    )
    .unwrap();
    let reject = sign_approval_token(
        "whatsapp-smoke-action",
        ApprovalDecision::Reject,
        exp.timestamp(),
        &secret,
    )
    .unwrap();
    let mut phones = BTreeMap::new();
    phones.insert("op-1".into(), to);
    let channel = WhatsAppApprovalChannel::new(WhatsAppApprovalChannelConfig {
        channel_id: ChannelId::new("whatsapp"),
        endpoint: format!("https://api.twilio.com/2010-04-01/Accounts/{sid}/Messages.json"),
        account_sid: sid,
        auth_token: token,
        from_number: from,
        operator_whatsapp_numbers: phones,
    });
    let receipt = channel
        .send(Notification {
            target: OperatorTarget {
                operator_id: "op-1".into(),
                email: "operator@example.com".into(),
            },
            action_id: "whatsapp-smoke-action".into(),
            approval_id: "whatsapp-smoke-action".into(),
            risk_level: RiskLevel::High,
            prompt_body: "Argo WhatsApp approval smoke test".into(),
            approve_url: format!("https://example.com/approve?token={approve}"),
            reject_url: format!("https://example.com/reject?token={reject}"),
            expiry: exp,
        })
        .await
        .unwrap();
    assert_eq!(receipt.status, DeliveryStatus::Sent);
}
