use super::*;
use argo_store::approvals::{create_request, expire_overdue, DefaultDecision, NewApprovalRequest};
use argo_types::TenantId;

async fn fresh_store() -> SessionStore {
    let store = SessionStore::open(":memory:").await.expect("open store");
    store.migrate().await.expect("migrate");
    store
}

async fn seed(store: &SessionStore, action_id: Vec<u8>, deadline_at: i64) -> String {
    create_request(
        store.pool(),
        &NewApprovalRequest {
            action_id,
            tenant_id: TenantId::new(),
            requested_by: "operator",
            deadline_at,
            default_decision: DefaultDecision::Deny,
        },
    )
    .await
    .expect("create")
}

#[tokio::test]
async fn no_expired_rows_returns_empty_list() {
    let store = fresh_store().await;
    let now = Utc.timestamp_micros(100).unwrap();
    let rows = collect_from_store(&store, now, Duration::from_secs(60))
        .await
        .unwrap();
    assert!(rows.is_empty());
}

#[tokio::test]
async fn multiple_expired_rows_are_rendered() {
    let store = fresh_store().await;
    let id_a = seed(&store, vec![0xaa], 9).await;
    let id_b = seed(&store, vec![0xbb, 0xcc], 8).await;
    expire_overdue(store.pool(), 10, SWEEPER_DECIDED_BY)
        .await
        .unwrap();

    let now = Utc.timestamp_micros(10).unwrap();
    let rows = collect_from_store(&store, now, Duration::from_secs(60))
        .await
        .unwrap();
    assert_eq!(rows.len(), 2);
    assert!(rows
        .iter()
        .any(|row| row.id == id_a && row.action_id == "aa"));
    assert!(rows
        .iter()
        .any(|row| row.id == id_b && row.action_id == "bbcc"));
    assert!(rows.iter().all(|row| row.decision == "deny"));
}

#[tokio::test]
async fn window_filter_excludes_older_rows() {
    let store = fresh_store().await;
    seed(&store, vec![1], 9).await;
    expire_overdue(store.pool(), 10, SWEEPER_DECIDED_BY)
        .await
        .unwrap();

    let now = Utc.timestamp_micros(30_000_000).unwrap();
    let rows = collect_from_store(&store, now, Duration::from_secs(1))
        .await
        .unwrap();
    assert!(rows.is_empty());
}

#[test]
fn parse_since_supports_common_units() {
    assert_eq!(parse_since("24h").unwrap(), Duration::from_secs(86_400));
    assert_eq!(parse_since("30m").unwrap(), Duration::from_secs(1_800));
    assert_eq!(parse_since("45s").unwrap(), Duration::from_secs(45));
    assert_eq!(parse_since("10").unwrap(), Duration::from_secs(10));
}
