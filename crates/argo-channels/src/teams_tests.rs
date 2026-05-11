//! Unit tests for [`super`] (the Teams approval channel).

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
        prompt_body: "Approve the Teams post?".into(),
        approve_url: "https://gw/decision?token=appr-1.approve.999.sig".into(),
        reject_url: "https://gw/decision?token=appr-1.reject.999.sig".into(),
        expiry: Utc::now() + Duration::minutes(5),
    }
}

fn channel(addr: &SocketAddr) -> TeamsApprovalChannel {
    let mut cfg = TeamsApprovalChannelConfig::disabled_for_tests();
    cfg.graph_base = format!("http://{addr}");
    cfg.login_base = format!("http://{addr}");
    TeamsApprovalChannel::new(cfg)
}

type RawRequest = String;
type Hits = Arc<Mutex<Vec<RawRequest>>>;
type Handler = dyn Fn(usize, &str) -> (u16, String) + Send + Sync;

/// Start an HTTP mock that dispatches each accepted request through
/// `handler(idx, raw_request) -> (status, body)`. Returns the bound
/// address and a shared vec of raw requests for post-hoc assertion.
async fn start_server<F>(handler: F) -> (SocketAddr, Hits)
where
    F: Fn(usize, &str) -> (u16, String) + Send + Sync + 'static,
{
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let hits: Hits = Arc::new(Mutex::new(Vec::new()));
    let hits_clone = Arc::clone(&hits);
    let handler: Arc<Handler> = Arc::new(handler);
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
                let phrase = match status {
                    200 => "OK",
                    401 => "Unauthorized",
                    _ => "ERR",
                };
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

fn token_body() -> String {
    r#"{"access_token":"tok-1","expires_in":3600,"token_type":"Bearer"}"#.to_string()
}

fn count_containing(hits: &Hits, needle: &str) -> usize {
    hits.lock().iter().filter(|r| r.contains(needle)).count()
}

#[tokio::test]
async fn send_to_default_channel_when_no_explicit_team() {
    let (addr, hits) = start_server(|_idx, req| {
        if req.contains("/oauth2/v2.0/token") {
            (200, token_body())
        } else {
            (200, r#"{"id":"msg-default"}"#.to_string())
        }
    })
    .await;
    let ch = channel(&addr);
    let receipt = ch.send(notification()).await.unwrap();
    assert_eq!(receipt.status, DeliveryStatus::Sent);
    let h = hits.lock();
    let send = h
        .iter()
        .find(|r| r.contains("POST /teams/"))
        .expect("send request observed");
    assert!(
        send.contains("/teams/T1/channels/C1/messages"),
        "expected default team/channel route; got: {}",
        send.lines().next().unwrap_or("")
    );
}

#[tokio::test]
async fn send_to_explicit_team_channel() {
    let (addr, hits) = start_server(|_idx, req| {
        if req.contains("/oauth2/v2.0/token") {
            (200, token_body())
        } else {
            (200, r#"{"id":"msg-explicit"}"#.to_string())
        }
    })
    .await;
    let ch = channel(&addr);
    let receipt = ch
        .send_to_target(&notification(), Some("Tx"), Some("Cx"), &[])
        .await
        .unwrap();
    assert_eq!(receipt.status, DeliveryStatus::Sent);
    let h = hits.lock();
    let send = h
        .iter()
        .find(|r| r.contains("POST /teams/"))
        .expect("send request observed");
    assert!(
        send.contains("/teams/Tx/channels/Cx/messages"),
        "expected explicit team/channel route; got: {}",
        send.lines().next().unwrap_or("")
    );
}

#[tokio::test]
async fn reply_to_thread_uses_chats_endpoint() {
    let (addr, hits) = start_server(|_idx, req| {
        if req.contains("/oauth2/v2.0/token") {
            (200, token_body())
        } else {
            (200, r#"{"id":"reply-1"}"#.to_string())
        }
    })
    .await;
    let ch = channel(&addr);
    let receipt = ch
        .reply_to_thread(&notification(), "Tt", "Cc", "thread-7", &[])
        .await
        .unwrap();
    assert_eq!(receipt.status, DeliveryStatus::Sent);
    let h = hits.lock();
    let reply = h
        .iter()
        .find(|r| r.contains("/replies"))
        .expect("thread-reply request observed");
    assert!(
        reply.contains("/teams/Tt/channels/Cc/messages/thread-7/replies"),
        "expected reply path; got: {}",
        reply.lines().next().unwrap_or("")
    );
}

#[tokio::test]
async fn mention_resolution_cached_second_call_no_hit() {
    let (addr, hits) = start_server(|_idx, req| {
        if req.contains("/oauth2/v2.0/token") {
            (200, token_body())
        } else {
            (
                200,
                r#"{"id":"aad-9","userPrincipalName":"a@b","displayName":"A B"}"#.to_string(),
            )
        }
    })
    .await;
    let ch = channel(&addr);
    let first = ch.resolve_mention("a@b").await.unwrap();
    let second = ch.resolve_mention("a@b").await.unwrap();
    assert_eq!(first, "aad-9");
    assert_eq!(second, "aad-9");
    let user_hits = count_containing(&hits, "GET /users/a@b");
    assert_eq!(
        user_hits, 1,
        "expected exactly one /users/a@b lookup; observed {user_hits}"
    );
}

#[tokio::test]
async fn token_refresh_on_401_then_retries_once() {
    let (addr, hits) = start_server(|idx, req| {
        if req.contains("/oauth2/v2.0/token") {
            (200, token_body())
        } else if idx == 1 {
            (401, "{}".to_string())
        } else {
            (200, r#"{"id":"msg-after-401"}"#.to_string())
        }
    })
    .await;
    let ch = channel(&addr);
    let receipt = ch.send(notification()).await.unwrap();
    assert_eq!(receipt.status, DeliveryStatus::Sent);
    assert_eq!(receipt.provider_message_id.as_deref(), Some("msg-after-401"));
    let token_hits = count_containing(&hits, "/oauth2/v2.0/token");
    let send_hits = count_containing(&hits, "POST /teams/");
    assert!(
        token_hits >= 2,
        "expected token endpoint hit at least twice (initial + post-401 refresh); got {token_hits}"
    );
    assert_eq!(
        send_hits, 2,
        "expected exactly two send attempts (initial 401 + one retry); got {send_hits}"
    );
}

#[tokio::test]
async fn token_refresh_on_5min_before_expiry_proactively() {
    let (addr, hits) = start_server(|_idx, req| {
        if req.contains("/oauth2/v2.0/token") {
            (200, token_body())
        } else {
            (200, r#"{"id":"msg-proactive"}"#.to_string())
        }
    })
    .await;
    let ch = channel(&addr);
    ch.seed_token("near-expiry", Utc::now() + Duration::minutes(1));
    let receipt = ch.send(notification()).await.unwrap();
    assert_eq!(receipt.status, DeliveryStatus::Sent);
    let token_hits = count_containing(&hits, "/oauth2/v2.0/token");
    assert_eq!(
        token_hits, 1,
        "expected proactive token refresh inside the 5-minute window; got {token_hits}"
    );
}

#[test]
fn attachment_becomes_adaptive_card() {
    let ch = TeamsApprovalChannel::new(TeamsApprovalChannelConfig::disabled_for_tests());
    let attachments = vec![TeamsAttachment {
        title: "Refund summary".into(),
        body: "Customer requested a $42 refund.".into(),
    }];
    let payload = ch.message_payload(&notification(), &attachments).unwrap();
    let atts = payload["attachments"]
        .as_array()
        .expect("attachments array");
    assert_eq!(atts.len(), 1);
    assert_eq!(
        atts[0]["contentType"], "application/vnd.microsoft.card.adaptive",
        "must use adaptive-card content-type"
    );
    let content = atts[0]["content"].as_str().expect("content is JSON string");
    assert!(
        content.contains("\"version\":\"1.5\""),
        "expected adaptive-card v1.5; content was: {content}"
    );
    assert!(content.contains("Refund summary"));
    assert!(content.contains("Customer requested a $42 refund."));
}

#[test]
fn message_payload_renders_inline_markdown_in_body() {
    let cfg = TeamsApprovalChannelConfig::disabled_for_tests();
    let ch = TeamsApprovalChannel::new(cfg);
    let mut n = notification();
    n.prompt_body = "approve **bold** *italic* `code` [link](https://example.com)".into();
    let payload = ch.message_payload(&n, &[]).unwrap();
    let body = payload["body"]["content"].as_str().unwrap();
    assert!(body.contains("<strong>bold</strong>"), "body: {body}");
    assert!(body.contains("<em>italic</em>"), "body: {body}");
    assert!(body.contains("<code>code</code>"), "body: {body}");
    assert!(
        body.contains("<a href=\"https://example.com\">link</a>"),
        "body: {body}"
    );
}
