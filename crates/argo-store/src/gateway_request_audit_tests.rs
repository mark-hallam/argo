use super::*;
use crate::SessionStore;

async fn fresh_pool() -> SqlitePool {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    store.pool().clone()
}

fn sample_entry(id: Uuid, received_at: i64) -> NewGatewayRequestAudit {
    NewGatewayRequestAudit {
        request_id: id,
        received_at,
        principal: Some("operator-1".into()),
        source_ip: "127.0.0.1".into(),
        method: "POST".into(),
        path: "/api/v1/sessions".into(),
        status_code: 201,
        latency_ms: 42,
        request_body_sha256: Some([7_u8; 32]),
        response_body_len: 12,
        user_agent: Some("test-agent".into()),
    }
}

#[tokio::test]
async fn migration_creates_gateway_request_audit_table() {
    let pool = fresh_pool().await;
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master \
         WHERE type = 'table' AND name = 'gateway_request_audit'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn record_and_get_round_trip_all_columns() {
    let pool = fresh_pool().await;
    let id = Uuid::now_v7();
    record(&pool, &sample_entry(id, 100)).await.unwrap();

    let row = get(&pool, &id.to_string()).await.unwrap().unwrap();
    assert_eq!(row.request_id, id.to_string());
    assert_eq!(row.received_at, 100);
    assert_eq!(row.principal.as_deref(), Some("operator-1"));
    assert_eq!(row.source_ip, "127.0.0.1");
    assert_eq!(row.method, "POST");
    assert_eq!(row.path, "/api/v1/sessions");
    assert_eq!(row.status_code, 201);
    assert_eq!(row.latency_ms, 42);
    assert_eq!(
        row.request_body_sha256.as_deref(),
        Some("0707070707070707070707070707070707070707070707070707070707070707")
    );
    assert_eq!(row.response_body_len, 12);
    assert_eq!(row.user_agent.as_deref(), Some("test-agent"));
}

#[tokio::test]
async fn list_filters_by_since_principal_and_status() {
    let pool = fresh_pool().await;
    let keep = Uuid::now_v7();
    let old = Uuid::now_v7();
    let other_status = Uuid::now_v7();
    record(&pool, &sample_entry(old, 10)).await.unwrap();
    record(&pool, &sample_entry(keep, 200)).await.unwrap();
    let mut entry = sample_entry(other_status, 300);
    entry.status_code = 500;
    record(&pool, &entry).await.unwrap();

    let rows = list(
        &pool,
        &GatewayRequestAuditFilter {
            since_micros: Some(100),
            principal: Some("operator-1".into()),
            status_code: Some(201),
            limit: 100,
        },
    )
    .await
    .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].request_id, keep.to_string());
}

#[tokio::test]
async fn list_orders_newest_first_and_clamps_limit() {
    let pool = fresh_pool().await;
    let first = Uuid::now_v7();
    let second = Uuid::now_v7();
    record(&pool, &sample_entry(first, 100)).await.unwrap();
    record(&pool, &sample_entry(second, 200)).await.unwrap();

    let rows = list(
        &pool,
        &GatewayRequestAuditFilter {
            limit: 1,
            ..GatewayRequestAuditFilter::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].request_id, second.to_string());
}

#[tokio::test]
async fn get_unknown_returns_none() {
    let pool = fresh_pool().await;
    assert!(get(&pool, &Uuid::now_v7().to_string())
        .await
        .unwrap()
        .is_none());
}
