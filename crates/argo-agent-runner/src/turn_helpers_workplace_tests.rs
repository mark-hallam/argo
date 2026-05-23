//! Unit tests for [`turn_helpers_workplace`] — pure-function rendering
//! plus cache behavior under (tenant_id, max_version) keying.
//!
//! Each `#[tokio::test]` clears the process-local cache up-front so
//! tests are order-independent.

use super::{clear_cache_for_tests, render_block, render_workplace_knowledge_block};
use argo_store::SessionStore;
use argo_types::TenantId;
use argo_workplace_knowledge::doc::{self, KnowledgeCategory, WorkplaceKnowledgeDoc};
use std::sync::Arc;

fn make_doc(
    slug: &str,
    title: &str,
    category: KnowledgeCategory,
    body: &str,
    version: i64,
    updated_at_micros: i64,
) -> WorkplaceKnowledgeDoc {
    WorkplaceKnowledgeDoc {
        id: format!("id-{slug}"),
        tenant_id: "tenant-x".into(),
        slug: slug.into(),
        title: title.into(),
        category,
        body_markdown: body.into(),
        version,
        parent_revision_id: None,
        created_at_micros: 0,
        created_by: "ops@example.com".into(),
        updated_at_micros,
        updated_by: "ops@example.com".into(),
    }
}

#[test]
fn render_block_starts_and_ends_with_markers() {
    let docs = vec![make_doc(
        "voice",
        "Voice",
        KnowledgeCategory::Brand,
        "be kind",
        1,
        0,
    )];
    let block = render_block(&docs);
    assert!(block.starts_with("[WORKPLACE KNOWLEDGE — read-only, do not contradict]\n"));
    assert!(block.trim_end().ends_with("[END WORKPLACE KNOWLEDGE]"));
}

#[test]
fn render_block_includes_title_version_body() {
    let docs = vec![make_doc(
        "voice",
        "Voice",
        KnowledgeCategory::Brand,
        "Never say cheap.",
        3,
        0,
    )];
    let block = render_block(&docs);
    assert!(block.contains("## Voice (v3, last updated"));
    assert!(block.contains("Never say cheap."));
}

#[test]
fn render_block_orders_alphabetical_by_category_then_slug() {
    let docs = vec![
        make_doc("zeta", "Zeta", KnowledgeCategory::Faq, "z", 1, 0),
        make_doc("alpha", "Alpha", KnowledgeCategory::Brand, "a", 1, 0),
        make_doc("beta", "Beta", KnowledgeCategory::Brand, "b", 1, 0),
        make_doc(
            "charlie",
            "Charlie",
            KnowledgeCategory::BusinessRules,
            "c",
            1,
            0,
        ),
    ];
    let block = render_block(&docs);
    let positions: Vec<_> = ["## Alpha", "## Beta", "## Charlie", "## Zeta"]
        .into_iter()
        .map(|needle| block.find(needle).unwrap_or(usize::MAX))
        .collect();
    assert!(
        positions.windows(2).all(|w| w[0] < w[1]),
        "expected ascending order; got {positions:?} in block:\n{block}"
    );
}

#[test]
fn render_block_iso_date_present() {
    // 2026-01-15T12:34:56Z → 1768480496 seconds × 1_000_000.
    let micros = 1_768_480_496_000_000i64;
    let docs = vec![make_doc(
        "voice",
        "Voice",
        KnowledgeCategory::Brand,
        "x",
        1,
        micros,
    )];
    let block = render_block(&docs);
    assert!(
        block.contains("2026-01-15"),
        "expected ISO date in block, got:\n{block}"
    );
}

#[tokio::test]
async fn render_empty_tenant_returns_none() {
    clear_cache_for_tests().await;
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    let tenant = TenantId::new();
    let out = render_workplace_knowledge_block(&store, &tenant)
        .await
        .unwrap();
    assert!(out.is_none());
}

#[tokio::test]
async fn render_single_doc_includes_body_byte_for_byte() {
    clear_cache_for_tests().await;
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    let tenant = TenantId::new();
    doc::create(
        store.pool(),
        &tenant,
        "voice",
        "Voice",
        KnowledgeCategory::Brand,
        "Never say cheap.",
        "ops@example.com",
    )
    .await
    .unwrap();
    let out = render_workplace_knowledge_block(&store, &tenant)
        .await
        .unwrap()
        .expect("expected workplace block");
    assert!(
        out.contains("Never say cheap."),
        "expected literal doc body in block; got:\n{out}"
    );
    assert!(out.starts_with("[WORKPLACE KNOWLEDGE — read-only, do not contradict]"));
    assert!(out.trim_end().ends_with("[END WORKPLACE KNOWLEDGE]"));
}

#[tokio::test]
async fn cache_hit_after_warmup() {
    clear_cache_for_tests().await;
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    let tenant = TenantId::new();
    doc::create(
        store.pool(),
        &tenant,
        "voice",
        "Voice",
        KnowledgeCategory::Brand,
        "be kind",
        "ops@example.com",
    )
    .await
    .unwrap();
    let first = render_workplace_knowledge_block(&store, &tenant)
        .await
        .unwrap()
        .unwrap();
    let second = render_workplace_knowledge_block(&store, &tenant)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(first, second, "cache hit must return identical bytes");
}

#[tokio::test]
async fn cache_invalidates_on_version_bump() {
    clear_cache_for_tests().await;
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    let tenant = TenantId::new();
    doc::create(
        store.pool(),
        &tenant,
        "voice",
        "Voice",
        KnowledgeCategory::Brand,
        "be kind",
        "ops@example.com",
    )
    .await
    .unwrap();
    let v1 = render_workplace_knowledge_block(&store, &tenant)
        .await
        .unwrap()
        .unwrap();
    assert!(v1.contains("be kind"));

    doc::update(
        store.pool(),
        &tenant,
        "voice",
        "be assertive",
        "ops@example.com",
        Some("tone shift"),
    )
    .await
    .unwrap();
    let v2 = render_workplace_knowledge_block(&store, &tenant)
        .await
        .unwrap()
        .unwrap();
    assert!(v2.contains("be assertive"));
    assert!(!v2.contains("be kind"));
}

#[tokio::test]
async fn cross_tenant_isolation() {
    clear_cache_for_tests().await;
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();

    doc::create(
        store.pool(),
        &tenant_a,
        "voice",
        "Voice",
        KnowledgeCategory::Brand,
        "tenant-a-secret-body",
        "ops@example.com",
    )
    .await
    .unwrap();
    doc::create(
        store.pool(),
        &tenant_b,
        "voice",
        "Voice",
        KnowledgeCategory::Brand,
        "tenant-b-other-body",
        "ops@example.com",
    )
    .await
    .unwrap();

    let a = render_workplace_knowledge_block(&store, &tenant_a)
        .await
        .unwrap()
        .unwrap();
    let b = render_workplace_knowledge_block(&store, &tenant_b)
        .await
        .unwrap()
        .unwrap();
    assert!(a.contains("tenant-a-secret-body"));
    assert!(!a.contains("tenant-b-other-body"));
    assert!(b.contains("tenant-b-other-body"));
    assert!(!b.contains("tenant-a-secret-body"));
}
