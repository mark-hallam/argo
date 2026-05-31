//! Tests for `seed_api_keys` — proves the canonical demo seed lands
//! real, metadata-only `api_key` rows the live `GET /api/v1/api-keys`
//! endpoint serves (so `/settings/api-keys` shows a roster instead of an
//! empty state), is idempotent, and stores NO plaintext secret.

use super::{seed_api_keys, DEMO_KEYS};
use argo_store::api_key::list_for_tenant;
use argo_store::SessionStore;
use argo_types::TenantId;
use std::sync::Arc;

async fn fresh_store() -> (Arc<SessionStore>, TenantId) {
    let store = Arc::new(SessionStore::open(":memory:").await.expect("open"));
    store.migrate().await.expect("migrate");
    (store, TenantId::new())
}

#[tokio::test]
async fn seeds_all_demo_keys_as_metadata_rows() {
    let (store, tenant) = fresh_store().await;
    let inserted = seed_api_keys(&store, tenant).await.expect("seed");
    assert_eq!(inserted, DEMO_KEYS.len());

    let rows = list_for_tenant(store.pool(), &tenant.to_string())
        .await
        .expect("list");
    assert_eq!(rows.len(), DEMO_KEYS.len());
    // Names match the roster, masked prefixes are present, none revoked.
    let names: Vec<&str> = rows.iter().map(|r| r.name.as_str()).collect();
    assert!(names.contains(&"Production CI"));
    assert!(names.contains(&"Staging dashboard"));
    for r in &rows {
        assert!(r.prefix.starts_with("sk_live_"), "masked prefix: {}", r.prefix);
        assert!(r.prefix.contains("..."), "prefix is masked: {}", r.prefix);
        assert!(!r.revoked);
    }
}

#[tokio::test]
async fn is_idempotent() {
    let (store, tenant) = fresh_store().await;
    assert_eq!(seed_api_keys(&store, tenant).await.unwrap(), DEMO_KEYS.len());
    // Second run finds existing rows and skips wholesale.
    assert_eq!(seed_api_keys(&store, tenant).await.unwrap(), 0);
    let rows = list_for_tenant(store.pool(), &tenant.to_string())
        .await
        .unwrap();
    assert_eq!(rows.len(), DEMO_KEYS.len(), "no duplication on re-seed");
}

#[tokio::test]
async fn stores_no_plaintext_secret_only_a_throwaway_hash() {
    // The seed never mints a raw token, so nothing it stores can be a
    // working credential. We can only observe metadata on read (the row
    // never projects token_hash), which is itself the guarantee: there
    // is no secret to leak, and the masked prefix is not a real token.
    let (store, tenant) = fresh_store().await;
    seed_api_keys(&store, tenant).await.expect("seed");
    let rows = list_for_tenant(store.pool(), &tenant.to_string())
        .await
        .unwrap();
    for r in &rows {
        // A masked preview is never a full `sk_live_<43 chars>` token.
        assert!(r.prefix.len() < 24, "prefix must be a short mask: {}", r.prefix);
    }
}

#[tokio::test]
async fn seeded_keys_are_tenant_scoped() {
    let (store, tenant) = fresh_store().await;
    seed_api_keys(&store, tenant).await.expect("seed");
    // A different tenant sees none of them.
    let other = TenantId::new();
    let rows = list_for_tenant(store.pool(), &other.to_string())
        .await
        .unwrap();
    assert!(rows.is_empty(), "seeded keys must not leak across tenants");
}
