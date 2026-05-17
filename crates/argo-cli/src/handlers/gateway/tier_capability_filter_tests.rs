//! Unit tests for the per-tier MCP-connector allowlist filter.

use super::{check_tier_connector_or_reject, normalise_connector_slug, tier_allows_connector};
use argo_identity::tier::Tier;
use argo_store::{tenant_query, SessionStore};
use argo_types::TenantId;
use std::sync::Arc;

async fn store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.expect("open");
    store.migrate().await.expect("migrate");
    Arc::new(store)
}

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

#[test]
fn normalise_strips_crate_prefix_and_lowercases() {
    assert_eq!(normalise_connector_slug("Gmail"), "gmail");
    assert_eq!(
        normalise_connector_slug("argo-mcp-connector-hubspot"),
        "hubspot"
    );
    assert_eq!(normalise_connector_slug("SLACK"), "slack");
}

#[test]
fn free_tier_allows_only_gmail_slack_pure() {
    assert!(tier_allows_connector(Tier::Free, "gmail"));
    assert!(tier_allows_connector(Tier::Free, "slack"));
    assert!(tier_allows_connector(Tier::Free, "argo-mcp-connector-gmail"));
    assert!(!tier_allows_connector(Tier::Free, "hubspot"));
    assert!(!tier_allows_connector(Tier::Free, "salesforce"));
}

#[test]
fn pro_and_enterprise_allow_every_connector_pure() {
    for tier in [Tier::Pro, Tier::Enterprise] {
        for slug in ["gmail", "slack", "hubspot", "salesforce", "notion"] {
            assert!(tier_allows_connector(tier, slug), "{tier:?} / {slug}");
        }
    }
}

#[tokio::test]
async fn free_tenant_hubspot_returns_403_with_upsell_url() {
    let store = store().await;
    let tenant = TenantId::new();
    set_tier(&store, tenant, "free").await;
    let res = check_tier_connector_or_reject(&store, tenant, "hubspot").await;
    let (status, body) = res.expect("free → hubspot should deny");
    assert_eq!(status, "403 Forbidden");
    assert!(body.contains(r#""error":"tier_forbidden""#), "{body}");
    assert!(body.contains(r#""tier":"free""#), "{body}");
    assert!(body.contains(r#""connector":"hubspot""#), "{body}");
    assert!(body.contains(r#""tier_upsell_url":"#), "{body}");
}

#[tokio::test]
async fn free_tenant_gmail_passes() {
    let store = store().await;
    let tenant = TenantId::new();
    set_tier(&store, tenant, "free").await;
    let res = check_tier_connector_or_reject(&store, tenant, "gmail").await;
    assert!(res.is_none());
}

#[tokio::test]
async fn pro_tenant_hubspot_passes() {
    let store = store().await;
    let tenant = TenantId::new();
    set_tier(&store, tenant, "pro").await;
    let res = check_tier_connector_or_reject(&store, tenant, "hubspot").await;
    assert!(res.is_none());
}

#[tokio::test]
async fn unknown_tier_collapses_to_free_fail_closed() {
    let store = store().await;
    let tenant = TenantId::new();
    set_tier(&store, tenant, "platinum").await;
    let res = check_tier_connector_or_reject(&store, tenant, "hubspot").await;
    let (status, _) = res.expect("unknown tier should fail-closed to free");
    assert_eq!(status, "403 Forbidden");
}
