//! Unit tests for `budget_watch`.

use super::*;
use chrono::{Duration as ChronoDuration, TimeZone};

async fn store() -> SessionStore {
    let store = SessionStore::open(":memory:").await.unwrap();
    ensure_alert_schema(&store).await.unwrap();
    ensure_worker_budget_schema(&store).await.unwrap();
    store
}

async fn insert_worker(store: &SessionStore, worker_id: &str, cap: u64) {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS worker (
            id TEXT PRIMARY KEY,
            tenant_id BLOB NOT NULL,
            auto_approval_json TEXT NOT NULL
         )",
    )
    .execute(store.pool())
    .await
    .unwrap();
    let policy = AutoApprovalPolicy {
        enabled: true,
        daily_budget_usd_micros: Some(cap),
        ..Default::default()
    };
    sqlx::query("INSERT INTO worker (id, tenant_id, auto_approval_json) VALUES (?, ?, ?)")
        .bind(worker_id)
        .bind(vec![0_u8; 16])
        .bind(serde_json::to_string(&policy).unwrap())
        .execute(store.pool())
        .await
        .unwrap();
}

async fn charge(store: &SessionStore, worker_id: &str, amount: u64, at: DateTime<Utc>) {
    worker_budget::record_charge(store.pool(), "act", worker_id, amount, at)
        .await
        .unwrap();
}

#[tokio::test]
async fn threshold_cross_fires_once_per_day() {
    let store = store().await;
    let now = Utc.with_ymd_and_hms(2026, 5, 7, 10, 0, 0).unwrap();
    insert_worker(&store, "w1", 1_000).await;
    charge(&store, "w1", 800, now).await;
    let first = run_tick(&store, Some("w1"), now).await.unwrap();
    assert_eq!(first.fired.len(), 1);
    assert_eq!(first.fired[0].threshold, 80);
    let second = run_tick(&store, Some("w1"), now).await.unwrap();
    assert!(second.fired.is_empty());
}

#[tokio::test]
async fn hundred_threshold_fires_independently() {
    let store = store().await;
    let now = Utc.with_ymd_and_hms(2026, 5, 7, 10, 0, 0).unwrap();
    insert_worker(&store, "w1", 1_000).await;
    charge(&store, "w1", 1_000, now).await;
    let tick = run_tick(&store, Some("w1"), now).await.unwrap();
    let thresholds: Vec<u8> = tick.fired.into_iter().map(|row| row.threshold).collect();
    assert_eq!(thresholds, vec![80, 100]);
}

#[tokio::test]
async fn dedup_resets_on_next_local_day() {
    let store = store().await;
    let day1 = Utc.with_ymd_and_hms(2026, 5, 7, 10, 0, 0).unwrap();
    let day2 = day1 + ChronoDuration::days(1);
    insert_worker(&store, "w1", 1_000).await;
    charge(&store, "w1", 800, day1).await;
    assert_eq!(
        run_tick(&store, Some("w1"), day1)
            .await
            .unwrap()
            .fired
            .len(),
        1
    );
    charge(&store, "w1", 800, day2).await;
    assert_eq!(
        run_tick(&store, Some("w1"), day2)
            .await
            .unwrap()
            .fired
            .len(),
        1
    );
}

#[tokio::test]
async fn synthetic_alert_bypasses_dedup() {
    let store = store().await;
    let now = Utc.with_ymd_and_hms(2026, 5, 7, 10, 0, 0).unwrap();
    let candidate = AlertCandidate {
        worker_id: "w1".into(),
        threshold: 100,
        spend_amount: 0,
        month_spend_amount: 0,
        cap_amount: 0,
        headroom_amount: 0,
        synthetic: true,
    };
    insert_alert(&store, &candidate, now, "push_failed:test")
        .await
        .unwrap();
    insert_alert(&store, &candidate, now, "push_failed:test")
        .await
        .unwrap();
    let rows = list_alerts(&store, Some("w1"), None, 10).await.unwrap();
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|row| row.synthetic));
}

#[test]
fn notification_includes_worker_and_threshold_metadata() {
    let now = Utc.with_ymd_and_hms(2026, 5, 7, 10, 0, 0).unwrap();
    let candidate = AlertCandidate {
        worker_id: "w1".into(),
        threshold: 80,
        spend_amount: 800,
        month_spend_amount: 800,
        cap_amount: 1_000,
        headroom_amount: 200,
        synthetic: false,
    };
    let notification = notification_for(&candidate, now).unwrap();
    assert_eq!(notification.action_id, "budget-alert-w1-80");
    assert!(notification.prompt_body.contains("Worker w1 crossed 80%"));
}

#[tokio::test]
async fn list_filters_by_worker_and_since() {
    let store = store().await;
    let old = Utc.with_ymd_and_hms(2026, 5, 6, 10, 0, 0).unwrap();
    let new = Utc.with_ymd_and_hms(2026, 5, 7, 10, 0, 0).unwrap();
    for (worker_id, at) in [("w1", old), ("w2", new)] {
        let candidate = AlertCandidate {
            worker_id: worker_id.into(),
            threshold: 80,
            spend_amount: 800,
            month_spend_amount: 800,
            cap_amount: 1_000,
            headroom_amount: 200,
            synthetic: false,
        };
        insert_alert(&store, &candidate, at, "push").await.unwrap();
    }
    let rows = list_alerts(&store, Some("w2"), Some(new - ChronoDuration::hours(1)), 10)
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].worker_id, "w2");
}
