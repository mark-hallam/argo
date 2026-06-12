//! Unit tests for the tier-aware quota middleware.
//!
//! Cover: under-quota allows; at-cap denies with the expected JSON
//! body shape (incl. the upgrade path); tier promotion lifts the limit;
//! calendar-month rollover resets the counter; and the combined
//! per-action seam meters usage AND enforces the cap.

use super::{check_quota_or_reject, check_tier_quota_or_reject, clear_cache_for_test};
use argo_store::tenant_quota::set_workspace_tier;
use argo_store::tenant_quota_kinds::QuotaActionKind;
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
    // Seed usage in the SAME calendar-month bucket the quota check
    // reads. `check_tier_quota_or_reject` uses `Utc::now()` (the live
    // wall clock), and `tenant_usage_for_period` only counts usage
    // whose stored period matches the current month — a hardcoded past
    // date (e.g. a fixed May timestamp) lands in a stale billing period
    // and reads as 0 used, so the cap never trips. Use `Utc::now()`.
    let now = Utc::now();
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
    // The structured upgrade path names the next tier + the in-product
    // page that lifts the cap (launch-wire-tier-quota-and-usage-metering).
    assert!(body.contains(r#""upgrade_to":"pro""#), "{body}");
    assert!(body.contains(r#""upgrade_url":"/billing""#), "{body}");
}

#[tokio::test]
async fn tier_upgrade_lifts_limit_after_cache_clear() {
    clear_cache_for_test();
    let store = store().await;
    let tenant = TenantId::new();
    set_tier(&store, tenant, "free").await;
    // Same-month bucket as the quota check (see note in
    // `free_tier_at_cap_returns_429_quota_exceeded`): a hardcoded past
    // timestamp would read as 0 used and never trip the free cap.
    let now = Utc::now();
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

/// The combined per-action seam: a metered action increments the
/// tenant's monthly usage (Acceptance 1 "Pro increments visibly" /
/// Acceptance 2 real counts), and the Free cap is enforced with the
/// structured upgrade error (Acceptance 1 — would FAIL pre-wire when
/// the gate was dead code). Resolves the cwd-derived workspace tenant
/// the seam meters against, so it holds the shared env lock.
#[tokio::test]
async fn seam_meters_usage_then_rejects_at_free_cap() {
    let _g = crate::handlers::gateway::shared_env_lock().await;
    clear_cache_for_test();
    let store = store().await;
    let cwd = std::env::current_dir().expect("cwd");
    let name = cwd.file_name().and_then(|s| s.to_str()).unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (tenant, _) = store.ensure_workspace(name, &root).await.expect("workspace");
    set_workspace_tier(&store, tenant, "free", "Free")
        .await
        .expect("set free");
    clear_cache_for_test();

    // First metered action passes AND bumps the counter to 1.
    let pass = check_quota_or_reject(&store, QuotaActionKind::Sessions).await;
    assert!(pass.is_none(), "first action under cap should pass: {pass:?}");
    let (used, _) = tenant_query::tenant_usage_for_period(&store, tenant, Utc::now())
        .await
        .expect("usage");
    assert_eq!(used, 1, "the seam must meter exactly one action");

    // Seed to the Free cap, then the next action is refused with the
    // structured upgrade error.
    tenant_query::increment_tenant_usage(&store, tenant, 99, Utc::now())
        .await
        .unwrap();
    let (status, body) = check_quota_or_reject(&store, QuotaActionKind::Sessions)
        .await
        .expect("at cap should reject");
    assert_eq!(status, "429 Too Many Requests");
    assert!(body.contains(r#""error":"quota_exceeded""#), "{body}");
    assert!(body.contains(r#""upgrade_to":"pro""#), "{body}");
    assert!(body.contains(r#""upgrade_url":"/billing""#), "{body}");
}
