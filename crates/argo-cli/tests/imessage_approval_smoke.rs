//! iMessage approval outbound smoke for the v1.0 sales-promise gate
//! (`streams/mission-imessage-wire-pro-tier-outbound-dispatch.md`).
//!
//! Two integration scenarios, modelled on `slack_approval_smoke.rs`:
//!  * `imessage_outbound_post_carries_message_with_approval_links` —
//!    real `IMessageChannel` POSTs to a Blue Bubbles stub URL with
//!    the chat GUID from the channel ref and the rendered approval
//!    message body (approve + reject URLs + approval id).
//!  * `imessage_env_config_loads_from_blue_bubbles_vars` —
//!    `IMessageChannelConfig::from_env_default()` reads
//!    `BLUE_BUBBLES_URL` + `BLUE_BUBBLES_PASSWORD` +
//!    `ARGO_CHANNEL_HMAC_SECRET`, building a config that the
//!    dispatcher arm at `worker_actions_notify_imessage::send_imessage`
//!    consumes verbatim.
//!
//! The pure-library seam (`IMessageChannel::parse_webhook` +
//! `verify_approval_token`) is already covered by the in-source tests
//! in `crates/argo-channels/src/imessage.rs`, so this file adds only
//! the env-driven outbound surface that ties the dispatcher arm to
//! the operator's Blue Bubbles credentials.
//!
//! `argo-cli` is a binary crate (no `lib.rs`), so this test cannot
//! import `EnvChannelSender` directly. The wire-up the brief calls
//! for is validated end-to-end inside the crate by the new
//! `imessage_posts_to_bluebubbles` test in
//! `worker_actions_notify_tests_env_channels.rs`, which drives
//! `EnvChannelSender::send_via("imessage", ...)` against the same
//! Blue Bubbles mock pattern.

use argo_channels::imessage::{IMessageChannel, IMessageChannelConfig};
use argo_channels::{ChannelId, Notification, NotifyChannel, OperatorTarget, RiskLevel};
use chrono::{Duration as ChDuration, Utc};
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{mpsc, Mutex, OnceLock};
use std::thread;
use std::time::Duration;

const ACTION: &str = "act-imessage-smoke";
const CHAT_GUID: &str = "iMessage;-;chat-smoke";
const HMAC: &str = "smoke-argo-channel-hmac-v1";

/// Serialise the env-mutating tests in this file so the `BLUE_BUBBLES_*`
/// vars one test sets don't leak into the next test's read.
fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[tokio::test]
async fn imessage_outbound_post_carries_message_with_approval_links() {
    let (stub_port, stub_rx) = start_bluebubbles_stub();
    let endpoint = format!("http://127.0.0.1:{stub_port}");
    let channel = build_channel(&endpoint);
    let receipt = channel
        .send(build_notification())
        .await
        .expect("iMessage send against stub");
    assert_eq!(receipt.channel_id.to_string(), "imessage");
    assert_eq!(receipt.provider_message_id.as_deref(), Some("msg-smoke-1"));

    let raw = stub_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("stub captured POST");
    assert!(
        raw.starts_with("POST /api/v1/message/text?password=smoke-password"),
        "Blue Bubbles message endpoint missing in request: {raw}"
    );
    let body = http_body(&raw);
    let payload: serde_json::Value =
        serde_json::from_str(body).expect("Blue Bubbles body is JSON");
    // Blue Bubbles serializes chat_guid as `chatGuid` and temp_guid as
    // `tempGuid` (see SendMessageRequest in argo-bluebubbles-client).
    assert_eq!(payload["chatGuid"], CHAT_GUID);
    assert_eq!(payload["tempGuid"], ACTION);
    let message = payload["message"].as_str().expect("message string");
    assert!(message.contains("approval requested"));
    assert!(message.contains(ACTION));
    assert!(message.contains("Approve: https://gw/approve"));
    assert!(message.contains("Reject: https://gw/reject"));
}

#[test]
fn imessage_env_config_loads_from_blue_bubbles_vars() {
    let _lock = env_lock();
    std::env::set_var("BLUE_BUBBLES_URL", "http://127.0.0.1:9");
    std::env::set_var("BLUE_BUBBLES_PASSWORD", "env-password");
    std::env::set_var("ARGO_CHANNEL_HMAC_SECRET", HMAC);
    let cfg = IMessageChannelConfig::from_env_default().expect("config from env");
    assert_eq!(cfg.server_url, "http://127.0.0.1:9");
    assert_eq!(cfg.password, "env-password");
    assert_eq!(cfg.approval_secret, HMAC);
    assert_eq!(cfg.channel_id.to_string(), "imessage");
    std::env::remove_var("BLUE_BUBBLES_URL");
    std::env::remove_var("BLUE_BUBBLES_PASSWORD");
    std::env::remove_var("ARGO_CHANNEL_HMAC_SECRET");
}

fn build_channel(endpoint: &str) -> IMessageChannel {
    let mut routes = BTreeMap::new();
    routes.insert("op-smoke".into(), CHAT_GUID.into());
    let cfg = IMessageChannelConfig {
        channel_id: ChannelId::new("imessage"),
        server_url: endpoint.into(),
        password: "smoke-password".into(),
        approval_secret: HMAC.into(),
        webhook_secret: Some("smoke-webhook".into()),
        operator_routes: routes,
        rate_limit: argo_channels::RateLimitConfig::unlimited(),
        quiet_hours: None,
    };
    IMessageChannel::new(cfg).expect("imessage channel")
}

fn build_notification() -> Notification {
    Notification {
        target: OperatorTarget {
            operator_id: "op-smoke".into(),
            email: "op-smoke@example.com".into(),
        },
        action_id: ACTION.into(),
        approval_id: ACTION.into(),
        risk_level: RiskLevel::High,
        prompt_body: "Approve the iMessage smoke action?".into(),
        approve_url: "https://gw/approve?token=a".into(),
        reject_url: "https://gw/reject?token=r".into(),
        expiry: Utc::now() + ChDuration::minutes(10),
    }
}

fn start_bluebubbles_stub() -> (u16, mpsc::Receiver<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind stub");
    let port = listener.local_addr().expect("local_addr").port();
    let (tx, rx) = mpsc::sync_channel::<String>(1);
    thread::spawn(move || {
        let Ok((mut stream, _)) = listener.accept() else {
            return;
        };
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .ok();
        let mut buf = Vec::with_capacity(4096);
        let mut tmp = [0_u8; 4096];
        let mut hend = 0_usize;
        let mut clen = 0_usize;
        loop {
            let Ok(n) = stream.read(&mut tmp) else { return };
            if n == 0 {
                break;
            }
            buf.extend_from_slice(&tmp[..n]);
            if hend == 0 {
                if let Some(p) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                    hend = p + 4;
                    for line in std::str::from_utf8(&buf[..p]).unwrap_or("").split("\r\n") {
                        let lower = line.to_ascii_lowercase();
                        if let Some(v) = lower.strip_prefix("content-length:") {
                            clen = v.trim().parse().unwrap_or(0);
                        }
                    }
                }
            }
            if hend != 0 && buf.len() >= hend + clen {
                break;
            }
        }
        let raw = String::from_utf8_lossy(&buf).to_string();
        let _ = tx.send(raw);
        let resp_body = br#"{"data":{"guid":"msg-smoke-1"}}"#;
        let _ = stream.write_all(
            format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                resp_body.len()
            )
            .as_bytes(),
        );
        let _ = stream.write_all(resp_body);
    });
    (port, rx)
}

fn http_body(raw: &str) -> &str {
    raw.find("\r\n\r\n").map_or("", |p| &raw[p + 4..])
}
