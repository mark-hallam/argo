//! Unit + integration tests for the canonical tier capability gate.

use super::{check_worker_cap_or_reject, tenant_tier_or_free, tier_allows};
use argo_store::{tenant_query, SessionStore};
use argo_types::TenantId;
use std::sync::Arc;

async fn store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.expect("open");
    store.migrate().await.expect("migrate");
    Arc::new(store)
}

/// Set `workspace.tier` for `tenant`. Mirrors the helper in
/// `tier_capability_filter_tests.rs`.
async fn set_tier(store: &SessionStore, tenant: TenantId, tier: &str) {
    tenant_query::ensure_pricing_schema(store).await.unwrap();
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

/// Insert `n` minimal worker rows for `tenant`. `worker.tenant_id`
/// stores the hyphenated UUID string — the same form the real hire
/// path binds — so `tenant_worker_count` sees them.
async fn insert_workers(store: &SessionStore, tenant: TenantId, n: usize) {
    for _ in 0..n {
        // Fresh UUID per row so repeated calls in one test never collide
        // on the `worker.id` primary key.
        sqlx::query(
            "INSERT INTO worker (id, tenant_id, vertical_id, persona_id, role_id, \
             memory_scope, auto_approval_json, channels_json, created_at) \
             VALUES (?, ?, '', '', '', 'worker_self', '{}', '[]', 0)",
        )
        .bind(uuid::Uuid::now_v7().to_string())
        .bind(tenant.to_string())
        .execute(store.pool())
        .await
        .expect("insert worker");
    }
}

#[tokio::test]
async fn free_tier_lacks_provider_real_keys() {
    let store = store().await;
    let tenant = TenantId::new();
    set_tier(&store, tenant, "free").await;
    assert!(!tier_allows(&store, tenant, "provider_real_keys").await);
}

#[tokio::test]
async fn pro_tier_allows_provider_real_keys() {
    let store = store().await;
    let tenant = TenantId::new();
    set_tier(&store, tenant, "pro").await;
    assert!(tier_allows(&store, tenant, "provider_real_keys").await);
}

#[tokio::test]
async fn enterprise_allows_provider_real_keys() {
    let store = store().await;
    let tenant = TenantId::new();
    set_tier(&store, tenant, "enterprise").await;
    assert!(tier_allows(&store, tenant, "provider_real_keys").await);
}

#[tokio::test]
async fn unknown_tenant_fails_closed_to_free() {
    let store = store().await;
    // No workspace row at all → resolves to "free" → no real keys.
    assert!(!tier_allows(&store, TenantId::new(), "provider_real_keys").await);
    // ...and "free" reports the feature flags it DOES carry.
    let tenant = TenantId::new();
    set_tier(&store, tenant, "free").await;
    assert!(tier_allows(&store, tenant, "provider_mock").await);
}

#[tokio::test]
async fn free_tenant_blocked_at_second_worker() {
    let store = store().await;
    let tenant = TenantId::new();
    set_tier(&store, tenant, "free").await;
    // free max_workers = 1. Zero workers → allowed.
    assert!(check_worker_cap_or_reject(&store, tenant).await.is_none());
    // One worker → already at cap → next hire rejected.
    insert_workers(&store, tenant, 1).await;
    let (status, body) = check_worker_cap_or_reject(&store, tenant)
        .await
        .expect("free at cap → reject");
    assert_eq!(status, "403 Forbidden");
    assert!(body.contains(r#""error":"tier_worker_cap""#), "{body}");
    assert!(body.contains(r#""tier":"free""#), "{body}");
    assert!(body.contains(r#""max_workers":1"#), "{body}");
    assert!(body.contains(r#""tier_upsell_url":"#), "{body}");
}

#[tokio::test]
async fn pro_tenant_allowed_up_to_cap_then_blocked() {
    let store = store().await;
    let tenant = TenantId::new();
    set_tier(&store, tenant, "pro").await;
    // pro max_workers = 5. Four existing → 5th hire allowed.
    insert_workers(&store, tenant, 4).await;
    assert!(check_worker_cap_or_reject(&store, tenant).await.is_none());
    // Fifth existing → 6th hire blocked.
    insert_workers(&store, tenant, 1).await;
    let (status, body) = check_worker_cap_or_reject(&store, tenant)
        .await
        .expect("pro at cap → reject");
    assert_eq!(status, "403 Forbidden");
    assert!(body.contains(r#""max_workers":5"#), "{body}");
}

#[tokio::test]
async fn enterprise_unlimited_workers() {
    let store = store().await;
    let tenant = TenantId::new();
    set_tier(&store, tenant, "enterprise").await;
    insert_workers(&store, tenant, 50).await;
    // max_workers = 0 → unlimited → never rejected.
    assert!(check_worker_cap_or_reject(&store, tenant).await.is_none());
}

/// Tenant isolation: one tenant's workers never count toward another's
/// cap (the count query is `WHERE tenant_id = ?`).
#[tokio::test]
async fn worker_cap_is_tenant_scoped() {
    let store = store().await;
    let free_a = TenantId::new();
    let free_b = TenantId::new();
    set_tier(&store, free_a, "free").await;
    set_tier(&store, free_b, "free").await;
    // Tenant A fills its single slot; tenant B is untouched.
    insert_workers(&store, free_a, 1).await;
    assert!(check_worker_cap_or_reject(&store, free_a).await.is_some());
    assert!(check_worker_cap_or_reject(&store, free_b).await.is_none());
}

/// The fail-closed resolver defaults an unknown tenant to Free (used by
/// `tier_allows` + `check_worker_cap_or_reject`).
#[tokio::test]
async fn tier_resolver_defaults_to_free() {
    let store = store().await;
    let tier = tenant_tier_or_free(&store, TenantId::new()).await;
    assert_eq!(tier.as_str(), "free");
}
