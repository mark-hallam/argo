//! Smoke test for the by-vertical SQL — exercises the join
//! shape against a fresh in-memory store. Full integration
//! tests live in `tests/audit_by_vertical.rs` (CLI binary)
//! and `tests/audit_by_vertical_gateway.rs`.
#![cfg(test)]

use argo_audit::{AuditKind, AuditSink};
use argo_store::SessionStore;
use argo_types::{TenantId, WorkspaceId};
use std::sync::Arc;

#[tokio::test]
async fn join_query_returns_zero_rows_on_empty_db() {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    let pool = store.pool();
    let rows: Vec<(Option<String>, i64)> = sqlx::query_as(
        "SELECT s.vertical_id, COUNT(*) \
         FROM audit_record a \
         LEFT JOIN session s ON s.id = a.session_id \
         GROUP BY s.vertical_id \
         ORDER BY COUNT(*) DESC",
    )
    .fetch_all(pool)
    .await
    .unwrap();
    assert!(rows.is_empty());
}

#[tokio::test]
async fn join_query_groups_audit_rows_by_session_vertical_id() {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    let store = Arc::new(store);
    let tenant = TenantId::new();
    let ws = WorkspaceId::new();
    sqlx::query(
        "INSERT INTO workspace (id, tenant_id, name, root_path, created_at)
         VALUES (?, ?, 'audit-by-vertical-test', '.', strftime('%s','now'))",
    )
    .bind(ws.0.as_bytes().as_slice())
    .bind(tenant.0.as_bytes().as_slice())
    .execute(store.pool())
    .await
    .unwrap();
    let alpha_sid = store
        .create_session(tenant, ws, "mock", "mock", Some("a"), Some("alpha"))
        .await
        .unwrap();
    let beta_sid = store
        .create_session(tenant, ws, "mock", "mock", Some("b"), Some("beta"))
        .await
        .unwrap();
    let unbound_sid = store
        .create_session(tenant, ws, "mock", "mock", Some("u"), None)
        .await
        .unwrap();

    let sink = AuditSink::new(Arc::clone(&store));
    // 3 rows for alpha, 2 for beta, 1 for unbound.
    for _ in 0..3 {
        sink.write(Some(alpha_sid), AuditKind::ToolCall, serde_json::json!({}), None)
            .await
            .unwrap();
    }
    for _ in 0..2 {
        sink.write(Some(beta_sid), AuditKind::ToolCall, serde_json::json!({}), None)
            .await
            .unwrap();
    }
    sink.write(Some(unbound_sid), AuditKind::ToolCall, serde_json::json!({}), None)
        .await
        .unwrap();

    let rows: Vec<(Option<String>, i64)> = sqlx::query_as(
        "SELECT s.vertical_id, COUNT(*) \
         FROM audit_record a \
         LEFT JOIN session s ON s.id = a.session_id \
         GROUP BY s.vertical_id \
         ORDER BY COUNT(*) DESC, s.vertical_id ASC",
    )
    .fetch_all(store.pool())
    .await
    .unwrap();

    // Three buckets — alpha (3), beta (2), NULL (1).
    assert_eq!(rows.len(), 3);
    let alpha = rows
        .iter()
        .find(|(v, _)| v.as_deref() == Some("alpha"))
        .expect("alpha bucket");
    assert_eq!(alpha.1, 3);
    let beta = rows
        .iter()
        .find(|(v, _)| v.as_deref() == Some("beta"))
        .expect("beta bucket");
    assert_eq!(beta.1, 2);
    let null = rows
        .iter()
        .find(|(v, _)| v.is_none())
        .expect("null bucket for unbound session");
    assert_eq!(null.1, 1);
}
