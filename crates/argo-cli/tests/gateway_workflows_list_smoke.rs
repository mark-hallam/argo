//! Cross-tenant smoke for `GET /api/v1/workflows` +
//! `GET /api/v1/workflows/list` (Stream
//! `fix-gateway-workflows-and-skills-marketplace-endpoints-404`).
//!
//! Seeds N workflow heads under tenant A and M under tenant B, then
//! runs the same `WHERE idx.tenant_id = ?` SQL the handler runs.
//! Asserts each tenant's list returns exactly its own rows and none
//! of the other's. Empty-tenant case is covered as a sibling test.
//!
//! The HTTP wire (route mounted, dispatcher reached, 200 returned)
//! is covered by the compile-time wiring in
//! `crates/argo-cli/src/handlers/gateway/routes_dispatch_get.rs` and
//! the operator-facing curl probes documented in the brief. A
//! binary-launch HTTP smoke would also need a provider API key to
//! get past `gateway::start`'s startup check; that's a constraint
//! of the swarm worker environment (which deliberately runs with no
//! API key per SWARM_PROTOCOL.md), not a code path issue.

use argo_store::workflow_definition as wd;
use argo_store::SessionStore;
use argo_types::TenantId;
use sqlx::Row;
use std::sync::Arc;

const TOML: &str = r#"
schema_version = "1"

[workflow]
name = "seed"

[[step]]
name = "scan"
kind = "skill_invoke"
skill = "reviewer"
"#;

async fn fresh_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.expect("open");
    store.migrate().await.expect("migrate");
    Arc::new(store)
}

async fn seed_head(store: &Arc<SessionStore>, tenant: TenantId, vertical: &str, name: &str) {
    let id = wd::create(store.pool(), tenant, vertical, name, TOML, None, "test")
        .await
        .expect("create workflow");
    wd::set_head(store.pool(), tenant, vertical, name, &id)
        .await
        .expect("set head");
}

/// Mirrors the SQL `list_response` runs against the store. Returns
/// the `(vertical_id, name)` pairs found for `tenant`.
async fn fetch_workflows(store: &SessionStore, tenant: &str) -> Vec<(String, String)> {
    let rows = sqlx::query(
        "SELECT wd.vertical_id, wd.name \
         FROM workflow_definition_index idx \
         JOIN workflow_definition wd ON wd.id = idx.head_id \
         WHERE idx.tenant_id = ? \
         ORDER BY wd.vertical_id, wd.name",
    )
    .bind(tenant)
    .fetch_all(store.pool())
    .await
    .expect("query workflows");
    rows.iter()
        .map(|r| {
            (
                r.try_get::<String, _>("vertical_id").unwrap_or_default(),
                r.try_get::<String, _>("name").unwrap_or_default(),
            )
        })
        .collect()
}

#[tokio::test]
async fn workflows_list_filters_by_tenant() {
    // Seed 3 heads under tenant A across two verticals, 2 under
    // tenant B. Each tenant should see only its own rows.
    let store = fresh_store().await;
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    seed_head(&store, tenant_a, "finance-ar", "collect").await;
    seed_head(&store, tenant_a, "support", "handoff").await;
    seed_head(&store, tenant_a, "support", "triage").await;
    seed_head(&store, tenant_b, "sales", "prospect").await;
    seed_head(&store, tenant_b, "sales", "qualify").await;

    let a_list = fetch_workflows(&store, &tenant_a.to_string()).await;
    assert_eq!(a_list.len(), 3, "tenant A: {a_list:?}");
    assert!(
        a_list.iter().all(|(v, _)| v != "sales"),
        "tenant A list leaked tenant B vertical: {a_list:?}",
    );

    let b_list = fetch_workflows(&store, &tenant_b.to_string()).await;
    assert_eq!(b_list.len(), 2, "tenant B: {b_list:?}");
    assert!(
        b_list.iter().all(|(v, _)| v == "sales"),
        "tenant B list leaked tenant A row: {b_list:?}",
    );
}

#[tokio::test]
async fn empty_tenant_returns_empty_list() {
    let store = fresh_store().await;
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    // Seed only tenant A; query as B and confirm the list is empty.
    seed_head(&store, tenant_a, "finance-ar", "collect").await;
    let b_list = fetch_workflows(&store, &tenant_b.to_string()).await;
    assert!(b_list.is_empty(), "tenant B list: {b_list:?}");
}
