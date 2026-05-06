//! Tests for `argo budget anomaly-tick` (the cli-side wrapper
//! around the store's spend_anomaly module). Sibling test file so
//! the host module stays focused on the handlers.
//!
//! Most tests run against an in-memory SQLite pool so we exercise
//! both the data-flow (load → update → save → record) and the
//! json/text output paths without needing a real workspace.
use super::short;
use chrono::{TimeZone, Utc};

#[test]
fn short_truncates_long_strings_to_12_chars() {
    assert_eq!(short("0123456789ABCDEF"), "0123456789AB");
    assert_eq!(short("short"), "short");
    assert_eq!(short(""), "");
}

// Each module owns its own schema setup at runtime — the
// argo-store migration runner only installs v1-v11; downstream
// modules (worker_budget, anomaly, spend_anomaly) call
// ensure_schema themselves on first use. Tests follow the same
// pattern: spin up an in-memory store, install the schemas this
// flow touches, then exercise the handler.
const WORKER_BUDGET_SCHEMA: &str = include_str!(
    "../../../argo-store/migrations/14__worker_budget.sql"
);

async fn fresh_store_with_budget_schema() -> argo_store::SessionStore {
    let dir = tempfile::tempdir().expect("tempdir");
    // Leak the tempdir so the file-backed pool stays valid for
    // the rest of the test (SessionStore holds the pool, not
    // the dir handle).
    let path = dir.path().join("argo.db");
    std::mem::forget(dir);
    let store = argo_store::SessionStore::open(&path)
        .await
        .expect("file store");
    store.migrate().await.expect("v1-v11 migrate");
    sqlx::query(WORKER_BUDGET_SCHEMA)
        .execute(store.pool())
        .await
        .expect("worker_budget schema");
    argo_store::spend_anomaly::ensure_schema(store.pool())
        .await
        .expect("spend anomaly schema");
    store
}

#[tokio::test]
async fn tick_records_anomaly_when_observed_far_above_baseline() {
    use argo_store::spend_anomaly::{
        list_anomalies, save_baseline, AnomalyFilters, SpendBaseline,
    };
    let store = fresh_store_with_budget_schema().await;
    // Seed a baseline with stddev > 0 for hour 13.
    let mut baseline = SpendBaseline::empty("w-1", 13);
    for v in [1.0, 1.0, 1.0, 1.0, 1.0, 2.0] {
        baseline.update(v, 1_000);
    }
    save_baseline(store.pool(), &baseline)
        .await
        .expect("seed baseline");
    // Inject one charge in the previous-hour window worth $50.
    sqlx::query(
        "INSERT INTO worker_budget (tenant_id, worker_id, action_id, usd_micros, charged_at) \
         VALUES (?, 'w-1', 'a-spike', 50000000, ?)",
    )
    .bind(vec![0u8; 16])
    .bind(
        Utc.with_ymd_and_hms(2026, 5, 7, 13, 30, 0)
            .unwrap()
            .timestamp(),
    )
    .execute(store.pool())
    .await
    .expect("seed budget");
    let now = Utc.with_ymd_and_hms(2026, 5, 7, 14, 30, 0).unwrap();
    super::tick(&store, now, true).await.expect("tick");
    let rows = list_anomalies(
        store.pool(),
        &AnomalyFilters {
            worker_id: Some("w-1"),
            since_micros: None,
        },
        100,
    )
    .await
    .expect("list");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].hour_of_day, 13);
    assert!(rows[0].z_score.abs() > 4.0);
}

#[tokio::test]
async fn tick_with_no_active_workers_is_noop_and_writes_nothing() {
    use argo_store::spend_anomaly::{list_anomalies, AnomalyFilters};
    let store = fresh_store_with_budget_schema().await;
    let now = Utc.with_ymd_and_hms(2026, 5, 7, 14, 30, 0).unwrap();
    super::tick(&store, now, true).await.expect("tick");
    let rows = list_anomalies(store.pool(), &AnomalyFilters::default(), 10)
        .await
        .expect("list");
    assert!(rows.is_empty());
}
