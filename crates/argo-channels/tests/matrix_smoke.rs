//! Env-gated integration smoke for the Matrix channel.
//!
//! Skipped unless every `ARGO_MATRIX_TEST_*` env var is present. When
//! enabled, performs ONE outbound send against a real Matrix
//! homeserver (matrix.org by default — free, public, federated) and
//! asserts the receipt carries a provider message id (the canonical
//! signal that the homeserver accepted the event).
//!
//! Required env:
//! - `ARGO_MATRIX_TEST_HOMESERVER_URL` — e.g. `https://matrix.org`
//! - `ARGO_MATRIX_TEST_USER_ID` — `@argo-bot:matrix.org`
//! - `ARGO_MATRIX_TEST_ACCESS_TOKEN` — long-lived access token
//! - `ARGO_MATRIX_TEST_ROOM_ID` — `!opaque:matrix.org` (the bot must
//!   already be joined)

use argo_channels::matrix::{MatrixChannel, MatrixChannelConfig};
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
            operator_id: "argo-matrix-smoke".into(),
            email: "smoke@example.com".into(),
        },
        action_id: "argo-matrix-smoke-action".into(),
        approval_id: "argo-matrix-smoke-action".into(),
        risk_level: RiskLevel::High,
        prompt_body: "Argo Matrix smoke test — please ignore.".into(),
        approve_url: "https://gw.example.com/decision?token=argo-matrix-smoke.approve.0.sig"
            .into(),
        reject_url: "https://gw.example.com/decision?token=argo-matrix-smoke.reject.0.sig".into(),
        expiry: Utc::now() + Duration::minutes(5),
    }
}

#[tokio::test]
async fn matrix_send_smoke_when_env_present() {
    let Some(homeserver_url) = env_var("ARGO_MATRIX_TEST_HOMESERVER_URL") else {
        return;
    };
    let Some(user_id) = env_var("ARGO_MATRIX_TEST_USER_ID") else {
        return;
    };
    let Some(access_token) = env_var("ARGO_MATRIX_TEST_ACCESS_TOKEN") else {
        return;
    };
    let Some(room_id) = env_var("ARGO_MATRIX_TEST_ROOM_ID") else {
        return;
    };

    let config = MatrixChannelConfig {
        channel_id: ChannelId::new("matrix"),
        homeserver_url,
        access_token,
        user_id,
        default_room_id: Some(room_id),
    };
    let channel = MatrixChannel::new(config);
    let receipt = channel.send(notification()).await.expect("send");
    assert_eq!(receipt.status, DeliveryStatus::Sent);
    assert!(
        receipt.provider_message_id.is_some(),
        "expected the homeserver to return an event_id; receipt={receipt:?}"
    );
}
