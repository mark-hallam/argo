//! `_in_tx` atomicity tests for [`super::workflow_definition`].
//!
//! `mission-workflow-audit-write-transactional` introduced
//! [`super::set_head_in_tx`] / [`super::archive_in_tx`] /
//! [`super::rollback_to_in_tx`] so the gateway handler can share one
//! sqlx tx with the `audit_record` insert. The atomicity contract: if
//! the caller drops the tx without committing, NONE of the in-tx
//! writes persist.
//!
//! Split out of `workflow_definition_tests.rs` to keep both files
//! under the 500 LOC ceiling.

use super::*;
use crate::SessionStore;
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
async fn set_head_in_tx_rolls_back_when_caller_drops_tx() {
    let pool = fresh_pool().await;
    let tenant = TenantId::new();
    let toml = sample_toml("greet");
    let id = create(&pool, tenant, "support", "greet", &toml, None, "u")
        .await
        .unwrap();
    {
        let mut tx = pool.begin().await.unwrap();
        set_head_in_tx(&mut tx, tenant, "support", "greet", &id)
            .await
            .unwrap();
        // Intentionally drop without committing.
    }
    let head = get_head(&pool, tenant, "support", "greet").await.unwrap();
    assert!(head.is_none(), "head must NOT be set — caller never committed");
    let row = get(&pool, &id).await.unwrap().unwrap();
    assert_eq!(
        row.lifecycle_state,
        WorkflowLifecycleState::Draft,
        "lifecycle UPDATE must roll back with the head INSERT"
    );
}

#[tokio::test]
async fn archive_in_tx_rolls_back_when_caller_drops_tx() {
    let pool = fresh_pool().await;
    let tenant = TenantId::new();
    let toml = sample_toml("greet");
    let id = create(&pool, tenant, "support", "greet", &toml, None, "u")
        .await
        .unwrap();
    set_head(&pool, tenant, "support", "greet", &id).await.unwrap();
    {
        let mut tx = pool.begin().await.unwrap();
        archive_in_tx(&mut tx, &id).await.unwrap();
        // Drop without commit.
    }
    let row = get(&pool, &id).await.unwrap().unwrap();
    assert_eq!(
        row.lifecycle_state,
        WorkflowLifecycleState::Active,
        "archive UPDATE must roll back"
    );
    assert!(
        get_head(&pool, tenant, "support", "greet").await.unwrap().is_some(),
        "head clear must roll back"
    );
}

#[tokio::test]
async fn rollback_to_in_tx_rolls_back_when_caller_drops_tx() {
    let pool = fresh_pool().await;
    let tenant = TenantId::new();
    let toml = sample_toml("greet");
    let v1 = create(&pool, tenant, "support", "greet", &toml, None, "u")
        .await
        .unwrap();
    let v2 = create(
        &pool,
        tenant,
        "support",
        "greet",
        &toml,
        Some(v1.clone()),
        "u",
    )
    .await
    .unwrap();
    set_head(&pool, tenant, "support", "greet", &v2).await.unwrap();
    {
        let mut tx = pool.begin().await.unwrap();
        rollback_to_in_tx(&mut tx, tenant, "support", "greet", &v1, "u")
            .await
            .unwrap();
        // Drop without commit.
    }
    let versions = list_versions(&pool, tenant, "support", "greet").await.unwrap();
    assert_eq!(
        versions.len(),
        2,
        "rollback INSERT must roll back — only the 2 seeded versions remain"
    );
    let head = get_head(&pool, tenant, "support", "greet").await.unwrap().unwrap();
    assert_eq!(head.id, v2, "head must remain v2");
}
