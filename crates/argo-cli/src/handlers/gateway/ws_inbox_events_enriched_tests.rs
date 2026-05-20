use super::*;
use argo_channels::webinbox::{
    ApprovalRowFull, WebInboxChannel, WebInboxChannelConfig, WebInboxEvent as ChannelEvent,
    WebInboxRow,
};
use argo_channels::{Notification, NotifyChannel, OperatorTarget, RiskLevel};
use chrono::Utc;
use serde_json::{json, Value};

#[tokio::test]
async fn approval_created_carries_full_row_on_happy_path() {
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
    let mut rx = subscribe_tenant("op-partial");
    let row = WebInboxRow {
        operator_id: "op-partial".into(),
        approval_id: "act-partial-1".into(),
        action_id: String::new(),
        risk_level: RiskLevel::Medium,
        prompt_body: "anything".into(),
        approve_url: "http://gw/approve?token=t".into(),
        reject_url: "http://gw/reject?token=t".into(),
        expiry: Utc::now() + chrono::Duration::minutes(5),
        enqueued_at: Utc::now(),
    };
    assert!(row.enrich().is_none(), "missing action_id => no full row");
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
            ChannelEvent::ApprovalCreated { row, full, .. } => WsInboxEventRow::ApprovalCreated {
                payload: full
                    .as_deref()
                    .and_then(|f| serde_json::to_value(f).ok())
                    .unwrap_or_else(|| serde_json::to_value(&row).unwrap_or(json!({}))),
            },
            ChannelEvent::ApprovalDecided { row, decision, .. } => {
                WsInboxEventRow::ApprovalDecided {
                    payload: serde_json::to_value(&row).unwrap_or(json!({})),
                    decision,
                }
            }
            ChannelEvent::ApprovalUndone { row, .. } => WsInboxEventRow::ApprovalUndone {
                payload: serde_json::to_value(&row).unwrap_or(json!({})),
            },
            ChannelEvent::ApprovalExpired { row, .. } => WsInboxEventRow::ApprovalExpired {
                payload: serde_json::to_value(&row).unwrap_or(json!({})),
            },
        };
        let body = envelope(1, &event);
        let parsed: Value = serde_json::from_str(&body).expect("envelope decodes");
        assert_eq!(parsed["type"], expected_type);
    }
}
