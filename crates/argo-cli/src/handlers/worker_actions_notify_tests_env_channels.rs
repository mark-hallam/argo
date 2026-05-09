//! Per-channel `EnvChannelSender` integration tests (slack, email, sms,
//! discord, chat, sse). Network-mock and env-fixture helpers live in
//! the sibling `env_mocks` module.

use super::env_mocks::{clear_env, env_guard, http_mock, send_env_channel, smtp_mock};

#[tokio::test]
async fn slack_missing_env_names_var() {
    let _guard = env_guard().await;
    clear_env(&["SLACK_WEBHOOK_URL"]);
    let err = send_env_channel("slack", "#ops").await.unwrap_err();
    assert!(err.contains("slack not configured"));
    assert!(err.contains("SLACK_WEBHOOK_URL"));
}

#[tokio::test]
async fn slack_posts_to_webhook() {
    let _guard = env_guard().await;
    let (url, handle) = http_mock("200 OK", "{}").await;
    std::env::set_var("SLACK_WEBHOOK_URL", format!("{url}/slack"));
    send_env_channel("slack", "#ops").await.unwrap();
    let req = handle.await.unwrap();
    assert!(req.starts_with("POST /slack "));
    assert!(req.contains("Approval needed"));
    assert!(req.contains("act-env"));
}

#[tokio::test]
async fn email_missing_env_names_var() {
    let _guard = env_guard().await;
    clear_env(&[
        "SMTP_HOST",
        "SMTP_PORT",
        "SMTP_USER",
        "SMTP_PASS",
        "SMTP_FROM",
    ]);
    let err = send_env_channel("email", "ops@example.com")
        .await
        .unwrap_err();
    assert!(err.contains("email not configured"));
    assert!(err.contains("SMTP_HOST"));
}

#[tokio::test]
async fn email_sends_via_smtp() {
    let _guard = env_guard().await;
    let (port, handle) = smtp_mock().await;
    std::env::set_var("SMTP_HOST", "127.0.0.1");
    std::env::set_var("SMTP_PORT", port.to_string());
    std::env::set_var("SMTP_USER", "user");
    std::env::set_var("SMTP_PASS", "pass");
    std::env::set_var("SMTP_FROM", "worker@example.com");
    send_env_channel("email", "ops@example.com").await.unwrap();
    let smtp = handle.await.unwrap();
    assert!(smtp.contains("MAIL FROM:<worker@example.com>"));
    assert!(smtp.contains("RCPT TO:<ops@example.com>"));
    assert!(smtp.contains("Approval needed"));
}

#[tokio::test]
async fn sms_missing_env_names_var() {
    let _guard = env_guard().await;
    clear_env(&[
        "TWILIO_ACCOUNT_SID",
        "TWILIO_AUTH_TOKEN",
        "TWILIO_FROM",
        "TWILIO_API_BASE_URL",
    ]);
    let err = send_env_channel("sms", "+15551234567").await.unwrap_err();
    assert!(err.contains("sms not configured"));
    assert!(err.contains("TWILIO_ACCOUNT_SID"));
}

#[tokio::test]
async fn sms_posts_to_twilio() {
    let _guard = env_guard().await;
    let (url, handle) = http_mock("201 Created", r#"{"sid":"SM123"}"#).await;
    std::env::set_var("TWILIO_ACCOUNT_SID", "AC123");
    std::env::set_var("TWILIO_AUTH_TOKEN", "token");
    std::env::set_var("TWILIO_FROM", "+15550000000");
    std::env::set_var("TWILIO_API_BASE_URL", url);
    send_env_channel("sms", "+15551234567").await.unwrap();
    let req = handle.await.unwrap();
    assert!(req.starts_with("POST /2010-04-01/Accounts/AC123/Messages.json "));
    assert!(req.contains("authorization: Basic") || req.contains("Authorization: Basic"));
    assert!(req.contains("To=%2B15551234567"));
    assert!(req.contains("Body="));
}

#[tokio::test]
async fn discord_missing_env_names_var() {
    let _guard = env_guard().await;
    clear_env(&["DISCORD_WEBHOOK_URL"]);
    let err = send_env_channel("discord", "ops").await.unwrap_err();
    assert!(err.contains("discord not configured"));
    assert!(err.contains("DISCORD_WEBHOOK_URL"));
}

#[tokio::test]
async fn discord_posts_to_webhook() {
    let _guard = env_guard().await;
    let (url, handle) = http_mock("204 No Content", "").await;
    std::env::set_var("DISCORD_WEBHOOK_URL", format!("{url}/discord"));
    send_env_channel("discord", "ops").await.unwrap();
    let req = handle.await.unwrap();
    assert!(req.starts_with("POST /discord "));
    assert!(req.contains("Approval needed"));
    assert!(req.contains("act-env"));
}

#[tokio::test]
async fn chat_missing_env_names_var() {
    let _guard = env_guard().await;
    clear_env(&["CHAT_WEBHOOK_URL"]);
    let err = send_env_channel("chat", "ops").await.unwrap_err();
    assert!(err.contains("chat not configured"));
    assert!(err.contains("CHAT_WEBHOOK_URL"));
}

#[tokio::test]
async fn chat_posts_generic_payload() {
    let _guard = env_guard().await;
    let (url, handle) = http_mock("200 OK", "{}").await;
    std::env::set_var("CHAT_WEBHOOK_URL", format!("{url}/chat"));
    send_env_channel("chat", "ops").await.unwrap();
    let req = handle.await.unwrap();
    assert!(req.starts_with("POST /chat "));
    assert!(req.contains("\"type\":\"notify_ask\""));
    assert!(req.contains("\"channel\":\"chat\""));
}

#[tokio::test]
async fn sse_missing_env_names_var() {
    let _guard = env_guard().await;
    clear_env(&["SSE_NOTIFY_URL"]);
    let err = send_env_channel("sse", "ops").await.unwrap_err();
    assert!(err.contains("sse not configured"));
    assert!(err.contains("SSE_NOTIFY_URL"));
}

#[tokio::test]
async fn sse_posts_notify_ask_event() {
    let _guard = env_guard().await;
    let (url, handle) = http_mock("200 OK", "{}").await;
    std::env::set_var("SSE_NOTIFY_URL", format!("{url}/sse"));
    send_env_channel("sse", "ops").await.unwrap();
    let req = handle.await.unwrap();
    assert!(req.starts_with("POST /sse "));
    assert!(req.contains("\"event\":\"notify_ask\""));
    assert!(req.contains("\"channel\":\"sse\""));
}
