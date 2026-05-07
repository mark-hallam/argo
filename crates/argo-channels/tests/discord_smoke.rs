use argo_channels::discord::{DiscordChannel, DiscordChannelConfig};
use argo_channels::{
    ChannelId, DeliveryStatus, Notification, NotifyChannel, OperatorTarget, RiskLevel,
};
use chrono::{Duration, Utc};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[tokio::test]
async fn discord_send_posts_to_mock_when_env_present() {
    let Ok(bot_token) = std::env::var("DISCORD_BOT_TOKEN") else {
        return;
    };
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut buf = [0_u8; 4096];
        let n = stream.read(&mut buf).await.unwrap();
        let req = String::from_utf8_lossy(&buf[..n]);
        assert!(req.contains("POST /channels/C123/messages HTTP/1.1"));
        assert!(req.to_ascii_lowercase().contains("authorization: bot "));
        assert!(req.contains("argo_approval:discord-smoke-action.approve.9999999999.sig"));
        let body = r#"{"id":"discord-smoke-message"}"#;
        let resp = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        stream.write_all(resp.as_bytes()).await.unwrap();
    });
    let channel = DiscordChannel::new(DiscordChannelConfig {
        channel_id: ChannelId::new("discord"),
        bot_token,
        discord_channel_id: "C123".into(),
        endpoint_base: format!("http://{addr}"),
    });
    let receipt = channel.send(notification()).await.unwrap();
    assert_eq!(receipt.status, DeliveryStatus::Sent);
    assert_eq!(
        receipt.provider_message_id.as_deref(),
        Some("discord-smoke-message")
    );
    server.await.unwrap();
}

fn notification() -> Notification {
    Notification {
        target: OperatorTarget {
            operator_id: "op-1".into(),
            email: "operator@example.com".into(),
        },
        action_id: "discord-smoke-action".into(),
        approval_id: "discord-smoke-action".into(),
        risk_level: RiskLevel::High,
        prompt_body: "Discord approval smoke test".into(),
        approve_url:
            "https://gw.example.com/decision?token=discord-smoke-action.approve.9999999999.sig"
                .into(),
        reject_url:
            "https://gw.example.com/decision?token=discord-smoke-action.reject.9999999999.sig"
                .into(),
        expiry: Utc::now() + Duration::minutes(5),
    }
}
