use argo_store::approvals::{
    create_request, list_pending_near_deadline, mark_escalation_sent_at, DefaultDecision,
    NewApprovalRequest,
};
use argo_store::SessionStore;
use argo_types::TenantId;
use chrono::{TimeZone, Utc};
use std::time::Duration;

#[tokio::test]
async fn approval_reminder_candidate_scan_smoke() {
    let store = SessionStore::open(":memory:").await.expect("open store");
    store.migrate().await.expect("migrate");
    let now = Utc.timestamp_micros(1_700_000_000_000_000).unwrap();
    let tenant = TenantId::new();
    let near = seed(&store, tenant, now.timestamp_micros() + 60_000_000, 1).await;
    let sent = seed(&store, tenant, now.timestamp_micros() + 60_000_000, 2).await;
    mark_escalation_sent_at(store.pool(), &sent, now.timestamp_micros())
        .await
        .unwrap();
    seed(&store, tenant, now.timestamp_micros() + 3_600_000_000, 3).await;

    let rows = list_pending_near_deadline(store.pool(), Duration::from_secs(120), now)
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, near);
}

async fn seed(store: &SessionStore, tenant: TenantId, deadline_at: i64, byte: u8) -> String {
    create_request(
        store.pool(),
        &NewApprovalRequest {
            action_id: vec![byte],
            tenant_id: tenant,
            requested_by: "operator",
            deadline_at,
            default_decision: DefaultDecision::Deny,
        },
    )
    .await
    .expect("create approval")
}
