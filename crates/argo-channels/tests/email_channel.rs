use argo_channels::email::{EmailChannel, EmailChannelConfig};
use argo_channels::token::{
    sign_approval_token, verify_approval_token, verify_webhook_signature, ApprovalDecision,
};
use argo_channels::{
    ChannelId, Notification, NotifyChannel, OperatorTarget, RateLimitConfig, RiskLevel,
};
use chrono::{Duration, Utc};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[tokio::test]
async fn email_send_posts_to_fake_postmark() {
    let (endpoint, handle) = fake_postmark().await;
    let channel = EmailChannel::new(EmailChannelConfig {
        channel_id: ChannelId::new("email"),
        postmark_token: "postmark-test".into(),
        from_address: "worker@example.com".into(),
        subject_template: "[{{risk_level}}] {{action_id}}".into(),
        endpoint,
        message_stream: None,
        quiet_hours: Default::default(),
        rate_limit: RateLimitConfig::unlimited(),
    });
    let receipt = channel
        .send(fake_notification("act-1"))
        .await
        .expect("send");
    assert_eq!(receipt.provider_message_id.as_deref(), Some("fake-message"));
    let request = handle.await.expect("server");
    assert!(request
        .to_lowercase()
        .contains("x-postmark-server-token: postmark-test"));
    assert!(request.contains("Approve"));
    assert!(request.contains("https://example.test/approve?token="));
}

#[test]
fn fake_webhook_reply_token_verifies() {
    let exp = Utc::now().timestamp() + 60;
    let token =
        sign_approval_token("act-2", ApprovalDecision::Reject, exp, "secret").expect("token");
    let body = serde_json::json!({
        "From": "operator@example.com",
        "TextBody": format!("rejecting\napproval_token={token}"),
    })
    .to_string();
    let sig = sign_body(body.as_bytes(), "secret");
    verify_webhook_signature(body.as_bytes(), &sig, "secret").expect("webhook sig");
    let claims = verify_approval_token(&token, "secret").expect("claims");
    assert_eq!(claims.approval_id, "act-2");
    assert_eq!(claims.decision, ApprovalDecision::Reject);
}

async fn fake_postmark() -> (String, tokio::task::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    let handle = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("accept");
        let mut buf = vec![0_u8; 4096];
        let n = stream.read(&mut buf).await.expect("read");
        let request = String::from_utf8_lossy(&buf[..n]).to_string();
        let body = r#"{"ErrorCode":0,"Message":"OK","MessageID":"fake-message"}"#;
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        stream.write_all(response.as_bytes()).await.expect("write");
        request
    });
    (format!("http://{addr}/email"), handle)
}

fn fake_notification(id: &str) -> Notification {
    let exp = Utc::now() + Duration::minutes(15);
    Notification {
        target: OperatorTarget {
            operator_id: "op".into(),
            email: "operator@example.com".into(),
        },
        action_id: id.into(),
        approval_id: id.into(),
        risk_level: RiskLevel::Medium,
        prompt_body: "Approve posting this listing?".into(),
        approve_url: "https://example.test/approve?token=approve-token".into(),
        reject_url: "https://example.test/reject?token=reject-token".into(),
        expiry: exp,
    }
}

fn sign_body(body: &[u8], secret: &str) -> String {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("hmac");
    mac.update(body);
    hex::encode(mac.finalize().into_bytes())
}
