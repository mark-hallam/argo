use super::*;
use crate::SessionStore;
use chrono::{TimeZone, Utc};
use sqlx::SqlitePool;

async fn store() -> SessionStore {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    store
}

fn at(day: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 5, day, 12, 0, 0).unwrap()
}

#[tokio::test]
async fn set_show_list_clear_round_trip() {
    let store = store().await;
    let quotas = TenantQuotaStore::new(store.pool());
    let update = TenantQuotaUpdate {
        period: Some(QuotaPeriod::Monthly),
        max_usd_micros: Some(100_000_000),
        max_actions: Some(10),
        max_workers: Some(2),
    };
    let set = quotas.set_quota("t-1", update, at(6)).await.unwrap();
    assert_eq!(set.quota.unwrap().max_usd_micros, Some(100_000_000));
    assert_eq!(quotas.list(at(6)).await.unwrap().len(), 1);
    let cleared = quotas.clear("t-1", None, at(6)).await.unwrap();
    assert!(cleared.quota.is_none());
}

#[tokio::test]
async fn clear_single_field_preserves_others() {
    let store = store().await;
    let quotas = TenantQuotaStore::new(store.pool());
    quotas
        .set_quota(
            "t-1",
            TenantQuotaUpdate {
                period: Some(QuotaPeriod::Daily),
                max_usd_micros: Some(5),
                max_actions: Some(6),
                max_workers: None,
            },
            at(6),
        )
        .await
        .unwrap();
    let out = quotas
        .clear("t-1", Some(TenantQuotaField::MaxUsd), at(6))
        .await
        .unwrap();
    let quota = out.quota.unwrap();
    assert_eq!(quota.max_usd_micros, None);
    assert_eq!(quota.max_actions, Some(6));
}

#[tokio::test]
async fn usage_counts_worker_budget_for_period() {
    let store = store().await;
    seed_worker_budget(store.pool()).await;
    let quotas = TenantQuotaStore::new(store.pool());
    let usage = quotas
        .usage("t-1", QuotaPeriod::Daily, at(6))
        .await
        .unwrap();
    assert_eq!(usage.used_usd_micros, 700_000);
    assert_eq!(usage.used_actions, 2);
    assert_eq!(usage.used_workers, 1);
}

#[tokio::test]
async fn monthly_usage_ignores_previous_month() {
    let store = store().await;
    seed_worker_budget(store.pool()).await;
    sqlx::query("INSERT INTO worker_budget (tenant_id, worker_id, action_id, usd_micros, charged_at) VALUES (x'00', 'w-1', 'old', 900000, ?)")
        .bind(Utc.with_ymd_and_hms(2026, 4, 30, 12, 0, 0).unwrap().timestamp())
        .execute(store.pool())
        .await
        .unwrap();
    let quotas = TenantQuotaStore::new(store.pool());
    let usage = quotas
        .usage("t-1", QuotaPeriod::Monthly, at(6))
        .await
        .unwrap();
    assert_eq!(usage.used_usd_micros, 700_000);
}

#[tokio::test]
async fn has_headroom_blocks_over_usd() {
    let store = store().await;
    seed_worker_budget(store.pool()).await;
    let quotas = TenantQuotaStore::new(store.pool());
    quotas
        .set_quota(
            "t-1",
            TenantQuotaUpdate {
                period: Some(QuotaPeriod::Daily),
                max_usd_micros: Some(1_000_000),
                max_actions: None,
                max_workers: None,
            },
            at(6),
        )
        .await
        .unwrap();
    let decision = quotas
        .has_headroom_for("t-1", TenantQuotaKind::UsdMicros, 400_000, at(6))
        .await
        .unwrap();
    assert!(!decision.allowed);
    assert!(decision.reason.unwrap().contains("tenant_quota"));
}

#[tokio::test]
async fn has_headroom_allows_missing_quota() {
    let store = store().await;
    let quotas = TenantQuotaStore::new(store.pool());
    let decision = quotas
        .has_headroom_for("t-missing", TenantQuotaKind::Actions, 1, at(6))
        .await
        .unwrap();
    assert!(decision.allowed);
}

async fn seed_worker_budget(pool: &SqlitePool) {
    sqlx::query("CREATE TABLE worker (id TEXT PRIMARY KEY, tenant_id TEXT NOT NULL)")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query(include_str!("../migrations/14__worker_budget.sql"))
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO worker (id, tenant_id) VALUES ('w-1', 't-1')")
        .execute(pool)
        .await
        .unwrap();
    for (action, amount) in [("a-1", 300_000), ("a-2", 400_000)] {
        sqlx::query("INSERT INTO worker_budget (tenant_id, worker_id, action_id, usd_micros, charged_at) VALUES (x'00', 'w-1', ?, ?, ?)")
            .bind(action)
            .bind(amount)
            .bind(at(6).timestamp())
            .execute(pool)
            .await
            .unwrap();
    }
}
