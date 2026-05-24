//! Regression pins for tenant-bound skill share writes.

use argo_store::skill_share::{set_shared, set_shared_for_tenant};
use argo_store::SessionStore;
use uuid::Uuid;

const PLACEHOLDER_TENANT: &str = "__migration_placeholder__";

async fn store() -> SessionStore {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    store
}

async fn insert_skill(store: &SessionStore, tenant_id: &str, name: &str, version: &str) {
    sqlx::query(
        "INSERT INTO skill \
         (id, name, version, body, frontmatter, source, hash, created_at, tenant_id) \
         VALUES (?, ?, ?, '', '', 'inline', randomblob(8), 1, ?)",
    )
    .bind(Uuid::now_v7().as_bytes().as_slice())
    .bind(name)
    .bind(version)
    .bind(tenant_id)
    .execute(store.pool())
    .await
    .unwrap();
}

async fn share_state(store: &SessionStore, tenant_id: &str, name: &str) -> (i64, Option<String>) {
    sqlx::query_as("SELECT shared, shared_with_cluster FROM skill WHERE tenant_id = ? AND name = ?")
        .bind(tenant_id)
        .bind(name)
        .fetch_one(store.pool())
        .await
        .unwrap()
}

#[tokio::test]
async fn tenant_scoped_share_does_not_mutate_another_tenant_skill() {
    let store = store().await;
    insert_skill(&store, "tenant-a", "payroll", "1.0.0").await;

    set_shared_for_tenant(
        &store,
        "tenant-b",
        "payroll",
        "1.0.0",
        true,
        Some("b-cluster"),
    )
    .await
    .unwrap();

    assert_eq!(share_state(&store, "tenant-a", "payroll").await, (0, None));
}

#[tokio::test]
async fn tenant_scoped_share_updates_matching_tenant_skill() {
    let store = store().await;
    insert_skill(&store, "tenant-a", "payroll", "1.0.0").await;

    set_shared_for_tenant(
        &store,
        "tenant-a",
        "payroll",
        "1.0.0",
        true,
        Some("a-cluster"),
    )
    .await
    .unwrap();

    assert_eq!(
        share_state(&store, "tenant-a", "payroll").await,
        (1, Some("a-cluster".to_string()))
    );
}

#[tokio::test]
async fn legacy_wrapper_only_targets_placeholder_tenant_rows() {
    let store = store().await;
    insert_skill(&store, "tenant-a", "tenant-owned", "1.0.0").await;
    insert_skill(&store, PLACEHOLDER_TENANT, "legacy-owned", "1.0.0").await;

    set_shared(&store, "tenant-owned", "1.0.0", true, Some("cluster"))
        .await
        .unwrap();
    set_shared(
        &store,
        "legacy-owned",
        "1.0.0",
        true,
        Some("legacy-cluster"),
    )
    .await
    .unwrap();

    assert_eq!(
        share_state(&store, "tenant-a", "tenant-owned").await,
        (0, None)
    );
    assert_eq!(
        share_state(&store, PLACEHOLDER_TENANT, "legacy-owned").await,
        (1, Some("legacy-cluster".to_string()))
    );
}
