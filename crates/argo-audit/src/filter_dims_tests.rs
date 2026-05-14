//! Integration tests for the denormalized filter-dimension write
//! path + verify-chain denorm sanity belt (Stream
//! `mission-enterprise-audit-full-text-search`, migration v67).
//!
//! Split out of `tests.rs` per HARD RULE 6 (500-LOC per-file
//! ceiling).

use super::*;
use argo_store::SessionStore;
use argo_types::{SessionId, TenantId, WorkspaceId};
use std::sync::Arc;

async fn fresh() -> (Arc<SessionStore>, SessionId) {
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    let tenant = TenantId::new();
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
    let session = store
        .create_session(tenant, ws, "model", "provider", Some("t"), None)
        .await
        .unwrap();
    (store, session)
}

type DimRow = (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
);

async fn fetch_dim_row(store: &SessionStore, session: SessionId) -> DimRow {
    sqlx::query_as(
        "SELECT worker_id, channel, action_type, outcome_status, \
                approver_id, cost_band, data_classification \
         FROM audit_record WHERE session_id = ? LIMIT 1",
    )
    .bind(session.0.as_bytes().as_slice())
    .fetch_one(store.pool())
    .await
    .unwrap()
}

#[tokio::test]
async fn write_populates_filter_dim_columns_from_payload() {
    // Write path denormalizes the 7 J14 filter dims into first-class
    // columns. Asserts each column lands on the row + the
    // `cost_usd` → `cost_band` derivation.
    let (store, session) = fresh().await;
    let sink = AuditSink::new(store.clone());
    sink.write(
        Some(session),
        AuditKind::HttpRequest,
        serde_json::json!({
            "worker_id": "w1",
            "channel": "slack",
            "action_type": "approval.decided",
            "outcome_status": "succeeded",
            "approver_id": "a1",
            "cost_usd": 0.5,
            "data_classification": "internal",
        }),
        None,
    )
    .await
    .unwrap();
    sink.flush_pending().await.unwrap();
    let row = fetch_dim_row(&store, session).await;
    assert_eq!(row.0.as_deref(), Some("w1"));
    assert_eq!(row.1.as_deref(), Some("slack"));
    assert_eq!(row.2.as_deref(), Some("approval.decided"));
    assert_eq!(row.3.as_deref(), Some("succeeded"));
    assert_eq!(row.4.as_deref(), Some("a1"));
    assert_eq!(row.5.as_deref(), Some("from001to10"));
    assert_eq!(row.6.as_deref(), Some("internal"));
}

#[tokio::test]
async fn write_with_no_dim_keys_leaves_columns_null() {
    // Downgrade-safety: a write with no dim keys in payload lands
    // NULL for every filter column, matching how legacy pre-v67
    // rows look. The read path's `(col IN (?,…) OR col IS NULL)`
    // gate keeps these rows discoverable.
    let (store, session) = fresh().await;
    let sink = AuditSink::new(store.clone());
    sink.write(
        Some(session),
        AuditKind::TurnStart,
        serde_json::json!({"unrelated": 1}),
        None,
    )
    .await
    .unwrap();
    sink.flush_pending().await.unwrap();
    let row = fetch_dim_row(&store, session).await;
    assert!(row.0.is_none());
    assert!(row.1.is_none());
    assert!(row.2.is_none());
    assert!(row.3.is_none());
    assert!(row.4.is_none());
    assert!(row.5.is_none());
    assert!(row.6.is_none());
}

#[tokio::test]
async fn dim_columns_do_not_affect_chain_hash() {
    // The hash chain is over (parent_hash, kind, payload_json) —
    // promoting the dim columns must NOT shift the chain. Asserts
    // verify_session passes on a session whose rows have populated
    // dim columns.
    let (store, session) = fresh().await;
    let sink = AuditSink::new(store.clone());
    for i in 0..4 {
        sink.write(
            Some(session),
            AuditKind::HttpRequest,
            serde_json::json!({
                "worker_id": format!("w{i}"),
                "channel": "web",
                "action_type": "approval.decided",
                "outcome_status": "succeeded",
                "cost_usd": (i as f64) * 0.5,
            }),
            None,
        )
        .await
        .unwrap();
    }
    sink.flush_pending().await.unwrap();
    let report = verify_session(&store, session).await.unwrap();
    assert!(report.ok, "{:?}", report.breaks);
    assert_eq!(report.rows, 4);
}

#[tokio::test]
async fn verify_chain_flags_denorm_column_drift() {
    // Acceptance criterion: tampering with a denorm column (without
    // touching payload_json) makes verify_session report a typed
    // break naming the drifted column.
    let (store, session) = fresh().await;
    let sink = AuditSink::new(store.clone());
    sink.write(
        Some(session),
        AuditKind::HttpRequest,
        serde_json::json!({
            "worker_id": "honest",
            "channel": "web",
            "action_type": "x",
        }),
        None,
    )
    .await
    .unwrap();
    sink.flush_pending().await.unwrap();
    sqlx::query("UPDATE audit_record SET worker_id = 'tampered' WHERE seq = 1")
        .execute(store.pool())
        .await
        .unwrap();
    let report = verify_session(&store, session).await.unwrap();
    assert!(!report.ok);
    let any_drift = report
        .breaks
        .iter()
        .any(|b| b.reason.contains("worker_id") && b.reason.contains("drift"));
    assert!(
        any_drift,
        "expected worker_id drift break, got {:?}",
        report.breaks
    );
}

#[tokio::test]
async fn verify_chain_skips_denorm_check_for_legacy_null_rows() {
    // Legacy rows (pre-v67) write the dim columns as NULL.
    // verify_session must skip the denorm check entirely for those
    // rows — only non-NULL columns trigger the drift assertion.
    let (store, session) = fresh().await;
    let sink = AuditSink::new(store.clone());
    sink.write(
        Some(session),
        AuditKind::TurnStart,
        serde_json::json!({"worker_id": "wL"}),
        None,
    )
    .await
    .unwrap();
    sink.flush_pending().await.unwrap();
    // Simulate the legacy posture: payload_json carries worker_id
    // but the denorm column was never populated.
    sqlx::query("UPDATE audit_record SET worker_id = NULL WHERE seq = 1")
        .execute(store.pool())
        .await
        .unwrap();
    let report = verify_session(&store, session).await.unwrap();
    assert!(
        report.ok,
        "legacy NULL columns must not trip the denorm check: {:?}",
        report.breaks
    );
}
