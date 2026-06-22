//! Tests for [`crate::verify::verify_tenant`] /
//! [`crate::verify::tenant_head_hash`] — the per-tenant trust-surface
//! verifiers behind `GET /api/v1/audit/verify` (`w3-audit-trust-surface`).
//!
//! A tenant verifies its OWN session-chains only and never observes
//! another tenant's rows. Kept in its own file so `tests.rs` stays
//! under the 500-LOC budget.

#![allow(deprecated)]

use super::{tenant_head_hash, verify_tenant, AuditKind, AuditSink};
use argo_store::SessionStore;
use argo_types::{SessionId, TenantId, WorkspaceId};
use serde_json::json;
use std::sync::Arc;

async fn mem_store() -> Arc<SessionStore> {
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    store
}

/// Create a fresh session bound to `tenant` in a fresh workspace.
async fn session_for(store: &Arc<SessionStore>, tenant: TenantId) -> SessionId {
    let ws = WorkspaceId::new();
    sqlx::query(
        "INSERT INTO workspace (id, tenant_id, name, root_path, created_at)
         VALUES (?, ?, 'test', '.', strftime('%s','now'))",
    )
    .bind(ws.0.as_bytes().as_slice())
    .bind(tenant.0.as_bytes().as_slice())
    .execute(store.pool())
    .await
    .unwrap();
    store
        .create_session(tenant, ws, "model", "provider", Some("t"), None)
        .await
        .unwrap()
}

#[tokio::test]
async fn verify_tenant_only_walks_its_own_sessions() {
    let store = mem_store().await;
    let acme = TenantId::new();
    let other = TenantId::new();
    let acme_session = session_for(&store, acme).await;
    let other_session = session_for(&store, other).await;
    let acme_sink = AuditSink::new_with_tenant(store.clone(), acme.to_string());
    for i in 0..3 {
        acme_sink
            .write(Some(acme_session), AuditKind::ToolCall, json!({"i": i}), None)
            .await
            .unwrap();
    }
    let other_sink = AuditSink::new_with_tenant(store.clone(), other.to_string());
    for i in 0..5 {
        other_sink
            .write(Some(other_session), AuditKind::ToolCall, json!({"i": i}), None)
            .await
            .unwrap();
    }
    // Acme's view sees ONLY acme's 3 rows and is intact.
    let report = verify_tenant(&store, &acme.to_string()).await.unwrap();
    assert!(report.ok, "{:?}", report.breaks);
    assert_eq!(report.rows, 3, "must not count the other tenant's 5 rows");
    // Other tenant sees only its own 5.
    let other_report = verify_tenant(&store, &other.to_string()).await.unwrap();
    assert!(other_report.ok, "{:?}", other_report.breaks);
    assert_eq!(other_report.rows, 5);
}

#[tokio::test]
async fn verify_tenant_flags_tamper_in_its_own_chain() {
    let store = mem_store().await;
    let tenant = TenantId::new();
    let session = session_for(&store, tenant).await;
    let sink = AuditSink::new_with_tenant(store.clone(), tenant.to_string());
    for i in 0..3 {
        sink.write(Some(session), AuditKind::ToolCall, json!({"i": i}), None)
            .await
            .unwrap();
    }
    sqlx::query("UPDATE audit_record SET payload_json = '{\"x\": 9}' WHERE seq = 2")
        .execute(store.pool())
        .await
        .unwrap();
    let report = verify_tenant(&store, &tenant.to_string()).await.unwrap();
    assert!(
        !report.ok,
        "tamper must surface as a break, not a fake-intact"
    );
    assert!(!report.breaks.is_empty());
}

#[tokio::test]
async fn tenant_head_hash_tracks_chain_tip() {
    let store = mem_store().await;
    let tenant = TenantId::new();
    let session = session_for(&store, tenant).await;
    let tenant_str = tenant.to_string();
    // No rows yet → None.
    assert!(tenant_head_hash(&store, &tenant_str)
        .await
        .unwrap()
        .is_none());
    let sink = AuditSink::new_with_tenant(store.clone(), tenant_str.clone());
    sink.write(Some(session), AuditKind::TurnStart, json!({}), None)
        .await
        .unwrap();
    let head = tenant_head_hash(&store, &tenant_str).await.unwrap().unwrap();
    assert_eq!(head.len(), 64, "head hash is 32 bytes hex-encoded");
    // A different tenant with no rows still reads None.
    let empty = TenantId::new().to_string();
    assert!(tenant_head_hash(&store, &empty).await.unwrap().is_none());
}
