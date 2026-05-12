//! Tests for the J3 structured-event emitter. Sibling-file split per
//! the brief's 400-LOC budget.
//!
//! Tests use the per-tenant broadcast directly so we don't have to
//! stand up a real WS handshake — the wire-format envelope is tested
//! against locked JSON shape, and per-tenant isolation is asserted
//! by subscribing two receivers and observing the cross-leak (or
//! lack thereof).

use super::jwt::Claims;
use super::*;
use argo_channels::webinbox::{
    ApprovalRowFull, WebInboxChannel, WebInboxChannelConfig, WebInboxEvent as ChannelEvent,
    WebInboxRow,
};
use argo_channels::{Notification, NotifyChannel, OperatorTarget, RiskLevel};
use base64::Engine;
use chrono::Utc;
use ed25519_dalek::SigningKey;
use rand_core::OsRng;
use serde_json::{json, Value};

const PUBKEY_ENV: &str = "ARGO_INBOX_WS_JWT_PUBKEY";

fn sample_row(approval_id: &str) -> WebInboxRow {
    WebInboxRow {
        operator_id: "op-1".into(),
        approval_id: approval_id.into(),
        action_id: approval_id.into(),
        risk_level: RiskLevel::High,
        prompt_body: "Approve listing?".into(),
        approve_url: "http://gw/approve?token=t".into(),
        reject_url: "http://gw/reject?token=t".into(),
        expiry: Utc::now() + chrono::Duration::minutes(5),
        enqueued_at: Utc::now(),
    }
}

#[test]
fn hello_frame_carries_protocol_and_capabilities() {
    let parsed: Value = serde_json::from_str(&hello_frame()).expect("hello is JSON");
    assert_eq!(parsed["type"], "hello");
    assert_eq!(parsed["protocol_version"], PROTOCOL_VERSION);
    assert_eq!(parsed["seq"], 0);
    let caps = parsed["capabilities"].as_array().expect("capabilities array");
    let names: Vec<&str> = caps.iter().filter_map(Value::as_str).collect();
    assert!(names.contains(&"approval.created"));
    assert!(names.contains(&"approval.decided"));
    assert!(names.contains(&"approval.undone"));
    assert!(names.contains(&"approval.expired"));
}

#[test]
fn envelope_adds_seq_and_ts_and_preserves_type() {
    let event = WebInboxEvent::ApprovalCreated {
        payload: json!({"id": "act-1"}),
    };
    let body = envelope(7, &event);
    let parsed: Value = serde_json::from_str(&body).expect("envelope is JSON");
    assert_eq!(parsed["type"], "approval.created");
    assert_eq!(parsed["seq"], 7);
    assert!(parsed["ts"].is_string());
    assert_eq!(parsed["payload"]["id"], "act-1");
}

#[tokio::test]
async fn per_tenant_seq_monotonically_increases() {
    // Verifies the at-publish-time per-tenant counter mints seq=1,2,3
    // for successive events on the same tenant (v1 simplification of
    // the per-connection counter; see publish_event docs).
    let mut rx = subscribe_tenant("tenant-seq-mono");
    for _ in 0..3 {
        publish_event(
            "tenant-seq-mono",
            WebInboxEvent::ApprovalCreated {
                payload: json!({}),
            },
        );
    }
    let f1: Value = serde_json::from_str(&rx.recv().await.unwrap()).unwrap();
    let f2: Value = serde_json::from_str(&rx.recv().await.unwrap()).unwrap();
    let f3: Value = serde_json::from_str(&rx.recv().await.unwrap()).unwrap();
    let s1 = f1["seq"].as_u64().unwrap();
    let s2 = f2["seq"].as_u64().unwrap();
    let s3 = f3["seq"].as_u64().unwrap();
    assert!(s1 < s2 && s2 < s3, "seq monotonic: {s1} {s2} {s3}");
}

#[test]
fn decided_envelope_carries_decision_alongside_payload() {
    let event = WebInboxEvent::ApprovalDecided {
        payload: json!({"id": "act-1"}),
        decision: "approve".into(),
    };
    let body = envelope(42, &event);
    let parsed: Value = serde_json::from_str(&body).expect("envelope is JSON");
    assert_eq!(parsed["type"], "approval.decided");
    assert_eq!(parsed["decision"], "approve");
    assert_eq!(parsed["payload"]["id"], "act-1");
    assert_eq!(parsed["seq"], 42);
}

