use super::*;
use chrono::TimeZone;

async fn store() -> SessionStore {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    ensure_budget_schema(&store).await.unwrap();
    store
}

fn tenant(n: u128) -> Uuid {
    Uuid::from_u128(n)
}

async fn insert_charge(
    store: &SessionStore,
    tenant: Uuid,
    worker_id: &str,
    action_id: &str,
    micros: i64,
    day: u32,
) {
    let at = Utc
        .with_ymd_and_hms(2026, 5, day, 12, 0, 0)
        .single()
        .unwrap()
        .timestamp();
    sqlx::query(
        "INSERT INTO worker_budget \
         (tenant_id, worker_id, action_id, usd_micros, charged_at) \
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(tenant.as_bytes().as_slice())
    .bind(worker_id)
    .bind(action_id)
    .bind(micros)
    .bind(at)
    .execute(store.pool())
    .await
    .unwrap();
}

#[tokio::test]
async fn report_filters_by_since_and_breaks_down_by_day() {
    let store = store().await;
    let tenant = tenant(1);
    insert_charge(&store, tenant, "worker-a", "a1", 1_000_000, 1).await;
    insert_charge(&store, tenant, "worker-a", "a2", 2_000_000, 2).await;

    let report = report(
        &store,
        "worker-a",
        None,
        Some("2026-05-02T00:00:00Z"),
        Some(WorkerCostsBreakdown::Day),
        10,
    )
    .await
    .unwrap();
    assert_eq!(report.total_usd_micros, 2_000_000);
    assert_eq!(report.row_count, 1);
    assert_eq!(
        report.breakdown_rows,
        vec![BreakdownRow {
            key: "2026-05-02".to_string(),
            usd_micros: 2_000_000,
            row_count: 1,
        }]
    );
}

#[tokio::test]
async fn report_filters_by_tenant_for_same_worker_id() {
    let store = store().await;
    let tenant_a = tenant(1);
    let tenant_b = tenant(2);
    insert_charge(&store, tenant_a, "worker-a", "a1", 1_000_000, 1).await;
    insert_charge(&store, tenant_b, "worker-a", "b1", 3_000_000, 1).await;

    let report = report(
        &store,
        "worker-a",
        Some(&tenant_a.to_string()),
        None,
        Some(WorkerCostsBreakdown::Action),
        10,
    )
    .await
    .unwrap();
    assert_eq!(report.total_usd_micros, 1_000_000);
    assert_eq!(report.row_count, 1);
    assert_eq!(report.breakdown_rows[0].key, "a1");
}

#[tokio::test]
async fn zero_charge_worker_returns_empty_success() {
    let store = store().await;
    let report = report(&store, "worker-zero", None, None, None, 10)
        .await
        .unwrap();
    assert_eq!(report.total_usd_micros, 0);
    assert_eq!(report.row_count, 0);
    assert!(report.recent_charges.is_empty());
}

#[test]
fn parse_since_rejects_non_rfc3339() {
    let err = parse_since(Some("last-week")).unwrap_err();
    assert!(err.to_string().contains("invalid --since"));
}
