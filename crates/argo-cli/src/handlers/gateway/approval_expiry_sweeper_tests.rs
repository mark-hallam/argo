use super::*;
use argo_store::approvals::{
    create_request, get_request, ApprovalStatus, DefaultDecision, NewApprovalRequest,
};
use argo_types::TenantId;
use chrono::TimeZone;

async fn fresh_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.expect("open store");
    store.migrate().await.expect("migrate");
    Arc::new(store)
}

async fn seed(store: &SessionStore, tenant_id: TenantId, deadline_at: i64) -> String {
    create_request(
        store.pool(),
        &NewApprovalRequest {
            action_id: vec![1, 2, 3],
            tenant_id,
            requested_by: "operator",
            deadline_at,
            default_decision: DefaultDecision::Deny,
        },
    )
    .await
    .expect("create approval")
}

#[tokio::test]
async fn empty_pool_noops() {
    let store = fresh_store().await;
    let now = Utc.timestamp_micros(1_700_000_000_000_000).unwrap();
    assert_eq!(sweep_once(&store, now).await.unwrap(), 0);
}

#[tokio::test]
async fn single_overdue_row_promoted_to_expired() {
    let store = fresh_store().await;
    let tenant = TenantId::new();
    let id = seed(&store, tenant, 9).await;
    let now = Utc.timestamp_micros(10).unwrap();

    assert_eq!(sweep_once(&store, now).await.unwrap(), 1);
    let row = get_request(store.pool(), &id).await.unwrap().unwrap();
    assert_eq!(row.status, ApprovalStatus::Expired);
    assert_eq!(row.decided_by.as_deref(), Some(DECIDED_BY));
    assert_eq!(row.decision.as_deref(), Some("deny"));
}

#[tokio::test]
async fn two_rows_different_tenants_both_swept() {
    let store = fresh_store().await;
    seed(&store, TenantId::new(), 9).await;
    seed(&store, TenantId::new(), 8).await;

    let now = Utc.timestamp_micros(10).unwrap();
    assert_eq!(sweep_once(&store, now).await.unwrap(), 2);
}

#[tokio::test]
async fn deadline_in_future_is_not_touched() {
    let store = fresh_store().await;
    let tenant = TenantId::new();
    let id = seed(&store, tenant, 11).await;
    let now = Utc.timestamp_micros(10).unwrap();

    assert_eq!(sweep_once(&store, now).await.unwrap(), 0);
    let row = get_request(store.pool(), &id).await.unwrap().unwrap();
    assert_eq!(row.status, ApprovalStatus::Pending);
    assert!(row.decided_at.is_none());
}

#[tokio::test]
async fn audit_row_emitted_with_kind_and_count() {
    let store = fresh_store().await;
    seed(&store, TenantId::new(), 9).await;
    let now = Utc.timestamp_micros(10).unwrap();

    sweep_once(&store, now).await.unwrap();
    let payload: String = sqlx::query_scalar(
        "SELECT payload_json FROM audit_record WHERE kind = 'approval.expired_by_timeout' LIMIT 1",
    )
    .fetch_one(store.pool())
    .await
    .unwrap();
    let json: serde_json::Value = serde_json::from_str(&payload).unwrap();
    assert_eq!(json["kind"], "approval.expired_by_timeout");
    assert_eq!(json["count"], 1);
    assert_eq!(json["swept_at_iso"], now.to_rfc3339());
}