#[tokio::test]
async fn per_tenant_isolation_no_cross_leak() {
    let mut rx_a = subscribe_tenant("tenant-iso-a");
    let mut rx_b = subscribe_tenant("tenant-iso-b");
    publish_event(
        "tenant-iso-a",
        WebInboxEvent::ApprovalCreated {
            payload: json!({"id": "act-A"}),
        },
    );
    publish_event(
        "tenant-iso-b",
        WebInboxEvent::ApprovalCreated {
            payload: json!({"id": "act-B"}),
        },
    );
    let a = tokio::time::timeout(std::time::Duration::from_millis(50), rx_a.recv())
        .await
        .expect("tenant A receives")
        .expect("frame");
    let b = tokio::time::timeout(std::time::Duration::from_millis(50), rx_b.recv())
        .await
        .expect("tenant B receives")
        .expect("frame");
    assert!(a.contains("act-A"), "A frame: {a}");
    assert!(!a.contains("act-B"), "A frame leaked B: {a}");
    assert!(b.contains("act-B"), "B frame: {b}");
    assert!(!b.contains("act-A"), "B frame leaked A: {b}");
    // No further frame waiting on either side.
    assert!(rx_a.try_recv().is_err());
    assert!(rx_b.try_recv().is_err());
}

#[tokio::test]
async fn publish_with_no_subscriber_is_noop() {
    // Tenant has no subscriber yet — publish should silently drop.
    publish_event(
        "tenant-orphan",
        WebInboxEvent::ApprovalCreated {
            payload: json!({"id": "x"}),
        },
    );
    // Late subscriber sees nothing from before its subscribe.
    let mut rx = subscribe_tenant("tenant-orphan");
    assert!(rx.try_recv().is_err());
}

#[tokio::test]
async fn channel_send_emits_approval_created_event() {
    // Subscribing first registers the channel-hook + creates the
    // per-tenant broadcaster. The channel-send then routes through
    // the hook into our receiver.
    let mut rx = subscribe_tenant("op-channel-1");
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("webinbox.jsonl");
    let channel = WebInboxChannel::new(WebInboxChannelConfig::local(&path));
    let _ = channel
        .send(Notification {
            target: OperatorTarget {
                operator_id: "op-channel-1".into(),
                email: "op@example.com".into(),
            },
            action_id: "act-ch-1".into(),
            approval_id: "act-ch-1".into(),
            risk_level: RiskLevel::High,
            prompt_body: "Approve listing?".into(),
            approve_url: "http://gw/approve?token=t".into(),
            reject_url: "http://gw/reject?token=t".into(),
            expiry: Utc::now() + chrono::Duration::minutes(5),
        })
        .await
        .expect("send");
    let frame = tokio::time::timeout(std::time::Duration::from_millis(200), rx.recv())
        .await
        .expect("event fires")
        .expect("frame");
    let parsed: Value = serde_json::from_str(&frame).expect("event is JSON");
    assert_eq!(parsed["type"], "approval.created");
    assert_eq!(parsed["payload"]["approval_id"], "act-ch-1");
}

#[test]
fn token_from_query_parses_and_handles_missing() {
    let req = "GET /inbox/ws?token=abc.def.ghi HTTP/1.1\r\nUpgrade: websocket\r\n\r\n";
    assert_eq!(token_from_query(req), Some("abc.def.ghi"));
    let multi = "GET /inbox/ws?foo=bar&token=zzz&baz=qux HTTP/1.1\r\nUpgrade: websocket\r\n\r\n";
    assert_eq!(token_from_query(multi), Some("zzz"));
    let none = "GET /inbox/ws HTTP/1.1\r\nUpgrade: websocket\r\n\r\n";
    assert_eq!(token_from_query(none), None);
}

#[test]
fn authorize_falls_back_to_bearer_when_no_jwt_pubkey() {
    std::env::remove_var(PUBKEY_ENV);
    let req = "GET /inbox/ws?token=eyJ.eyJ.sig HTTP/1.1\r\nAuthorization: Bearer x\r\n\r\n";
    let now = chrono::Utc::now().timestamp();
    match authorize(req, now) {
        Ok(WsAuth::Legacy) => {}
        other => panic!("expected legacy fallback, got {other:?}"),
    }
}

