//! Integration tests for `argo_store::skill_share`. Cover the
//! acceptance criteria from the brief
//! `claude-skill-share-cross-tenant-poc`:
//! toggle, list-same-cluster, list-different-cluster (empty),
//! unshared-invisible, no-cluster-tenant, migration up/down +
//! idempotence.

use argo_store::skill_share::{list_shared_for_tenant, set_shared};
use argo_store::SessionStore;
use uuid::Uuid;

async fn store() -> SessionStore {
    let s = SessionStore::open(":memory:").await.unwrap();
    s.migrate().await.unwrap();
    s
}

async fn insert_skill(
    store: &SessionStore,
    id: Uuid,
    name: &str,
    version: &str,
    created_at: i64,
) {
    sqlx::query(
        "INSERT INTO skill (id, name, version, body, frontmatter, source, hash, created_at) \
         VALUES (?, ?, ?, '', '', 'inline', randomblob(8), ?)",
    )
    .bind(id.as_bytes().as_slice())
    .bind(name)
    .bind(version)
    .bind(created_at)
    .execute(store.pool())
    .await
    .unwrap();
}

async fn insert_org_unit(
    store: &SessionStore,
    id: &str,
    tenant_id: &str,
    name: &str,
    parent_id: Option<&str>,
    created_at: i64,
) {
    sqlx::query(
        "INSERT INTO org_unit (id, tenant_id, name, parent_id, created_at) \
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(id)
    .bind(tenant_id)
    .bind(name)
    .bind(parent_id)
    .bind(created_at)
    .execute(store.pool())
    .await
    .unwrap();
}

#[tokio::test]
async fn set_shared_toggles_flag_and_persists_cluster() {
    let s = store().await;
    let id = Uuid::now_v7();
    insert_skill(&s, id, "alpha", "1.0", 100).await;

    set_shared(&s, "alpha", "1.0", true, Some("eng")).await.unwrap();

    let row: (i64, Option<String>) = sqlx::query_as(
        "SELECT shared, shared_with_cluster FROM skill WHERE name = ? AND version = ?",
    )
    .bind("alpha")
    .bind("1.0")
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(row.0, 1);
    assert_eq!(row.1.as_deref(), Some("eng"));

    set_shared(&s, "alpha", "1.0", false, None).await.unwrap();
    let row: (i64, Option<String>) = sqlx::query_as(
        "SELECT shared, shared_with_cluster FROM skill WHERE name = ? AND version = ?",
    )
    .bind("alpha")
    .bind("1.0")
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(row.0, 0);
    assert_eq!(row.1, None);
}

#[tokio::test]
async fn list_returns_only_skills_in_same_cluster() {
    let s = store().await;
    insert_org_unit(&s, "u-a", "tenant-a", "eng", None, 1).await;
    insert_org_unit(&s, "u-b", "tenant-b", "eng", None, 1).await;

    let id_a = Uuid::now_v7();
    let id_b = Uuid::now_v7();
    insert_skill(&s, id_a, "alpha", "1.0", 200).await;
    insert_skill(&s, id_b, "beta", "1.0", 201).await;

    set_shared(&s, "alpha", "1.0", true, Some("eng")).await.unwrap();
    set_shared(&s, "beta", "1.0", true, Some("finance")).await.unwrap();

    let rows = list_shared_for_tenant(&s, "tenant-b").await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].name, "alpha");
    assert_eq!(rows[0].shared_with_cluster, "eng");
}

#[tokio::test]
async fn list_returns_empty_when_no_cluster_match() {
    let s = store().await;
    insert_org_unit(&s, "u-c", "tenant-c", "sales", None, 1).await;
    insert_org_unit(&s, "u-d", "tenant-d", "eng", None, 1).await;

    let id = Uuid::now_v7();
    insert_skill(&s, id, "gamma", "1.0", 300).await;
    set_shared(&s, "gamma", "1.0", true, Some("eng")).await.unwrap();

    let rows = list_shared_for_tenant(&s, "tenant-c").await.unwrap();
    assert!(rows.is_empty());
}

#[tokio::test]
async fn unshared_skills_are_never_returned() {
    let s = store().await;
    insert_org_unit(&s, "u-e", "tenant-e", "eng", None, 1).await;

    let id_shared = Uuid::now_v7();
    let id_private = Uuid::now_v7();
    insert_skill(&s, id_shared, "shared-skill", "1.0", 400).await;
    insert_skill(&s, id_private, "private-skill", "1.0", 401).await;

    set_shared(&s, "shared-skill", "1.0", true, Some("eng")).await.unwrap();
    // `private-skill` stays unshared but DOES carry the cluster label,
    // confirming `shared = 0` is the gate, not the cluster column alone.
    sqlx::query("UPDATE skill SET shared_with_cluster = 'eng' WHERE name = 'private-skill'")
        .execute(s.pool())
        .await
        .unwrap();

    let rows = list_shared_for_tenant(&s, "tenant-e").await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].name, "shared-skill");
}

#[tokio::test]
async fn list_returns_empty_for_tenant_without_org_unit() {
    let s = store().await;
    let id = Uuid::now_v7();
    insert_skill(&s, id, "delta", "1.0", 500).await;
    set_shared(&s, "delta", "1.0", true, Some("eng")).await.unwrap();

    let rows = list_shared_for_tenant(&s, "tenant-z").await.unwrap();
    assert!(rows.is_empty());
}

#[tokio::test]
async fn migration_is_idempotent_after_reapply() {
    let s = store().await;
    let v_before = s.schema_version().await.unwrap();
    s.migrate().await.unwrap();
    s.migrate().await.unwrap();
    let v_after = s.schema_version().await.unwrap();
    assert_eq!(v_before, v_after);
    assert!(v_after >= 41, "v41 must be applied (got {v_after})");
}

#[tokio::test]
async fn migration_down_drops_columns() {
    let s = store().await;
    let id = Uuid::now_v7();
    insert_skill(&s, id, "epsilon", "1.0", 600).await;
    set_shared(&s, "epsilon", "1.0", true, Some("eng")).await.unwrap();

    s.migrate_down_to(40).await.unwrap();

    let cols: Vec<(i64, String, String, i64, Option<String>, i64)> = sqlx::query_as(
        "SELECT cid, name, type, \"notnull\", dflt_value, pk FROM pragma_table_info('skill')",
    )
    .fetch_all(s.pool())
    .await
    .unwrap();
    let names: Vec<&str> = cols.iter().map(|c| c.1.as_str()).collect();
    assert!(
        !names.contains(&"shared"),
        "shared column should be dropped, got cols = {names:?}"
    );
    assert!(
        !names.contains(&"shared_with_cluster"),
        "shared_with_cluster column should be dropped, got cols = {names:?}"
    );

    s.migrate().await.unwrap();
    let cols: Vec<(i64, String, String, i64, Option<String>, i64)> = sqlx::query_as(
        "SELECT cid, name, type, \"notnull\", dflt_value, pk FROM pragma_table_info('skill')",
    )
    .fetch_all(s.pool())
    .await
    .unwrap();
    let names: Vec<&str> = cols.iter().map(|c| c.1.as_str()).collect();
    assert!(names.contains(&"shared"));
    assert!(names.contains(&"shared_with_cluster"));
}
