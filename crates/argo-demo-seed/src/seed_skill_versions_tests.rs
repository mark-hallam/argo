//! Tests for the clean version-chain seeder
//! ([`super::seed_skill_version_chain`]).
//!
//! Covers: a single active catalog row becomes a coherent v1→v2→v3 chain
//! (v3 active, v1/v2 superseded); the active head id is preserved (no FK
//! churn); re-seeding is a no-op; the chain is tenant-scoped + its labels
//! align with `argo_skill::list_versions_for_tenant`.

use super::*;
use crate::marketplace::seed_demo_marketplace_skills;
use sqlx::Row;

async fn open_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    Arc::new(store)
}

/// The active (non-superseded) row's id + version for `(name, tenant)`.
async fn active_row(store: &Arc<SessionStore>, tenant: &str, name: &str) -> (Vec<u8>, String) {
    let row = sqlx::query(
        "SELECT id, version FROM skill \
         WHERE name = ? AND tenant_id = ? AND superseded_by IS NULL AND lifecycle_state = 'active'",
    )
    .bind(name)
    .bind(tenant)
    .fetch_one(store.pool())
    .await
    .unwrap();
    (row.get("id"), row.get("version"))
}

#[tokio::test]
async fn builds_a_clean_v1_v2_v3_chain() {
    let store = open_store().await;
    let tenant = TenantId::new();
    seed_demo_marketplace_skills(&store, tenant).await.unwrap();

    let built = seed_skill_version_chain(&store, tenant).await.unwrap();
    assert_eq!(
        built,
        crate::demo_marketplace_catalog_size(),
        "one chain built per catalog skill"
    );

    // Pick the Scene-4 headline skill and assert its timeline is exactly
    // v1 (superseded) → v2 (superseded) → v3 (active head).
    let name = "pricing.competitor_scrape";
    let versions = argo_skill::list_versions_for_tenant(&store, name, &tenant.to_string())
        .await
        .unwrap();
    assert_eq!(versions.len(), 3, "exactly three versions: {versions:?}");
    // Newest-first ordering.
    assert_eq!(versions[0].version, "v3");
    assert_eq!(versions[1].version, "v2");
    assert_eq!(versions[2].version, "v1");
    // Exactly one active (non-superseded) row, and it's v3.
    assert!(!versions[0].superseded, "v3 is the active head");
    assert!(versions[1].superseded, "v2 is superseded");
    assert!(versions[2].superseded, "v1 is superseded");
    let active_count = versions.iter().filter(|v| !v.superseded).count();
    assert_eq!(active_count, 1, "exactly one active version in the chain");
}

#[tokio::test]
async fn preserves_the_active_head_id() {
    // The relabel-in-place design must keep the active row's id stable so
    // worker_skill / skill_ir references stay valid.
    let store = open_store().await;
    let tenant = TenantId::new();
    seed_demo_marketplace_skills(&store, tenant).await.unwrap();
    let name = "pricing.competitor_scrape";
    let (id_before, ver_before) = active_row(&store, &tenant.to_string(), name).await;
    assert_eq!(ver_before, "0.1.0", "marketplace installs at 0.1.0");

    seed_skill_version_chain(&store, tenant).await.unwrap();
    let (id_after, ver_after) = active_row(&store, &tenant.to_string(), name).await;
    assert_eq!(id_after, id_before, "active row id must be preserved");
    assert_eq!(ver_after, "v3", "the active head is relabelled in place to v3");
}

#[tokio::test]
async fn reseed_is_a_noop() {
    let store = open_store().await;
    let tenant = TenantId::new();
    seed_demo_marketplace_skills(&store, tenant).await.unwrap();
    let first = seed_skill_version_chain(&store, tenant).await.unwrap();
    let second = seed_skill_version_chain(&store, tenant).await.unwrap();
    assert_eq!(first, crate::demo_marketplace_catalog_size());
    assert_eq!(second, 0, "re-seed must not rebuild an existing chain");

    // Still exactly three versions for the headline skill (no stacking).
    let versions =
        argo_skill::list_versions_for_tenant(&store, "pricing.competitor_scrape", &tenant.to_string())
            .await
            .unwrap();
    assert_eq!(versions.len(), 3, "re-seed must not stack extra versions");
}

#[tokio::test]
async fn chain_is_tenant_scoped() {
    // Building tenant A's chain leaves tenant B untouched.
    let store = open_store().await;
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    seed_demo_marketplace_skills(&store, tenant_a).await.unwrap();
    seed_demo_marketplace_skills(&store, tenant_b).await.unwrap();

    seed_skill_version_chain(&store, tenant_a).await.unwrap();

    // Tenant A has the 3-version chain; tenant B still has its single
    // untouched 0.1.0 row.
    let a = argo_skill::list_versions_for_tenant(
        &store,
        "pricing.competitor_scrape",
        &tenant_a.to_string(),
    )
    .await
    .unwrap();
    assert_eq!(a.len(), 3);
    let b = argo_skill::list_versions_for_tenant(
        &store,
        "pricing.competitor_scrape",
        &tenant_b.to_string(),
    )
    .await
    .unwrap();
    assert_eq!(b.len(), 1, "tenant B's chain must be untouched");
    assert_eq!(b[0].version, "0.1.0");
}

#[tokio::test]
async fn empty_tenant_builds_nothing() {
    let store = open_store().await;
    let tenant = TenantId::new();
    // No catalog seeded — nothing to chain.
    let built = seed_skill_version_chain(&store, tenant).await.unwrap();
    assert_eq!(built, 0);
}
