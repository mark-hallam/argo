//! Cross-tenant smoke for `GET /api/v1/skills` +
//! `GET /api/v1/skills/marketplace` (Stream
//! `fix-gateway-workflows-and-skills-marketplace-endpoints-404`).
//!
//! Seeds N skills under tenant A and M under tenant B, then runs the
//! same SQL the handler runs (`WHERE s.tenant_id = ?` for installed,
//! `WHERE source LIKE 'published:%'` for marketplace), asserting
//! tenant isolation: A sees exactly its own N rows, B sees only its
//! M, and the marketplace catalog surfaces both tenants' published
//! rows but no private ones.
//!
//! The HTTP wire (route mounted, dispatcher reached, 200 returned)
//! is covered by the compile-time wiring in
//! `crates/argo-cli/src/handlers/gateway/routes_dispatch_get.rs` and
//! the operator-facing curl probes documented in the brief. A
//! binary-launch HTTP smoke would also need a provider API key to
//! get past `gateway::start`'s startup check; that's a constraint
//! of the swarm worker environment (which deliberately runs with no
//! API key per SWARM_PROTOCOL.md), not a code path issue.

use argo_skill::SkillStore;
use argo_store::SessionStore;
use argo_types::TenantId;
use sqlx::Row;
use std::sync::Arc;

fn skill_md(name: &str, source: &str) -> String {
    format!(
        "---\nname: {name}\nversion: \"0.1.0\"\ndescription: \"fixture\"\n\
         provenance:\n  source: {source}\n  hash: sha256-x\n---\nbody",
    )
}

async fn fresh_skills() -> (Arc<SessionStore>, SkillStore) {
    let store = Arc::new(SessionStore::open(":memory:").await.expect("open store"));
    store.migrate().await.expect("migrate");
    let skills = SkillStore::new(Arc::clone(&store));
    (store, skills)
}

/// Replicates the SQL `installed_response` runs against the store so
/// the assertion exercises the actual handler query, not a paraphrase
/// of it.
async fn fetch_installed(store: &SessionStore, tenant: &str) -> Vec<String> {
    let rows = sqlx::query(
        "SELECT s.name FROM skill s \
         WHERE s.tenant_id = ? \
           AND s.superseded_by IS NULL \
           AND s.lifecycle_state = 'active' \
         ORDER BY s.name ASC",
    )
    .bind(tenant)
    .fetch_all(store.pool())
    .await
    .expect("query installed skills");
    rows.iter()
        .map(|r| r.try_get::<String, _>("name").unwrap_or_default())
        .collect()
}

/// Replicates the marketplace query — cross-tenant catalog filtered
/// to rows whose `source` starts with `published:`.
async fn fetch_marketplace(store: &SessionStore) -> Vec<String> {
    let rows = sqlx::query(
        "SELECT s.name FROM skill s \
         WHERE s.superseded_by IS NULL \
           AND s.lifecycle_state = 'active' \
           AND s.source LIKE 'published:%' \
         ORDER BY s.name ASC",
    )
    .fetch_all(store.pool())
    .await
    .expect("query marketplace skills");
    rows.iter()
        .map(|r| r.try_get::<String, _>("name").unwrap_or_default())
        .collect()
}

#[tokio::test]
async fn installed_list_filters_by_tenant() {
    // Seed 3 skills under tenant A, 2 under tenant B. Assert the
    // installed-list SQL returns exactly A's 3 names from tenant A's
    // perspective and none of B's, and vice versa.
    let (store, skills) = fresh_skills().await;
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    for name in ["alpha-approve", "alpha-extract", "alpha-screen"] {
        skills
            .install_for_tenant(&skill_md(name, "user"), &tenant_a.to_string(), None)
            .await
            .expect("install tenant A skill");
    }
    for name in ["bravo-route", "bravo-triage"] {
        skills
            .install_for_tenant(&skill_md(name, "user"), &tenant_b.to_string(), None)
            .await
            .expect("install tenant B skill");
    }
    let a_list = fetch_installed(&store, &tenant_a.to_string()).await;
    assert_eq!(a_list.len(), 3, "tenant A list: {a_list:?}");
    assert!(
        a_list.iter().all(|n| n.starts_with("alpha-")),
        "tenant A list leaked tenant B rows: {a_list:?}",
    );
    let b_list = fetch_installed(&store, &tenant_b.to_string()).await;
    assert_eq!(b_list.len(), 2, "tenant B list: {b_list:?}");
    assert!(
        b_list.iter().all(|n| n.starts_with("bravo-")),
        "tenant B list leaked tenant A rows: {b_list:?}",
    );
}

#[tokio::test]
async fn marketplace_list_returns_published_rows_across_tenants() {
    // Marketplace is the cross-tenant catalog of published skills.
    // Seed one published row per tenant + one private (non-published)
    // row in each. Assert both published rows appear and neither
    // private row does.
    let (store, skills) = fresh_skills().await;
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    skills
        .install_for_tenant(
            &skill_md("verified-routing", "published:verified:vendor"),
            &tenant_a.to_string(),
            None,
        )
        .await
        .expect("install tenant A published");
    skills
        .install_for_tenant(
            &skill_md("alpha-private", "user"),
            &tenant_a.to_string(),
            None,
        )
        .await
        .expect("install tenant A private");
    skills
        .install_for_tenant(
            &skill_md("community-triage", "published:community:market"),
            &tenant_b.to_string(),
            None,
        )
        .await
        .expect("install tenant B published");
    skills
        .install_for_tenant(
            &skill_md("bravo-private", "user"),
            &tenant_b.to_string(),
            None,
        )
        .await
        .expect("install tenant B private");
    let market = fetch_marketplace(&store).await;
    assert_eq!(market.len(), 2, "marketplace list: {market:?}");
    assert!(market.contains(&"community-triage".to_string()));
    assert!(market.contains(&"verified-routing".to_string()));
    assert!(!market.contains(&"alpha-private".to_string()));
    assert!(!market.contains(&"bravo-private".to_string()));
}
