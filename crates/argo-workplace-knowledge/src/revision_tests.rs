use super::{get, list_for_doc};
use crate::doc::{self, KnowledgeCategory};
use argo_store::SessionStore;
use argo_types::TenantId;

async fn fresh_pool() -> sqlx::Pool<sqlx::Sqlite> {
    let store = SessionStore::open(":memory:")
        .await
        .expect("open :memory: store");
    store.migrate().await.expect("apply migrations");
    store.pool().clone()
}

#[tokio::test]
async fn revisions_chained_by_parent_id() {
    let pool = fresh_pool().await;
    let tenant = TenantId::new();
    let head_v1 = doc::create(
        &pool,
        &tenant,
        "policy",
        "Policy",
        KnowledgeCategory::BusinessRules,
        "v1",
        "ops@example.com",
    )
    .await
    .expect("create");
    let head_v2 = doc::update(&pool, &tenant, "policy", "v2", "ops@example.com", None)
        .await
        .expect("update v2");
    let head_v3 = doc::update(&pool, &tenant, "policy", "v3", "ops@example.com", None)
        .await
        .expect("update v3");
    let head_v4 = doc::update(&pool, &tenant, "policy", "v4", "ops@example.com", None)
        .await
        .expect("update v4");

    // The head row's parent_revision_id should reference the latest
    // revision, which itself captures the prior head body.
    let rev_id_for_v4 = head_v4.parent_revision_id.clone().expect("parent set");
    let parent_rev = get(&pool, &tenant, &rev_id_for_v4)
        .await
        .expect("get rev")
        .expect("rev exists");
    assert_eq!(parent_rev.version, 3);
    assert_eq!(parent_rev.body_markdown, "v3");

    // The revision chain (newest → oldest) walks back through the
    // versions captured by each successive update.
    let revs = list_for_doc(&pool, &tenant, &head_v1.id)
        .await
        .expect("list");
    assert_eq!(revs.len(), 3);
    assert_eq!(
        revs.iter().map(|r| r.version).collect::<Vec<_>>(),
        vec![3, 2, 1]
    );
    // The v2 update's parent_revision_id points at the revision row
    // that captured the v1 body.
    let rev_id_for_v2 = head_v2.parent_revision_id.expect("parent v2");
    let rev_for_v2 = get(&pool, &tenant, &rev_id_for_v2)
        .await
        .expect("get rev v2")
        .expect("exists");
    assert_eq!(rev_for_v2.version, 1);
    assert_eq!(rev_for_v2.body_markdown, "v1");
    let _ = head_v3;
}

#[tokio::test]
async fn revision_list_ordered_desc_by_version() {
    let pool = fresh_pool().await;
    let tenant = TenantId::new();
    let head = doc::create(
        &pool,
        &tenant,
        "ordering",
        "T",
        KnowledgeCategory::Brand,
        "body 1",
        "ops@example.com",
    )
    .await
    .expect("create");
    for n in 2..=5 {
        let body = format!("body {n}");
        doc::update(&pool, &tenant, "ordering", &body, "ops@example.com", None)
            .await
            .expect("update");
    }
    let revs = list_for_doc(&pool, &tenant, &head.id)
        .await
        .expect("list");
    assert_eq!(
        revs.iter().map(|r| r.version).collect::<Vec<_>>(),
        vec![4, 3, 2, 1]
    );
}

#[tokio::test]
async fn revision_get_filters_by_tenant() {
    let pool = fresh_pool().await;
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();

    doc::create(
        &pool,
        &tenant_a,
        "cross",
        "T",
        KnowledgeCategory::Internal,
        "v1",
        "ops@example.com",
    )
    .await
    .expect("create");
    let head_v2 = doc::update(&pool, &tenant_a, "cross", "v2", "ops@example.com", None)
        .await
        .expect("update");

    let rev_id = head_v2.parent_revision_id.expect("present");
    let seen_by_a = get(&pool, &tenant_a, &rev_id).await.expect("get");
    assert!(seen_by_a.is_some());
    let seen_by_b = get(&pool, &tenant_b, &rev_id).await.expect("get");
    assert!(
        seen_by_b.is_none(),
        "tenant B must not see tenant A's revision"
    );
}
