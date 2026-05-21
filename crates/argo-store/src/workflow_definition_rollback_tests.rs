//! Unit coverage for [`super::workflow_definition::rollback_to`] —
//! split out from `workflow_definition_tests.rs` to keep both files
//! under the 500-LOC ceiling (HARD RULE 6). Same `#[path]`-included
//! pattern as the parent test file.

use super::*;
use crate::SessionStore;
use argo_types::ArgoError;
use sqlx::{Pool, Sqlite};

async fn fresh_pool() -> Pool<Sqlite> {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    store.pool().clone()
}

fn sample_toml(name: &str) -> String {
    format!(
        "schema_version = \"1\"\n\n\
         [workflow]\n\
         name = \"{name}\"\n\n\
         [[step]]\n\
         name = \"scan\"\n\
         kind = \"skill_invoke\"\n\
         skill = \"do_a_thing\"\n"
    )
}

#[tokio::test]
async fn rollback_to_creates_new_version_with_target_toml() {
    let pool = fresh_pool().await;
    let tenant = TenantId::new();
    let toml_v1 = sample_toml("flow_v1");
    let toml_v2 = sample_toml("flow_v2");
    let v1 = create(&pool, tenant, "ops", "flow", &toml_v1, None, "user:a")
        .await
        .unwrap();
    set_head(&pool, tenant, "ops", "flow", &v1).await.unwrap();
    let v2 = create(
        &pool,
        tenant,
        "ops",
        "flow",
        &toml_v2,
        Some(v1.clone()),
        "user:a",
    )
    .await
    .unwrap();
    set_head(&pool, tenant, "ops", "flow", &v2).await.unwrap();

    let (new_id, new_version) = rollback_to(&pool, tenant, "ops", "flow", &v1, "user:b")
        .await
        .unwrap();
    assert_eq!(new_version, 3);
    let row = get(&pool, &new_id).await.unwrap().expect("row exists");
    assert_eq!(row.definition_toml, toml_v1, "new row carries v1's TOML");
    assert_eq!(row.lifecycle_state, WorkflowLifecycleState::Active);
    assert_eq!(row.modified_by, "user:b");
}

#[tokio::test]
async fn rollback_to_sets_parent_id_to_target() {
    let pool = fresh_pool().await;
    let tenant = TenantId::new();
    let toml = sample_toml("flow");
    let v1 = create(&pool, tenant, "ops", "flow", &toml, None, "user:a")
        .await
        .unwrap();
    set_head(&pool, tenant, "ops", "flow", &v1).await.unwrap();
    let v2 = create(
        &pool,
        tenant,
        "ops",
        "flow",
        &toml,
        Some(v1.clone()),
        "user:a",
    )
    .await
    .unwrap();
    set_head(&pool, tenant, "ops", "flow", &v2).await.unwrap();

    let (new_id, _) = rollback_to(&pool, tenant, "ops", "flow", &v1, "user:b")
        .await
        .unwrap();
    let row = get(&pool, &new_id).await.unwrap().unwrap();
    assert_eq!(row.parent_id, Some(v1), "new row's parent_id == rollback target");
}

#[tokio::test]
async fn rollback_to_flips_head_to_new_row() {
    let pool = fresh_pool().await;
    let tenant = TenantId::new();
    let toml = sample_toml("flow");
    let v1 = create(&pool, tenant, "ops", "flow", &toml, None, "user:a")
        .await
        .unwrap();
    set_head(&pool, tenant, "ops", "flow", &v1).await.unwrap();
    let v2 = create(
        &pool,
        tenant,
        "ops",
        "flow",
        &toml,
        Some(v1.clone()),
        "user:a",
    )
    .await
    .unwrap();
    set_head(&pool, tenant, "ops", "flow", &v2).await.unwrap();

    let (new_id, _) = rollback_to(&pool, tenant, "ops", "flow", &v1, "user:b")
        .await
        .unwrap();
    let head = get_head(&pool, tenant, "ops", "flow")
        .await
        .unwrap()
        .expect("head must be set");
    assert_eq!(head.id, new_id, "head now points to the new (rolled-back) row");
}

#[tokio::test]
async fn rollback_to_cross_tenant_returns_not_found() {
    let pool = fresh_pool().await;
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    let toml = sample_toml("flow");
    let v1 = create(&pool, tenant_a, "ops", "flow", &toml, None, "user:a")
        .await
        .unwrap();
    set_head(&pool, tenant_a, "ops", "flow", &v1).await.unwrap();

    let err = rollback_to(&pool, tenant_b, "ops", "flow", &v1, "user:b")
        .await
        .unwrap_err();
    assert!(
        matches!(err, ArgoError::NotFound(_)),
        "cross-tenant rollback must surface NotFound (existence must not leak), got {err:?}"
    );
}

#[tokio::test]
async fn rollback_to_target_id_is_head_returns_invariant() {
    let pool = fresh_pool().await;
    let tenant = TenantId::new();
    let toml = sample_toml("flow");
    let v1 = create(&pool, tenant, "ops", "flow", &toml, None, "user:a")
        .await
        .unwrap();
    set_head(&pool, tenant, "ops", "flow", &v1).await.unwrap();

    let err = rollback_to(&pool, tenant, "ops", "flow", &v1, "user:b")
        .await
        .unwrap_err();
    assert!(
        matches!(err, ArgoError::Invariant(_)),
        "rollback-to-current-head must surface Invariant, got {err:?}"
    );
}
