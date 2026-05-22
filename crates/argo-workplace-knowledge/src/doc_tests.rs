use super::{create, delete, get, list, max_version_for_tenant, update, KnowledgeCategory};
use crate::revision;
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
async fn create_and_get() {
    let pool = fresh_pool().await;
    let tenant = TenantId::new();
    let head = create(
        &pool,
        &tenant,
        "refund-policy",
        "Refund policy",
        KnowledgeCategory::BusinessRules,
        "Refunds within 30 days.",
        "ops@example.com",
    )
    .await
    .expect("create head");

    assert_eq!(head.version, 1);
    assert_eq!(head.parent_revision_id, None);
    assert_eq!(head.body_markdown, "Refunds within 30 days.");

    let fetched = get(&pool, &tenant, "refund-policy")
        .await
        .expect("get")
        .expect("row present");
    assert_eq!(fetched.id, head.id);
    assert_eq!(fetched.title, "Refund policy");
    assert_eq!(fetched.category, KnowledgeCategory::BusinessRules);
    assert_eq!(fetched.created_by, "ops@example.com");
    assert_eq!(fetched.updated_by, "ops@example.com");
}

#[tokio::test]
async fn update_increments_version_and_writes_revision() {
    let pool = fresh_pool().await;
    let tenant = TenantId::new();
    let head = create(
        &pool,
        &tenant,
        "tone-guide",
        "Tone",
        KnowledgeCategory::Brand,
        "v1 body",
        "brand@example.com",
    )
    .await
    .expect("create");

    let after_first = update(
        &pool,
        &tenant,
        "tone-guide",
        "v2 body",
        "editor@example.com",
        Some("first round of edits"),
    )
    .await
    .expect("update 1");
    assert_eq!(after_first.version, 2);
    assert!(after_first.parent_revision_id.is_some());

    update(
        &pool,
        &tenant,
        "tone-guide",
        "v3 body",
        "editor@example.com",
        None,
    )
    .await
    .expect("update 2");
    let after_third = update(
        &pool,
        &tenant,
        "tone-guide",
        "v4 body",
        "editor@example.com",
        None,
    )
    .await
    .expect("update 3");

    assert_eq!(after_third.version, 4);
    let revs = revision::list_for_doc(&pool, &tenant, &head.id)
        .await
        .expect("list revisions");
    assert_eq!(revs.len(), 3);
    // newest first
    assert_eq!(revs[0].version, 3);
    assert_eq!(revs[1].version, 2);
    assert_eq!(revs[2].version, 1);
    assert_eq!(revs[2].body_markdown, "v1 body");
    assert_eq!(revs[2].edit_reason.as_deref(), Some("first round of edits"));

    // After the third update, the head body should be the newest version.
    let refreshed = get(&pool, &tenant, "tone-guide")
        .await
        .expect("get")
        .expect("present");
    assert_eq!(refreshed.body_markdown, "v4 body");
    assert_eq!(refreshed.version, 4);
    assert_eq!(refreshed.created_by, "brand@example.com");
    assert_eq!(refreshed.updated_by, "editor@example.com");
    // The head's parent_revision_id must point at the newest revision
    // row, which captured the previous head (v3) body.
    assert_eq!(refreshed.parent_revision_id, Some(revs[0].id.clone()));
}

#[tokio::test]
async fn delete_returns_true_only_if_existed() {
    let pool = fresh_pool().await;
    let tenant = TenantId::new();
    create(
        &pool,
        &tenant,
        "slug-a",
        "A",
        KnowledgeCategory::Internal,
        "body",
        "ops@example.com",
    )
    .await
    .expect("create");

    let first = delete(&pool, &tenant, "slug-a").await.expect("delete");
    assert!(first, "first delete must return true");
    let second = delete(&pool, &tenant, "slug-a").await.expect("delete");
    assert!(!second, "second delete must return false");
    let missing = get(&pool, &tenant, "slug-a").await.expect("get");
    assert!(missing.is_none());
}

