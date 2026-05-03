//! Tests for `gap::detect_policy_denials` (#682).
#![cfg(test)]

use super::*;
use crate::{AuditKind, AuditSink};
use argo_store::SessionStore;
use argo_types::{TenantId, WorkspaceId};
use std::sync::Arc;

async fn fresh() -> (Arc<SessionStore>, AuditSink, argo_types::SessionId) {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    let store = Arc::new(store);
    let sink = AuditSink::new(Arc::clone(&store));
    let tenant = TenantId::new();
    let ws = WorkspaceId::new();
    sqlx::query(
        "INSERT INTO workspace (id, tenant_id, name, root_path, created_at)
         VALUES (?, ?, 'gap-test', '.', strftime('%s','now'))",
    )
    .bind(ws.0.as_bytes().as_slice())
    .bind(tenant.0.as_bytes().as_slice())
    .execute(store.pool())
    .await
    .unwrap();
    let sid = store
        .create_session(tenant, ws, "mock", "mock", Some("gap"), None)
        .await
        .unwrap();
    (store, sink, sid)
}

async fn write_deny(sink: &AuditSink, sid: argo_types::SessionId, tool: &str) {
    sink.write(
        Some(sid),
        AuditKind::RbacDeny,
        serde_json::json!({"tool": tool, "denied_by": "test"}),
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn no_denials_returns_empty() {
    let (store, _sink, sid) = fresh().await;
    let gaps = detect_policy_denials(&store, sid, 0, 1).await.unwrap();
    assert!(gaps.is_empty());
}

#[tokio::test]
async fn one_denial_below_threshold_yields_no_gap() {
    let (store, sink, sid) = fresh().await;
    write_deny(&sink, sid, "file_write").await;
    // Threshold = 2 → single denial doesn't fire.
    let gaps = detect_policy_denials(&store, sid, 0, 2).await.unwrap();
    assert!(gaps.is_empty());
}

#[tokio::test]
async fn repeated_denials_for_same_tool_fire_a_gap() {
    let (store, sink, sid) = fresh().await;
    for _ in 0..3 {
        write_deny(&sink, sid, "delegate_task").await;
    }
    let gaps = detect_policy_denials(&store, sid, 0, 3).await.unwrap();
    assert_eq!(gaps.len(), 1);
    assert_eq!(gaps[0].kind, GapKind::PolicyDenial);
    assert_eq!(gaps[0].anchor, "delegate_task");
    assert_eq!(gaps[0].count, 3);
    assert!(gaps[0].message.contains("delegate_task"));
}

#[tokio::test]
async fn denials_for_distinct_tools_yield_separate_gaps() {
    let (store, sink, sid) = fresh().await;
    for _ in 0..3 {
        write_deny(&sink, sid, "file_write").await;
    }
    for _ in 0..5 {
        write_deny(&sink, sid, "terminal").await;
    }
    let gaps = detect_policy_denials(&store, sid, 0, 2).await.unwrap();
    assert_eq!(gaps.len(), 2);
    // Sorted by count DESC, then anchor ASC.
    assert_eq!(gaps[0].anchor, "terminal");
    assert_eq!(gaps[0].count, 5);
    assert_eq!(gaps[1].anchor, "file_write");
    assert_eq!(gaps[1].count, 3);
}

#[tokio::test]
async fn detector_ignores_non_deny_audit_rows() {
    let (store, sink, sid) = fresh().await;
    // Mix of deny + escalation rows. Only deny should count.
    write_deny(&sink, sid, "x").await;
    write_deny(&sink, sid, "x").await;
    sink.write(
        Some(sid),
        AuditKind::EscalationRequired,
        serde_json::json!({"tool": "x"}),
    )
    .await
    .unwrap();
    let gaps = detect_policy_denials(&store, sid, 0, 2).await.unwrap();
    assert_eq!(gaps.len(), 1);
    assert_eq!(gaps[0].count, 2, "escalation rows must not be counted");
}

#[tokio::test]
async fn json_payload_round_trips() {
    let g = CapabilityGap {
        kind: GapKind::PolicyDenial,
        anchor: "delegate_task".into(),
        count: 4,
        message: "test".into(),
    };
    let v = g.to_json();
    assert_eq!(v["kind"], "policy_denial");
    assert_eq!(v["anchor"], "delegate_task");
    assert_eq!(v["count"], 4);
    assert_eq!(v["message"], "test");
}

#[tokio::test]
async fn detector_can_emit_back_to_audit_chain_as_capability_gap() {
    // End-to-end: detect + write the gap row + read it back.
    // Locks the integration that the detector's output is the
    // canonical payload for `capability.gap` audit rows.
    let (store, sink, sid) = fresh().await;
    for _ in 0..3 {
        write_deny(&sink, sid, "file_write").await;
    }
    let gaps = detect_policy_denials(&store, sid, 0, 2).await.unwrap();
    assert_eq!(gaps.len(), 1);
    sink.write(Some(sid), AuditKind::CapabilityGap, gaps[0].to_json())
        .await
        .unwrap();
    // Chain still verifies and the new row carries the right
    // kind label.
    let rows = store.list_audit_for_session(sid).await.unwrap();
    assert!(rows.iter().any(|r| r.kind == "capability.gap"));
}