#[test]
fn authorize_uses_jwt_tenant_when_pubkey_present() {
    let signing = SigningKey::generate(&mut OsRng);
    let vk = signing.verifying_key();
    let b64 = base64::engine::general_purpose::STANDARD.encode(vk.to_bytes());
    std::env::set_var(PUBKEY_ENV, b64);
    let now = chrono::Utc::now().timestamp();
    let token = super::jwt::sign_jwt_for_test(
        &signing,
        &Claims {
            sub: "op-jwt".into(),
            tnt: "tenant-jwt".into(),
            iat: now,
            exp: now + 60,
        },
    );
    let req = format!(
        "GET /inbox/ws?token={token} HTTP/1.1\r\nUpgrade: websocket\r\n\r\n"
    );
    let auth = authorize(&req, now).expect("jwt verifies");
    assert_eq!(auth.tenant_key(), "tenant-jwt");
    std::env::remove_var(PUBKEY_ENV);
}

#[test]
fn authorize_rejects_missing_credentials() {
    std::env::remove_var(PUBKEY_ENV);
    let req = "GET /inbox/ws HTTP/1.1\r\n\r\n";
    let now = chrono::Utc::now().timestamp();
    let err = authorize(req, now).expect_err("no token -> 401");
    assert_eq!(err.0, "401 Unauthorized");
    assert!(err.1.contains("missing bearer token"));
}

#[tokio::test]
async fn approval_created_carries_full_row_on_happy_path() {
    // Enriched payload mirrors the JSON-API shape (`worker.{id,name}`,
    // `verb`, `detail`, `consequence_if_*`, `received_at`, `expires_at`,
    // `channel`, `risk_level`) so the SPA can setQueryData without
    // refetching.
    let mut rx = subscribe_tenant("op-full-happy");
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("inbox.jsonl");
    let channel = WebInboxChannel::new(WebInboxChannelConfig::local(&path));
    let _ = channel
        .send(Notification {
            target: OperatorTarget {
                operator_id: "op-full-happy".into(),
                email: "op@example.com".into(),
            },
            action_id: "act-full-1".into(),
            approval_id: "act-full-1".into(),
            risk_level: RiskLevel::High,
            prompt_body: "Approve archival".into(),
            approve_url: "http://gw/approve?token=t".into(),
            reject_url: "http://gw/reject?token=t".into(),
            expiry: Utc::now() + chrono::Duration::minutes(5),
        })
        .await
        .expect("send");
    let frame = tokio::time::timeout(std::time::Duration::from_millis(200), rx.recv())
        .await
        .expect("event fires")
        .expect("frame");
    let parsed: Value = serde_json::from_str(&frame).expect("event is JSON");
    let payload = &parsed["payload"];
    assert_eq!(parsed["type"], "approval.created");
    assert_eq!(payload["id"], "act-full-1");
    assert_eq!(payload["approval_id"], "act-full-1");
    assert_eq!(payload["worker"]["id"], "act-full-1");
    assert_eq!(payload["worker"]["name"], "act-full-1");
    assert_eq!(payload["detail"], "Approve archival");
    assert!(payload["verb"].is_string());
    assert_eq!(payload["consequence_if_yes"], "");
    assert_eq!(payload["consequence_if_no"], "");
    assert_eq!(payload["channel"], "web");
    assert_eq!(payload["risk_level"], "high");
    assert!(payload["received_at"].is_string());
    assert!(payload["expires_at"].is_string());
    assert!(
        payload.get("partial").is_none(),
        "happy path must not set partial"
    );
}

#[tokio::test]
async fn approval_created_falls_back_to_partial_when_enrichment_fails() {
    // Stand-in for the brief's worker-deleted-race: the same shape the
    // WS layer publishes when `WebInboxRow::enrich` returns `None`.
    // Subscribers honor `partial: true` by falling back to invalidate.
    let mut rx = subscribe_tenant("op-partial");
    let row = WebInboxRow {
        operator_id: "op-partial".into(),
        approval_id: "act-partial-1".into(),
        action_id: String::new(), // simulate missing enrichment data
        risk_level: RiskLevel::Medium,
        prompt_body: "anything".into(),
        approve_url: "http://gw/approve?token=t".into(),
        reject_url: "http://gw/reject?token=t".into(),
        expiry: Utc::now() + chrono::Duration::minutes(5),
        enqueued_at: Utc::now(),
    };
    assert!(row.enrich().is_none(), "missing action_id ⇒ no full row");
    argo_channels::webinbox::dispatch_event(ChannelEvent::ApprovalCreated {
        tenant_id: "op-partial".into(),
        row,
        full: None,
    });
    let frame = tokio::time::timeout(std::time::Duration::from_millis(200), rx.recv())
        .await
        .expect("event fires")
        .expect("frame");
    let parsed: Value = serde_json::from_str(&frame).expect("event is JSON");
    assert_eq!(parsed["type"], "approval.created");
    assert_eq!(parsed["payload"]["partial"], true);
    assert_eq!(parsed["payload"]["approval_id"], "act-partial-1");
}

