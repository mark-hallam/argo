use super::*;
use crate::SessionStore;
use chrono::TimeZone;

async fn fresh_pool() -> SqlitePool {
    let store = SessionStore::open(":memory:").await.expect("open store");
    store.migrate().await.expect("migrate");
    store.pool().clone()
}

async fn seed(pool: &SqlitePool, tenant_id: TenantId, deadline_at: i64, action: u8) -> String {
    create_request(
        pool,
        &NewApprovalRequest {
            action_id: vec![action],
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
async fn migrate_adds_escalation_sent_at_column() {
    let pool = fresh_pool().await;
    let names: Vec<String> =
        sqlx::query_scalar("SELECT name FROM pragma_table_info('approval_request')")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert!(
        names.contains(&"escalation_sent_at".to_string()),
        "missing escalation_sent_at column, got {names:?}"
    );
}

#[tokio::test]
async fn list_pending_near_deadline_filters_to_unescalated_future_rows_inside_window() {
    let pool = fresh_pool().await;
    let tenant = TenantId::new();
    let now = chrono::Utc.timestamp_micros(1_700_000_000_000_000).unwrap();
    let near = seed(&pool, tenant, now.timestamp_micros() + 10_000_000, 1).await;
    let already_sent = seed(&pool, tenant, now.timestamp_micros() + 20_000_000, 2).await;
    mark_escalation_sent_at(&pool, &already_sent, now.timestamp_micros())
        .await
        .unwrap();
    seed(&pool, tenant, now.timestamp_micros() + 3_600_000_000, 3).await;
    seed(&pool, tenant, now.timestamp_micros() - 1, 4).await;

    let rows = list_pending_near_deadline(&pool, std::time::Duration::from_secs(30), now)
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, near);
    assert!(rows[0].escalation_sent_at.is_none());
}

#[tokio::test]
async fn mark_escalation_sent_is_idempotent() {
    let pool = fresh_pool().await;
    let tenant = TenantId::new();
    let now = 1_700_000_000_000_000i64;
    let id = seed(&pool, tenant, now + 1, 1).await;

    let first = mark_escalation_sent_at(&pool, &id, now).await.unwrap();
    let second = mark_escalation_sent_at(&pool, &id, now + 1).await.unwrap();
    assert!(first);
    assert!(!second);

    let row = get_request(&pool, &id).await.unwrap().unwrap();
    assert_eq!(row.escalation_sent_at, Some(now));
    assert_eq!(row.status, ApprovalStatus::Pending);
}

#[tokio::test]
async fn mark_escalation_sent_ignores_decided_rows() {
    let pool = fresh_pool().await;
    let tenant = TenantId::new();
    let id = seed(&pool, tenant, 1_700_000_000, 1).await;
    decide_request(&pool, &id, "grantor", ApprovalDecision::Granted)
        .await
        .unwrap();

    assert!(!mark_escalation_sent_at(&pool, &id, 1_700_000_000_000_000)
        .await
        .unwrap());
    let row = get_request(&pool, &id).await.unwrap().unwrap();
    assert!(row.escalation_sent_at.is_none());
}
