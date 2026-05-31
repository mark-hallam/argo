//! Tests for `seed_notifications` — proves the canonical demo seed
//! lands real broadcast `notifications` rows the live
//! `GET /api/v1/notifications` endpoint serves (so the bell shows real
//! rows and the SPA `SAMPLE_NOTIFICATIONS` fallback never fires), and
//! proves the seed is idempotent.

use super::{seed_notifications, DEMO_NOTIFICATIONS};
use argo_store::notifications::{list_for_principal, unread_count, ListQuery};
use argo_store::SessionStore;
use argo_types::TenantId;
use std::sync::Arc;

async fn fresh_store() -> (Arc<SessionStore>, TenantId) {
    let store = Arc::new(SessionStore::open(":memory:").await.expect("open"));
    store.migrate().await.expect("migrate");
    (store, TenantId::new())
}

#[tokio::test]
async fn seeds_all_demo_notifications_as_broadcast() {
    let (store, tenant) = fresh_store().await;
    let inserted = seed_notifications(&store, tenant).await.expect("seed");
    assert_eq!(inserted, DEMO_NOTIFICATIONS.len());

    // Broadcast rows are visible to ANY principal (and to a None
    // principal) — the model the demo principal (resolved from the JWT
    // at request time) relies on.
    let tenant_str = tenant.to_string();
    let page = list_for_principal(store.pool(), &tenant_str, None, &ListQuery::default())
        .await
        .expect("list");
    assert_eq!(page.items.len(), DEMO_NOTIFICATIONS.len());
    // A specific (unknown-at-seed-time) principal still sees every row.
    let as_human = list_for_principal(
        store.pool(),
        &tenant_str,
        Some("human:operator@example.com"),
        &ListQuery::default(),
    )
    .await
    .expect("list as human");
    assert_eq!(as_human.items.len(), DEMO_NOTIFICATIONS.len());
}

#[tokio::test]
async fn unread_count_is_nonzero_but_not_all() {
    // The bell badge reads `unread_count`. Some rows are seeded read
    // (older ones) so the badge shows a believable count, not "all".
    let (store, tenant) = fresh_store().await;
    seed_notifications(&store, tenant).await.expect("seed");
    let tenant_str = tenant.to_string();
    let unread = unread_count(store.pool(), &tenant_str, None)
        .await
        .expect("unread");
    let expected_unread = DEMO_NOTIFICATIONS.iter().filter(|n| !n.read).count() as u32;
    assert_eq!(unread, expected_unread);
    assert!(unread > 0, "badge must show at least one unread");
    assert!(
        unread < DEMO_NOTIFICATIONS.len() as u32,
        "some rows seeded read so the count is believable"
    );
}

#[tokio::test]
async fn wire_json_carries_a_title_for_the_popover() {
    // The popover renders `title` (the gateway maps `body` → `title`);
    // every seeded row must have a non-empty headline.
    let (store, tenant) = fresh_store().await;
    seed_notifications(&store, tenant).await.expect("seed");
    let tenant_str = tenant.to_string();
    let page = list_for_principal(store.pool(), &tenant_str, None, &ListQuery::default())
        .await
        .expect("list");
    for row in &page.items {
        let wire = row.to_wire_json();
        assert!(
            wire.get("title").and_then(|t| t.as_str()).is_some_and(|s| !s.is_empty()),
            "every notification needs a non-empty title for the popover"
        );
    }
}

#[tokio::test]
async fn seed_is_idempotent() {
    let (store, tenant) = fresh_store().await;
    let first = seed_notifications(&store, tenant).await.expect("first");
    let second = seed_notifications(&store, tenant).await.expect("second");
    assert!(first > 0);
    assert_eq!(second, 0, "re-seed must not duplicate the bell");

    let tenant_str = tenant.to_string();
    let page = list_for_principal(store.pool(), &tenant_str, None, &ListQuery::default())
        .await
        .expect("list");
    assert_eq!(page.items.len(), first, "row count stable after re-seed");
}
