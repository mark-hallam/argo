//! Tests for `seed_knowledge` — proves the canonical demo seed lands
//! real `workplace_knowledge_doc` rows the live `GET /api/v1/knowledge`
//! endpoint serves (so /settings/knowledge lists real handbook pages),
//! and proves per-doc idempotency.

use super::{seed_knowledge, DEMO_DOCS};
use argo_store::SessionStore;
use argo_types::TenantId;
use argo_workplace_knowledge::doc;
use std::sync::Arc;

async fn fresh_store() -> (Arc<SessionStore>, TenantId) {
    let store = Arc::new(SessionStore::open(":memory:").await.expect("open"));
    store.migrate().await.expect("migrate");
    (store, TenantId::new())
}

#[tokio::test]
async fn seeds_all_demo_docs() {
    let (store, tenant) = fresh_store().await;
    let inserted = seed_knowledge(&store, tenant).await.expect("seed");
    assert_eq!(inserted, DEMO_DOCS.len());

    // The page lists docs via doc::list (the same call the GET handler
    // makes); every seeded slug must come back, tenant-scoped.
    let listed = doc::list(store.pool(), &tenant).await.expect("list");
    assert_eq!(listed.len(), DEMO_DOCS.len());
    for d in DEMO_DOCS {
        assert!(
            listed.iter().any(|row| row.slug == d.slug),
            "seeded doc '{}' must be listed",
            d.slug
        );
    }
}

#[tokio::test]
async fn seeded_docs_carry_title_category_and_body() {
    let (store, tenant) = fresh_store().await;
    seed_knowledge(&store, tenant).await.expect("seed");
    for d in DEMO_DOCS {
        let head = doc::get(store.pool(), &tenant, d.slug)
            .await
            .expect("get")
            .expect("doc present");
        assert_eq!(head.title, d.title);
        assert_eq!(head.category, d.category);
        assert!(!head.body_markdown.is_empty(), "body must not be empty");
        assert_eq!(head.version, 1, "fresh docs start at version 1");
    }
}

#[tokio::test]
async fn docs_are_tenant_scoped() {
    // A different tenant must not see this tenant's handbook (the
    // substrate binds tenant_id into every WHERE clause).
    let (store, tenant) = fresh_store().await;
    seed_knowledge(&store, tenant).await.expect("seed");
    let other = TenantId::new();
    let listed_other = doc::list(store.pool(), &other).await.expect("list other");
    assert!(listed_other.is_empty(), "cross-tenant handbook must be invisible");
}

#[tokio::test]
async fn seed_is_idempotent() {
    let (store, tenant) = fresh_store().await;
    let first = seed_knowledge(&store, tenant).await.expect("first");
    let second = seed_knowledge(&store, tenant).await.expect("second");
    assert!(first > 0);
    assert_eq!(second, 0, "re-seed must not duplicate or error on UNIQUE(slug)");

    let listed = doc::list(store.pool(), &tenant).await.expect("list");
    assert_eq!(listed.len(), first, "doc count stable after re-seed");
}
