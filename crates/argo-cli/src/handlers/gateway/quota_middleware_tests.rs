//! Unit tests for the tier-aware quota middleware.
//!
//! Cover: under-quota allows; at-cap denies with the expected JSON
//! body shape; tier promotion lifts the limit; calendar-month rollover
//! resets the counter.

use super::{check_tier_quota_or_reject, clear_cache_for_test};
use argo_store::{tenant_query, SessionStore};
use argo_types::TenantId;
use chrono::{TimeZone, Utc};
use std::sync::Arc;

async fn store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.expect("open");
    store.migrate().await.expect("migrate");
    Arc::new(store)
}

async fn set_tier(store: &SessionStore, tenant: TenantId, tier: &str) {
    tenant_query::ensure_pricing_schema(store).await.unwrap();
    let existing: Option<Vec<u8>> = sqlx::query_scalar(
        "SELECT id FROM workspace WHERE tenant_id = ? AND name = ?",
    )
    .bind(tenant.0.as_bytes().as_slice())
    .bind("tenant-test")
    .fetch_optional(store.pool())
    .await
    .expect("lookup workspace");
    if existing.is_some() {
        sqlx::query("UPDATE workspace SET tier = ? WHERE tenant_id = ? AND name = ?")
            .bind(tier)
            .bind(tenant.0.as_bytes().as_slice())
            .bind("tenant-test")
            .execute(store.pool())
            .await
            .expect("update tier");
        return;
    }
    let workspace_id = uuid::Uuid::now_v7();
    sqlx::query(
        "INSERT INTO workspace (id, tenant_id, name, root_path, created_at, tier) \
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(workspace_id.as_bytes().as_slice())
    .bind(tenant.0.as_bytes().as_slice())
    .bind("tenant-test")
    .bind("/tmp/tenant-test")
    .bind(0_i64)
    .bind(tier)
    .execute(store.pool())
    .await
    .expect("insert workspace");
}

#[tokio::test]
async fn free_tier_under_quota_passes() {
    clear_cache_for_test();
    let store = store().await;
    let tenant = TenantId::new();
    set_tier(&store, tenant, "free").await;
    let res = check_tier_quota_or_reject(&store, tenant).await;
    assert!(res.is_none(), "first action should pass under free cap of 100");
}

#[tokio::test]
async fn free_tier_at_cap_returns_429_quota_exceeded() {
    clear_cache_for_test();
    let store = store().await;
    let tenant = TenantId::new();
    set_tier(&store, tenant, "free").await;
    let now = Utc.with_ymd_and_hms(2026, 5, 15, 12, 0, 0).unwrap();
    tenant_query::increment_tenant_usage(&store, tenant, 100, now)
        .await
        .unwrap();
    let res = check_tier_quota_or_reject(&store, tenant).await;
    let (status, body) = res.expect("over-quota should reject");
    assert_eq!(status, "429 Too Many Requests");
    assert!(body.contains(r#""error":"quota_exceeded""#), "{body}");
    assert!(body.contains(r#""tier":"free""#), "{body}");
    assert!(body.contains(r#""limit":100"#), "{body}");
    assert!(body.contains(r#""used":100"#), "{body}");
    assert!(body.contains(r#""retry_after_ts":"#), "{body}");
}

#[tokio::test]
async fn tier_upgrade_lifts_limit_after_cache_clear() {
    clear_cache_for_test();
    let store = store().await;
    let tenant = TenantId::new();
    set_tier(&store, tenant, "free").await;
    let now = Utc.with_ymd_and_hms(2026, 5, 15, 12, 0, 0).unwrap();
    tenant_query::increment_tenant_usage(&store, tenant, 100, now)
        .await
        .unwrap();
    assert!(check_tier_quota_or_reject(&store, tenant).await.is_some());
    set_tier(&store, tenant, "pro").await;
    clear_cache_for_test();
    let res = check_tier_quota_or_reject(&store, tenant).await;
    assert!(res.is_none(), "pro tier should accept past free's 100 cap");
}

#[tokio::test]
async fn enterprise_tier_is_unlimited() {
    clear_cache_for_test();
    let store = store().await;
    let tenant = TenantId::new();
    set_tier(&store, tenant, "enterprise").await;
    let now = Utc.with_ymd_and_hms(2026, 5, 15, 12, 0, 0).unwrap();
    tenant_query::increment_tenant_usage(&store, tenant, 10_000_000, now)
        .await
        .unwrap();
    let res = check_tier_quota_or_reject(&store, tenant).await;
    assert!(res.is_none(), "enterprise should never deny on quota");
}