#[tokio::test]
async fn list_filters_by_tenant() {
    let pool = fresh_pool().await;
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();

    create(
        &pool,
        &tenant_a,
        "policy-a1",
        "A1",
        KnowledgeCategory::BusinessRules,
        "body a1",
        "ops@example.com",
    )
    .await
    .expect("create a1");
    create(
        &pool,
        &tenant_a,
        "policy-a2",
        "A2",
        KnowledgeCategory::Faq,
        "body a2",
        "ops@example.com",
    )
    .await
    .expect("create a2");
    create(
        &pool,
        &tenant_b,
        "policy-b1",
        "B1",
        KnowledgeCategory::Compliance,
        "body b1",
        "ops@example.com",
    )
    .await
    .expect("create b1");

    let list_a = list(&pool, &tenant_a).await.expect("list A");
    let list_b = list(&pool, &tenant_b).await.expect("list B");

    let slugs_a: Vec<&str> = list_a.iter().map(|d| d.slug.as_str()).collect();
    let slugs_b: Vec<&str> = list_b.iter().map(|d| d.slug.as_str()).collect();

    assert_eq!(slugs_a, vec!["policy-a1", "policy-a2"]);
    assert_eq!(slugs_b, vec!["policy-b1"]);
    // No row from the other tenant leaks through.
    assert!(list_a.iter().all(|d| d.tenant_id == tenant_a.to_string()));
    assert!(list_b.iter().all(|d| d.tenant_id == tenant_b.to_string()));
}

#[tokio::test]
async fn get_with_wrong_tenant_returns_none() {
    let pool = fresh_pool().await;
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();

    create(
        &pool,
        &tenant_a,
        "private-runbook",
        "Runbook",
        KnowledgeCategory::Internal,
        "secret",
        "ops@example.com",
    )
    .await
    .expect("create");

    let seen_by_a = get(&pool, &tenant_a, "private-runbook")
        .await
        .expect("get");
    assert!(seen_by_a.is_some());
    let seen_by_b = get(&pool, &tenant_b, "private-runbook")
        .await
        .expect("get");
    assert!(seen_by_b.is_none(), "tenant B must not see tenant A's doc");
}

#[tokio::test]
async fn max_version_for_tenant_returns_zero_when_empty() {
    let pool = fresh_pool().await;
    let tenant = TenantId::new();
    let max = max_version_for_tenant(&pool, &tenant)
        .await
        .expect("max version");
    assert_eq!(max, 0);

    create(
        &pool,
        &tenant,
        "policy",
        "P",
        KnowledgeCategory::BusinessRules,
        "body",
        "ops@example.com",
    )
    .await
    .expect("create");
    let after_create = max_version_for_tenant(&pool, &tenant)
        .await
        .expect("max version");
    assert_eq!(after_create, 1);

    update(
        &pool,
        &tenant,
        "policy",
        "body v2",
        "editor@example.com",
        None,
    )
    .await
    .expect("update");
    let after_update = max_version_for_tenant(&pool, &tenant)
        .await
        .expect("max version");
    assert_eq!(after_update, 2);
}

#[tokio::test]
async fn create_duplicate_slug_rejected_per_tenant() {
    let pool = fresh_pool().await;
    let tenant = TenantId::new();
    create(
        &pool,
        &tenant,
        "slug",
        "T",
        KnowledgeCategory::Brand,
        "body",
        "ops@example.com",
    )
    .await
    .expect("first create");
    let dup = create(
        &pool,
        &tenant,
        "slug",
        "T2",
        KnowledgeCategory::Brand,
        "body2",
        "ops@example.com",
    )
    .await;
    assert!(dup.is_err(), "second create with same slug must fail");
}

#[tokio::test]
async fn update_on_missing_slug_returns_not_found() {
    let pool = fresh_pool().await;
    let tenant = TenantId::new();
    let err = update(
        &pool,
        &tenant,
        "missing-slug",
        "body",
        "editor@example.com",
        None,
    )
    .await
    .expect_err("must fail");
    assert!(
        matches!(err, argo_types::ArgoError::NotFound(_)),
        "expected NotFound, got {err:?}"
    );
}

#[test]
fn category_db_roundtrip_covers_every_variant() {
    let variants = [
        KnowledgeCategory::Brand,
        KnowledgeCategory::BusinessRules,
        KnowledgeCategory::Compliance,
        KnowledgeCategory::Internal,
        KnowledgeCategory::Faq,
    ];
    for cat in variants {
        let s = cat.as_db_str();
        let parsed = KnowledgeCategory::from_db_str(s).expect("roundtrip");
        assert_eq!(parsed, cat);
    }
    assert!(KnowledgeCategory::from_db_str("not-a-real-category").is_none());
}