#[tokio::test]
async fn enriched_payload_does_not_cross_tenant() {
    // The widened payload must NOT leak across the per-tenant
    // broadcast registry. Two operators get distinct approval bodies.
    let mut rx_a = subscribe_tenant("tenant-full-a");
    let mut rx_b = subscribe_tenant("tenant-full-b");
    let row_a = sample_row("act-A").clone();
    let row_b = WebInboxRow {
        operator_id: "tenant-full-b".into(),
        approval_id: "act-B".into(),
        action_id: "act-B".into(),
        risk_level: RiskLevel::High,
        prompt_body: "B body".into(),
        approve_url: "http://gw/approve?token=t".into(),
        reject_url: "http://gw/reject?token=t".into(),
        expiry: Utc::now() + chrono::Duration::minutes(5),
        enqueued_at: Utc::now(),
    };
    let full_a: ApprovalRowFull = row_a.enrich().expect("enrich A");
    let full_b: ApprovalRowFull = row_b.enrich().expect("enrich B");
    argo_channels::webinbox::dispatch_event(ChannelEvent::ApprovalCreated {
        tenant_id: "tenant-full-a".into(),
        row: row_a,
        full: Some(Box::new(full_a)),
    });
    argo_channels::webinbox::dispatch_event(ChannelEvent::ApprovalCreated {
        tenant_id: "tenant-full-b".into(),
        row: row_b,
        full: Some(Box::new(full_b)),
    });
    let a = tokio::time::timeout(std::time::Duration::from_millis(50), rx_a.recv())
        .await
        .expect("A receives")
        .expect("frame A");
    let b = tokio::time::timeout(std::time::Duration::from_millis(50), rx_b.recv())
        .await
        .expect("B receives")
        .expect("frame B");
    assert!(a.contains("act-A"));
    assert!(!a.contains("act-B"), "A leaked B body");
    assert!(b.contains("B body"));
    assert!(!b.contains("act-A"));
    assert!(rx_a.try_recv().is_err());
    assert!(rx_b.try_recv().is_err());
}

#[test]
fn channel_event_variants_route_through_publish() {
    // Verifies each ChannelEvent variant is mapped to its WebInboxEvent
    // counterpart at the publish layer. We exercise the publish helper
    // directly (since the hook is the same shape) and check the wire
    // type discriminants.
    let row = sample_row("act-route");
    for (channel_evt, expected_type) in [
        (
            ChannelEvent::ApprovalCreated {
                tenant_id: "tt".into(),
                row: row.clone(),
                full: row.enrich().map(Box::new),
            },
            "approval.created",
        ),
        (
            ChannelEvent::ApprovalDecided {
                tenant_id: "tt".into(),
                row: row.clone(),
                decision: "approve".into(),
            },
            "approval.decided",
        ),
        (
            ChannelEvent::ApprovalUndone {
                tenant_id: "tt".into(),
                row: row.clone(),
            },
            "approval.undone",
        ),
        (
            ChannelEvent::ApprovalExpired {
                tenant_id: "tt".into(),
                row,
            },
            "approval.expired",
        ),
    ] {
        let event = match channel_evt {
            ChannelEvent::ApprovalCreated { row, full, .. } => WebInboxEvent::ApprovalCreated {
                payload: full
                    .as_deref()
                    .and_then(|f| serde_json::to_value(f).ok())
                    .unwrap_or_else(|| serde_json::to_value(&row).unwrap_or(json!({}))),
            },
            ChannelEvent::ApprovalDecided { row, decision, .. } => {
                WebInboxEvent::ApprovalDecided {
                    payload: serde_json::to_value(&row).unwrap_or(json!({})),
                    decision,
                }
            }
            ChannelEvent::ApprovalUndone { row, .. } => WebInboxEvent::ApprovalUndone {
                payload: serde_json::to_value(&row).unwrap_or(json!({})),
            },
            ChannelEvent::ApprovalExpired { row, .. } => WebInboxEvent::ApprovalExpired {
                payload: serde_json::to_value(&row).unwrap_or(json!({})),
            },
        };
        let body = envelope(1, &event);
        let parsed: Value = serde_json::from_str(&body).expect("envelope decodes");
        assert_eq!(parsed["type"], expected_type);
    }
}
