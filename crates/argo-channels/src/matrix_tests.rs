//! Unit tests for [`super`] (the Matrix approval channel).

use super::*;
use crate::{OperatorTarget, RiskLevel};
use chrono::{Duration, Utc};
use parking_lot::Mutex;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

fn notification() -> Notification {
    Notification {
        target: OperatorTarget {
            operator_id: "op-1".into(),
            email: "operator@example.com".into(),
        },
        action_id: "act-1".into(),
        approval_id: "appr-1".into(),
        risk_level: RiskLevel::High,
        prompt_body: "Approve the Matrix post?".into(),
        approve_url: "https://gw/decision?token=appr-1.approve.999.sig".into(),
        reject_url: "https://gw/decision?token=appr-1.reject.999.sig".into(),
        expiry: Utc::now() + Duration::minutes(5),
    }
}

fn channel(addr: &SocketAddr) -> MatrixChannel {
    let mut cfg = MatrixChannelConfig::disabled_for_tests();
    cfg.homeserver_url = format!("http://{addr}");
    MatrixChannel::new(cfg)
}

type RawRequest = String;
type Hits = Arc<Mutex<Vec<RawRequest>>>;

async fn start_server<F>(handler: F) -> (SocketAddr, Hits)
where
    F: Fn(usize, &str) -> (u16, String) + Send + Sync + 'static,
{
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let hits: Hits = Arc::new(Mutex::new(Vec::new()));
    let hits_clone = Arc::clone(&hits);
    let handler = Arc::new(handler);
    tokio::spawn(async move {
        loop {
            let Ok((mut stream, _)) = listener.accept().await else {
                break;
            };
            let h = Arc::clone(&handler);
            let hits = Arc::clone(&hits_clone);
            tokio::spawn(async move {
                let mut buf = vec![0_u8; 16_384];
                let n = stream.read(&mut buf).await.unwrap_or(0);
                let req = String::from_utf8_lossy(&buf[..n]).to_string();
                let idx = {
                    let mut g = hits.lock();
                    g.push(req.clone());
                    g.len() - 1
                };
                let (status, body) = h(idx, &req);
                let phrase = if status == 200 { "OK" } else { "ERR" };
                let resp = format!(
                    "HTTP/1.1 {status} {phrase}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(resp.as_bytes()).await;
            });
        }
    });
    (addr, hits)
}

fn ok_event(id: &str) -> (u16, String) {
    (200, format!(r#"{{"event_id":"{id}"}}"#))
}

fn put_lines(hits: &Hits) -> Vec<String> {
    hits.lock()
        .iter()
        .filter_map(|r| r.lines().next().map(str::to_string))
        .filter(|l| l.starts_with("PUT "))
        .collect()
}

#[tokio::test]
async fn send_to_default_room_when_no_explicit_target() {
    let (addr, hits) = start_server(|_, _| ok_event("$evt-1")).await;
    let ch = channel(&addr);
    let receipt = ch.send(notification()).await.unwrap();
    assert_eq!(receipt.status, DeliveryStatus::Sent);
    assert_eq!(receipt.provider_message_id.as_deref(), Some("$evt-1"));
    let put = put_lines(&hits).into_iter().next().expect("PUT observed");
    assert!(
        put.contains("/rooms/!default:matrix.example.com/send/m.room.message/"),
        "expected default-room PUT path; got: {put}"
    );
}

#[tokio::test]
async fn send_to_explicit_room_overrides_default() {
    let (addr, hits) = start_server(|_, _| ok_event("$evt-2")).await;
    let ch = channel(&addr);
    let receipt = ch
        .send_to_room(&notification(), Some("!alt:matrix.example.com"), None, &[])
        .await
        .unwrap();
    assert_eq!(receipt.status, DeliveryStatus::Sent);
    let put = put_lines(&hits).into_iter().next().expect("PUT observed");
    assert!(
        put.contains("/rooms/!alt:matrix.example.com/send/m.room.message/"),
        "expected explicit-room PUT path; got: {put}"
    );
}

#[tokio::test]
async fn txn_id_monotonic_across_sends() {
    let (addr, hits) = start_server(|_, _| ok_event("$evt-n")).await;
    let ch = channel(&addr);
    for _ in 0..5 {
        ch.send(notification()).await.unwrap();
    }
    let puts = put_lines(&hits);
    assert_eq!(puts.len(), 5);
    let observed: Vec<u64> = puts
        .iter()
        .map(|p| {
            let tail = p
                .split("/m.room.message/")
                .nth(1)
                .expect("PUT path contains /m.room.message/");
            let id = tail
                .split_whitespace()
                .next()
                .expect("txn_id present before HTTP version");
            id.parse::<u64>().expect("txn_id parses as u64")
        })
        .collect();
    let mut unique = observed.clone();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(
        unique.len(),
        5,
        "expected 5 distinct txn ids; got {observed:?}"
    );
    let mut sorted = observed.clone();
    sorted.sort_unstable();
    assert_eq!(
        observed, sorted,
        "expected strictly increasing txn ids; got {observed:?}"
    );
}

#[tokio::test]
async fn retry_with_same_txn_id_is_idempotent_server_side() {
    let (addr, hits) = start_server(|_, _| ok_event("$evt-same")).await;
    let ch = channel(&addr);
    let n = notification();
    let r1 = ch
        .send_with_txn_id(&n, "!retry:matrix.example.com", 42)
        .await
        .unwrap();
    let r2 = ch
        .send_with_txn_id(&n, "!retry:matrix.example.com", 42)
        .await
        .unwrap();
    assert_eq!(r1.status, DeliveryStatus::Sent);
    assert_eq!(r2.status, DeliveryStatus::Sent);
    let puts = put_lines(&hits);
    assert_eq!(puts.len(), 2, "expected exactly two PUT attempts");
    assert!(
        puts.iter()
            .all(|p| p.contains("/rooms/!retry:matrix.example.com/send/m.room.message/42")),
        "expected both PUTs to share the same txn_id path; got {puts:?}"
    );
}

#[tokio::test]
async fn thread_reply_includes_m_relates_to() {
    let (addr, hits) = start_server(|_, _| ok_event("$evt-thread")).await;
    let ch = channel(&addr);
    ch.send_to_room(
        &notification(),
        None,
        Some("$root-event-id"),
        &[],
    )
    .await
    .unwrap();
    let raw = hits.lock().first().cloned().expect("request observed");
    let body_start = raw.find("\r\n\r\n").map(|i| i + 4).unwrap_or(raw.len());
    let body_json: Value = serde_json::from_str(&raw[body_start..]).expect("body parses as JSON");
    assert_eq!(body_json["m.relates_to"]["rel_type"], "m.thread");
    assert_eq!(body_json["m.relates_to"]["event_id"], "$root-event-id");
}

#[tokio::test]
async fn mention_includes_m_mentions_array() {
    let (addr, hits) = start_server(|_, _| ok_event("$evt-mention")).await;
    let ch = channel(&addr);
    ch.send_to_room(
        &notification(),
        None,
        None,
        &["alice@matrix.org".to_string(), "@bob:matrix.org".to_string()],
    )
    .await
    .unwrap();
    let raw = hits.lock().first().cloned().expect("request observed");
    let body_start = raw.find("\r\n\r\n").map(|i| i + 4).unwrap_or(raw.len());
    let body_json: Value = serde_json::from_str(&raw[body_start..]).expect("body parses as JSON");
    let user_ids = body_json["m.mentions"]["user_ids"]
        .as_array()
        .expect("user_ids array");
    let ids: Vec<&str> = user_ids.iter().filter_map(Value::as_str).collect();
    assert_eq!(ids, vec!["@alice:matrix.org", "@bob:matrix.org"]);
}

#[test]
fn html_formatted_body_renders_markdown() {
    let ch = MatrixChannel::new(MatrixChannelConfig::disabled_for_tests());
    let mut n = notification();
    n.prompt_body = "approve **bold** *italic* `code`".into();
    let payload = ch.message_payload(&n, None, &[]);
    let plain = payload["body"].as_str().expect("plain body");
    let html = payload["formatted_body"].as_str().expect("formatted body");
    assert_eq!(payload["msgtype"], "m.text");
    assert_eq!(payload["format"], "org.matrix.custom.html");
    assert!(
        plain.contains("**bold**"),
        "expected plain body to retain raw markdown; got {plain}"
    );
    assert!(html.contains("<strong>bold</strong>"), "html: {html}");
    assert!(html.contains("<em>italic</em>"), "html: {html}");
    assert!(html.contains("<code>code</code>"), "html: {html}");
}

