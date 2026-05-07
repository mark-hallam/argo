use super::*;
use crate::handlers::worker_actions::ActionRecord;
use argo_identity::NotifyChannelRef;
use chrono::Utc;
use std::sync::{Mutex, OnceLock};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;

#[derive(Default)]
struct StubSender {
    calls: Mutex<Vec<NotifyChannelRef>>,
    fail_for: Vec<String>,
}

impl StubSender {
    fn new() -> Self {
        Self::default()
    }

    fn fail_for(channel: &str) -> Self {
        Self {
            calls: Mutex::new(Vec::new()),
            fail_for: vec![channel.to_string()],
        }
    }

    fn calls(&self) -> Vec<NotifyChannelRef> {
        self.calls.lock().unwrap().clone()
    }
}

#[async_trait::async_trait]
impl NotifySender for StubSender {
    async fn send_via(&self, ref_: &NotifyChannelRef, _n: &Notification) -> Result<(), String> {
        self.calls.lock().unwrap().push(ref_.clone());
        if self.fail_for.iter().any(|f| f == &ref_.channel) {
            Err(format!("stubbed failure for {}", ref_.channel))
        } else {
            Ok(())
        }
    }
}

fn action(action_id: &str, risk: &str) -> ActionRecord {
    ActionRecord {
        schema_key: ActionRecord::SCHEMA_KEY.to_string(),
        action_id: action_id.to_string(),
        job_id: "job-1".to_string(),
        worker_id: argo_identity::WorkerId::nil(),
        item_id: None,
        action_type: "send_buyer_followup_email".to_string(),
        risk_level: risk.to_string(),
        requires_approval: true,
        state: "pending".to_string(),
        payload: serde_json::json!({}),
        approved_by: None,
        approved_at: None,
        executed_at: None,
        platform: None,
        result: None,
        evidence: Vec::new(),
        blocked_reason: None,
        created_at: Utc::now().to_rfc3339(),
    }
}

async fn env_guard() -> tokio::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| tokio::sync::Mutex::new(()))
        .lock()
        .await
}

fn clear_env(vars: &[&str]) {
    for var in vars {
        std::env::remove_var(var);
    }
}

async fn send_env_channel(channel: &str, target: &str) -> Result<(), String> {
    let sender = EnvChannelSender;
    let notification = build_notification(&action("act-env", "high"));
    sender
        .send_via(&NotifyChannelRef::new(channel, target), &notification)
        .await
}

async fn http_mock(
    status: &'static str,
    body: &'static str,
) -> (String, tokio::task::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut bytes = Vec::new();
        let mut buf = [0_u8; 1024];
        loop {
            let n = socket.read(&mut buf).await.unwrap();
            if n == 0 {
                break;
            }
            bytes.extend_from_slice(&buf[..n]);
            if request_complete(&bytes) {
                break;
            }
        }
        let resp = format!(
            "HTTP/1.1 {status}\r\nContent-Length: {}\r\nContent-Type: application/json\r\n\r\n{body}",
            body.len()
        );
        socket.write_all(resp.as_bytes()).await.unwrap();
        String::from_utf8_lossy(&bytes).to_string()
    });
    (format!("http://{addr}"), handle)
}

fn request_complete(bytes: &[u8]) -> bool {
    let Some(head_end) = bytes.windows(4).position(|w| w == b"\r\n\r\n") else {
        return false;
    };
    let head = String::from_utf8_lossy(&bytes[..head_end]);
    let len = head
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().ok())
                .flatten()
        })
        .unwrap_or(0);
    bytes.len() >= head_end + 4 + len
}

async fn smtp_mock() -> (u16, tokio::task::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let handle = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        let (reader, mut writer) = socket.into_split();
        let mut reader = BufReader::new(reader);
        let mut log = String::new();
        writer.write_all(b"220 mock.smtp\r\n").await.unwrap();
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line).await.unwrap() == 0 {
                break;
            }
            log.push_str(&line);
            let upper = line.to_ascii_uppercase();
            if upper.starts_with("EHLO") || upper.starts_with("HELO") {
                writer
                    .write_all(b"250-mock.smtp\r\n250-AUTH PLAIN LOGIN\r\n250 OK\r\n")
                    .await
                    .unwrap();
            } else if upper.starts_with("AUTH PLAIN") {
                writer.write_all(b"235 2.7.0 OK\r\n").await.unwrap();
            } else if upper.starts_with("AUTH LOGIN") {
                writer.write_all(b"334 VXNlcm5hbWU6\r\n").await.unwrap();
                let mut user = String::new();
                reader.read_line(&mut user).await.unwrap();
                log.push_str(&user);
                writer.write_all(b"334 UGFzc3dvcmQ6\r\n").await.unwrap();
                let mut pass = String::new();
                reader.read_line(&mut pass).await.unwrap();
                log.push_str(&pass);
                writer.write_all(b"235 2.7.0 OK\r\n").await.unwrap();
            } else if upper.starts_with("MAIL FROM") || upper.starts_with("RCPT TO") {
                writer.write_all(b"250 2.1.0 OK\r\n").await.unwrap();
            } else if upper.starts_with("DATA") {
                writer
                    .write_all(b"354 End data with <CR><LF>.<CR><LF>\r\n")
                    .await
                    .unwrap();
                loop {
                    let mut data = String::new();
                    reader.read_line(&mut data).await.unwrap();
                    log.push_str(&data);
                    if data.trim_end() == "." {
                        break;
                    }
                }
                writer.write_all(b"250 2.0.0 queued\r\n").await.unwrap();
            } else if upper.starts_with("QUIT") {
                writer.write_all(b"221 2.0.0 bye\r\n").await.unwrap();
                break;
            } else {
                writer.write_all(b"250 OK\r\n").await.unwrap();
            }
        }
        log
    });
    (port, handle)
}

