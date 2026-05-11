//! Env-gated integration smoke for the Teams channel.
//!
//! Skipped unless every `ARGO_TEAMS_TEST_*` env var is present. When
//! enabled, this performs ONE outbound send against a real Microsoft
//! Graph tenant and asserts the receipt carries a provider message
//! id (the canonical signal that Graph accepted the message).
//!
//! Required env:
//! - `ARGO_TEAMS_TEST_TENANT_ID` — Azure-AD tenant id
//! - `ARGO_TEAMS_TEST_CLIENT_ID` — Azure-AD app client id
//! - `ARGO_TEAMS_TEST_CLIENT_SECRET` — app client secret
//! - `ARGO_TEAMS_TEST_TEAM_ID` — target team id
//! - `ARGO_TEAMS_TEST_CHANNEL_ID` — target channel id within the team

use argo_channels::teams::{
    TeamsApprovalChannel, TeamsApprovalChannelConfig, DEFAULT_GRAPH_BASE, DEFAULT_LOGIN_BASE,
};
use argo_channels::{
    ChannelId, DeliveryStatus, Notification, NotifyChannel, OperatorTarget, RiskLevel,
};
use chrono::{Duration, Utc};

fn env_var(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.trim().is_empty())
}

fn notification() -> Notification {
    Notification {
        target: OperatorTarget {
            operator_id: "argo-teams-smoke".into(),
            email: "smoke@example.com".into(),
        },
        action_id: "argo-teams-smoke-action".into(),
        approval_id: "argo-teams-smoke-action".into(),
        risk_level: RiskLevel::High,
        prompt_body: "Argo Teams smoke test — please ignore.".into(),
        approve_url: "https://gw.example.com/decision?token=argo-teams-smoke.approve.0.sig".into(),
        reject_url: "https://gw.example.com/decision?token=argo-teams-smoke.reject.0.sig".into(),
        expiry: Utc::now() + Duration::minutes(5),
    }
}

#[tokio::test]
async fn teams_send_smoke_when_env_present() {
    let Some(tenant_id) = env_var("ARGO_TEAMS_TEST_TENANT_ID") else {
        return;
    };
    let Some(client_id) = env_var("ARGO_TEAMS_TEST_CLIENT_ID") else {
        return;
    };
    let Some(client_secret) = env_var("ARGO_TEAMS_TEST_CLIENT_SECRET") else {
        return;
    };
    let Some(team_id) = env_var("ARGO_TEAMS_TEST_TEAM_ID") else {
        return;
    };
    let Some(channel_id) = env_var("ARGO_TEAMS_TEST_CHANNEL_ID") else {
        return;
    };

    let config = TeamsApprovalChannelConfig {
        channel_id: ChannelId::new("teams"),
        tenant_id,
        client_id,
        client_secret,
        default_team_id: Some(team_id),
        default_channel_id: Some(channel_id),
        graph_base: DEFAULT_GRAPH_BASE.into(),
        login_base: DEFAULT_LOGIN_BASE.into(),
        markdown_to_html: true,
        quiet_hours: None,
    };
    let channel = TeamsApprovalChannel::new(config);
    let receipt = channel.send(notification()).await.expect("send");
    assert_eq!(receipt.status, DeliveryStatus::Sent);
    assert!(
        receipt.provider_message_id.is_some(),
        "expected Graph to return a message id; receipt={receipt:?}"
    );
}