#[tokio::test]
async fn no_channels_is_no_op() {
    let cfg = NotifyConfig::default();
    let sender = StubSender::new();
    let results = dispatch(&cfg, &action("act-1", "high"), &sender).await;
    assert!(results.is_empty());
    assert!(sender.calls().is_empty());
}

#[tokio::test]
async fn single_channel_one_send() {
    let cfg = NotifyConfig {
        channels: vec![NotifyChannelRef::new("push", "operator-iphone")],
    };
    let sender = StubSender::new();
    let results = dispatch(&cfg, &action("act-2", "medium"), &sender).await;
    assert_eq!(results.len(), 1);
    assert!(results[0].success);
    assert_eq!(results[0].channel, "push");
    assert_eq!(sender.calls().len(), 1);
}

#[tokio::test]
async fn multi_channel_fan_out() {
    let cfg = NotifyConfig {
        channels: vec![
            NotifyChannelRef::new("push", "iphone"),
            NotifyChannelRef::new("email", "ops@example.com"),
            NotifyChannelRef::new("slack", "C123"),
        ],
    };
    let sender = StubSender::new();
    let results = dispatch(&cfg, &action("act-3", "low"), &sender).await;
    assert_eq!(results.len(), 3);
    assert!(results.iter().all(|r| r.success));
    let calls = sender.calls();
    assert_eq!(calls.len(), 3);
    assert!(calls.iter().any(|c| c.channel == "push"));
    assert!(calls.iter().any(|c| c.channel == "email"));
    assert!(calls.iter().any(|c| c.channel == "slack"));
}

#[tokio::test]
async fn one_channel_fails_others_still_fire() {
    let cfg = NotifyConfig {
        channels: vec![
            NotifyChannelRef::new("push", "iphone"),
            NotifyChannelRef::new("email", "ops@example.com"),
            NotifyChannelRef::new("slack", "C123"),
        ],
    };
    let sender = StubSender::fail_for("email");
    let results = dispatch(&cfg, &action("act-4", "high"), &sender).await;
    assert_eq!(results.len(), 3);
    assert!(
        results
            .iter()
            .find(|r| r.channel == "push")
            .unwrap()
            .success
    );
    let email_result = results.iter().find(|r| r.channel == "email").unwrap();
    assert!(!email_result.success);
    assert!(email_result.error.is_some());
    assert!(
        results
            .iter()
            .find(|r| r.channel == "slack")
            .unwrap()
            .success
    );
}

#[tokio::test]
async fn risk_class_filter_drops_mismatched_channels() {
    let mut high_only = NotifyChannelRef::new("sms", "+15551234567");
    high_only.risk_level = Some("high".into());
    let mut low_only = NotifyChannelRef::new("email", "ops@example.com");
    low_only.risk_level = Some("low".into());
    let cfg = NotifyConfig {
        channels: vec![
            NotifyChannelRef::new("push", "iphone"),
            high_only.clone(),
            low_only,
        ],
    };
    let sender = StubSender::new();
    let results = dispatch(&cfg, &action("act-5", "high"), &sender).await;
    assert_eq!(results.len(), 2, "low-only filter should drop email");
    assert!(results.iter().any(|r| r.channel == "push"));
    assert!(results.iter().any(|r| r.channel == "sms"));
    assert!(!results.iter().any(|r| r.channel == "email"));
}

#[tokio::test]
async fn risk_filter_is_case_insensitive() {
    let mut ch = NotifyChannelRef::new("push", "iphone");
    ch.risk_level = Some("HIGH".into());
    let cfg = NotifyConfig { channels: vec![ch] };
    let sender = StubSender::new();
    let results = dispatch(&cfg, &action("act-6", "high"), &sender).await;
    assert_eq!(results.len(), 1);
    assert!(results[0].success);
}

#[test]
fn matches_risk_handles_none_filter() {
    let ch = NotifyChannelRef::new("push", "iphone");
    assert!(matches_risk(&ch, "low"));
    assert!(matches_risk(&ch, "medium"));
    assert!(matches_risk(&ch, "high"));
}

#[test]
fn build_notification_carries_action_metadata() {
    let act = action("xyz-789", "medium");
    let n = build_notification(&act);
    assert_eq!(n.action_id, "xyz-789");
    assert_eq!(n.approval_id, "xyz-789");
    assert_eq!(n.risk_level, RiskLevel::Medium);
    assert!(n.prompt_body.contains("xyz-789"));
    assert!(n.prompt_body.contains("send_buyer_followup_email"));
}

#[test]
fn notify_config_load_returns_default_when_missing() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let cfg = NotifyConfig::load(tmp.path());
    assert!(cfg.channels.is_empty());
}

#[test]
fn notify_config_load_parses_channels() {
    let tmp = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(tmp.path().join(".skycot")).unwrap();
    let body = r#"
        [[channels]]
        channel = "push"
        target = "operator-iphone"

        [[channels]]
        channel = "email"
        target = "ops@example.com"
        risk_level = "high"
    "#;
    std::fs::write(tmp.path().join(".skycot/notify.toml"), body).unwrap();
    let cfg = NotifyConfig::load(tmp.path());
    assert_eq!(cfg.channels.len(), 2);
    assert_eq!(cfg.channels[0].channel, "push");
    assert_eq!(cfg.channels[0].target, "operator-iphone");
    assert!(cfg.channels[0].risk_level.is_none());
    assert_eq!(cfg.channels[1].channel, "email");
    assert_eq!(cfg.channels[1].risk_level.as_deref(), Some("high"));
}

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
