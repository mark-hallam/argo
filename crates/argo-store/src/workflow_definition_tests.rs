//! Unit + integration coverage for [`super::workflow_definition`] —
//! fresh in-memory pool per test, exercises round-trip semantics,
//! version chain, head pointer transitions, tenant isolation, the
//! duplicate `(tenant, vertical, name, version)` rejection, and
//! migration idempotency.

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
async fn create_then_get_round_trip() {
    let pool = fresh_pool().await;
    let tenant = TenantId::new();
    let toml = sample_toml("greet");
    let id = create(&pool, tenant, "support", "greet", &toml, None, "user:alice")
        .await
        .unwrap();
    let fetched = get(&pool, &id).await.unwrap().expect("row should exist");
    assert_eq!(fetched.id, id);
    assert_eq!(fetched.tenant_id, tenant);
    assert_eq!(fetched.vertical_id, "support");
    assert_eq!(fetched.name, "greet");
    assert_eq!(fetched.definition_toml, toml);
    assert_eq!(fetched.version, 1);
    assert_eq!(fetched.parent_id, None);
    assert_eq!(fetched.lifecycle_state, WorkflowLifecycleState::Draft);
    assert_eq!(fetched.modified_by, "user:alice");
    assert!(!fetched.created_at.is_empty());
}

#[tokio::test]
async fn version_increments_per_triple() {
    let pool = fresh_pool().await;
    let tenant = TenantId::new();
    let toml = sample_toml("greet");
    let v1 = create(&pool, tenant, "support", "greet", &toml, None, "user:a")
        .await
        .unwrap();
    let v2 = create(
        &pool,
        tenant,
        "support",
        "greet",
        &toml,
        Some(v1.clone()),
        "user:a",
    )
    .await
    .unwrap();
    let v3 = create(
        &pool,
        tenant,
        "support",
        "greet",
        &toml,
        Some(v2.clone()),
        "user:a",
    )
    .await
    .unwrap();
    // A different name on the same (tenant, vertical) should reset.
    let other_v1 = create(&pool, tenant, "support", "escalate", &toml, None, "user:a")
        .await
        .unwrap();

    let r1 = get(&pool, &v1).await.unwrap().unwrap();
    let r2 = get(&pool, &v2).await.unwrap().unwrap();
    let r3 = get(&pool, &v3).await.unwrap().unwrap();
    let ro1 = get(&pool, &other_v1).await.unwrap().unwrap();
    assert_eq!(r1.version, 1);
    assert_eq!(r2.version, 2);
    assert_eq!(r3.version, 3);
    assert_eq!(ro1.version, 1, "different name resets the counter");
}

#[tokio::test]
async fn parent_id_chain_threads_versions() {
    let pool = fresh_pool().await;
    let tenant = TenantId::new();
    let toml = sample_toml("flow");
    let v1 = create(&pool, tenant, "ops", "flow", &toml, None, "user:a")
        .await
        .unwrap();
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
    let v3 = create(
        &pool,
        tenant,
        "ops",
        "flow",
        &toml,
        Some(v2.clone()),
        "user:a",
    )
    .await
    .unwrap();
    let r2 = get(&pool, &v2).await.unwrap().unwrap();
    let r3 = get(&pool, &v3).await.unwrap().unwrap();
    assert_eq!(r2.parent_id, Some(v1.clone()));
    assert_eq!(r3.parent_id, Some(v2.clone()));
}

#[tokio::test]
async fn list_versions_newest_first() {
    let pool = fresh_pool().await;
    let tenant = TenantId::new();
    let toml = sample_toml("flow");
    let v1 = create(&pool, tenant, "ops", "flow", &toml, None, "user:a")
        .await
        .unwrap();
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
    let v3 = create(
        &pool,
        tenant,
        "ops",
        "flow",
        &toml,
        Some(v2.clone()),
        "user:a",
    )
    .await
    .unwrap();
    let rows = list_versions(&pool, tenant, "ops", "flow").await.unwrap();
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].id, v3);
    assert_eq!(rows[1].id, v2);
    assert_eq!(rows[2].id, v1);
    assert_eq!(rows[0].version, 3);
    assert_eq!(rows[1].version, 2);
    assert_eq!(rows[2].version, 1);
}

#[tokio::test]
async fn get_head_returns_none_when_no_head_set() {
    let pool = fresh_pool().await;
    let tenant = TenantId::new();
    let toml = sample_toml("flow");
    let _v1 = create(&pool, tenant, "ops", "flow", &toml, None, "user:a")
        .await
        .unwrap();
    let head = get_head(&pool, tenant, "ops", "flow").await.unwrap();
    assert!(
        head.is_none(),
        "head should be None until set_head is called"
    );
}

#[tokio::test]
async fn set_head_then_get_head_returns_row() {
    let pool = fresh_pool().await;
    let tenant = TenantId::new();
    let toml = sample_toml("flow");
    let v1 = create(&pool, tenant, "ops", "flow", &toml, None, "user:a")
        .await
        .unwrap();
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
    let head = get_head(&pool, tenant, "ops", "flow")
        .await
        .unwrap()
        .expect("head must be set");
    assert_eq!(head.id, v2);
    assert_eq!(head.version, 2);
    assert_eq!(
        head.lifecycle_state,
        WorkflowLifecycleState::Active,
        "set_head promotes Draft to Active"
    );

    // set_head on a non-existent id returns NotFound.
    let bogus = WorkflowDefinitionId::new();
    let err = set_head(&pool, tenant, "ops", "flow", &bogus)
        .await
        .unwrap_err();
    assert!(
        matches!(err, ArgoError::NotFound(_)),
        "expected NotFound, got {err:?}"
    );

    // Re-pointing head to v1 works.
    set_head(&pool, tenant, "ops", "flow", &v1).await.unwrap();
    let head2 = get_head(&pool, tenant, "ops", "flow")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(head2.id, v1);
}

#[tokio::test]
async fn archive_flips_state_and_clears_head_when_head() {
    let pool = fresh_pool().await;
    let tenant = TenantId::new();
    let toml = sample_toml("flow");
    let v1 = create(&pool, tenant, "ops", "flow", &toml, None, "user:a")
        .await
        .unwrap();
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

    // Archiving a non-head row leaves the head pointer alone.
    archive(&pool, &v1).await.unwrap();
    let r1 = get(&pool, &v1).await.unwrap().unwrap();
    assert_eq!(r1.lifecycle_state, WorkflowLifecycleState::Archived);
    let head_after_v1_archive = get_head(&pool, tenant, "ops", "flow").await.unwrap();
    assert!(
        head_after_v1_archive.is_some(),
        "head still set when a non-head row archives"
    );

    // Archiving the current head clears the head pointer in the same
    // transaction.
    archive(&pool, &v2).await.unwrap();
    let r2 = get(&pool, &v2).await.unwrap().unwrap();
    assert_eq!(r2.lifecycle_state, WorkflowLifecycleState::Archived);
    let head_after_v2_archive = get_head(&pool, tenant, "ops", "flow").await.unwrap();
    assert!(
        head_after_v2_archive.is_none(),
        "head should clear when its row archives"
    );
}

#[tokio::test]
async fn tenant_isolation_blocks_cross_tenant_reads() {
    let pool = fresh_pool().await;
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    let toml = sample_toml("flow");
    let a1 = create(&pool, tenant_a, "ops", "flow", &toml, None, "user:a")
        .await
        .unwrap();
    let b1 = create(&pool, tenant_b, "ops", "flow", &toml, None, "user:b")
        .await
        .unwrap();
    set_head(&pool, tenant_a, "ops", "flow", &a1).await.unwrap();
    set_head(&pool, tenant_b, "ops", "flow", &b1).await.unwrap();

    // list_versions is tenant-scoped.
    let a_rows = list_versions(&pool, tenant_a, "ops", "flow").await.unwrap();
    let b_rows = list_versions(&pool, tenant_b, "ops", "flow").await.unwrap();
    assert_eq!(a_rows.len(), 1);
    assert_eq!(b_rows.len(), 1);
    assert_eq!(a_rows[0].id, a1);
    assert_eq!(b_rows[0].id, b1);

    // get_head is tenant-scoped.
    let head_a = get_head(&pool, tenant_a, "ops", "flow")
        .await
        .unwrap()
        .unwrap();
    let head_b = get_head(&pool, tenant_b, "ops", "flow")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(head_a.id, a1);
    assert_eq!(head_b.id, b1);
    assert_ne!(head_a.id, head_b.id);

    // set_head with the wrong tenant for an existing id rejects.
    let err = set_head(&pool, tenant_b, "ops", "flow", &a1)
        .await
        .unwrap_err();
    assert!(
        matches!(err, ArgoError::Invariant(_)),
        "cross-tenant set_head must be rejected, got {err:?}"
    );
}

#[tokio::test]
async fn duplicate_triple_version_is_rejected() {
    // The UNIQUE(tenant_id, vertical_id, name, version) constraint
    // is the data-integrity guard; the CRUD layer computes
    // version = MAX + 1 so the only way to hit the constraint is to
    // hand-craft an INSERT with the same version. Probe the raw
    // constraint so a future CRUD change that breaks the invariant
    // still fails this test.
    let pool = fresh_pool().await;
    let tenant = TenantId::new();
    let toml = sample_toml("flow");
    let _v1 = create(&pool, tenant, "ops", "flow", &toml, None, "user:a")
        .await
        .unwrap();

    let dup = sqlx::query(
        "INSERT INTO workflow_definition \
            (id, tenant_id, vertical_id, name, definition_toml, version, \
             parent_id, lifecycle_state, created_at, modified_by) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind("manual-dup-id")
    .bind(tenant.to_string())
    .bind("ops")
    .bind("flow")
    .bind(&toml)
    .bind(1_i64) // collides with v1
    .bind(Option::<String>::None)
    .bind("draft")
    .bind("2026-01-01T00:00:00Z")
    .bind("user:a")
    .execute(&pool)
    .await;
    assert!(
        dup.is_err(),
        "duplicate (tenant, vertical, name, version) must be rejected"
    );
}

#[tokio::test]
async fn migration_is_idempotent_on_double_run() {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    // Second run of the migration over the same pool must succeed
    // (CREATE TABLE IF NOT EXISTS + CREATE INDEX IF NOT EXISTS).
    store
        .migrate()
        .await
        .expect("re-running migration on a migrated pool must be a no-op");
    let pool = store.pool().clone();
    // And the tables are still writable after the second pass.
    let tenant = TenantId::new();
    let toml = sample_toml("flow");
    let id = create(&pool, tenant, "ops", "flow", &toml, None, "user:a")
        .await
        .unwrap();
    assert!(get(&pool, &id).await.unwrap().is_some());
}

#[tokio::test]
async fn create_rejects_malformed_toml() {
    let pool = fresh_pool().await;
    let tenant = TenantId::new();
    // Missing required `[workflow]` section + zero steps.
    let bad = "schema_version = \"1\"\n";
    let err = create(&pool, tenant, "ops", "flow", bad, None, "user:a")
        .await
        .unwrap_err();
    assert!(
        matches!(err, WorkflowDefinitionError::Argo(ArgoError::Schema(_))),
        "malformed TOML must surface as Schema, got {err:?}"
    );
}

#[tokio::test]
async fn concurrent_create_surfaces_typed_unique_violation() {
    // Brief `claude-workflow-create-unique-collision-as-409`:
    // evaluator round-7 saw concurrent POSTs collide on the
    // `UNIQUE(tenant_id, vertical_id, name, version)` constraint and
    // the loser surfaced as a generic 500 instead of a typed conflict.
    // Spawn N writers behind a barrier so the SELECT MAX + INSERT
    // race-window opens for every racer simultaneously; at least one
    // must surface the typed [`WorkflowDefinitionError::UniqueViolation`]
    // carrying the colliding `(vertical, name)`.
    use std::sync::Arc;
    use tokio::sync::Barrier;

    let pool = fresh_pool().await;
    let tenant = TenantId::new();
    let toml = sample_toml("flow");

    const N: usize = 8;
    let barrier = Arc::new(Barrier::new(N));
    let mut handles = Vec::with_capacity(N);
    for i in 0..N {
        let pool = pool.clone();
        let toml = toml.clone();
        let barrier = barrier.clone();
        let actor = format!("user-{i}");
        handles.push(tokio::spawn(async move {
            barrier.wait().await;
            create(&pool, tenant, "ops", "flow", &toml, None, &actor).await
        }));
    }
    let mut results = Vec::with_capacity(N);
    for h in handles {
        results.push(h.await.expect("join"));
    }

    let success_count = results.iter().filter(|r| r.is_ok()).count();
    let collision_count = results
        .iter()
        .filter(|r| matches!(r, Err(WorkflowDefinitionError::UniqueViolation { .. })))
        .count();
    let other_error_count = results
        .iter()
        .filter(|r| matches!(r, Err(WorkflowDefinitionError::Argo(_))))
        .count();

    assert_eq!(
        other_error_count, 0,
        "concurrent inserts must not surface as untyped Argo errors: {results:?}"
    );
    assert!(
        success_count >= 1,
        "at least one writer must win the race"
    );
    assert!(
        collision_count >= 1,
        "expected at least one UniqueViolation among {N} concurrent creators; got {success_count} successes"
    );
    for r in &results {
        if let Err(WorkflowDefinitionError::UniqueViolation { vertical, name }) = r {
            assert_eq!(vertical, "ops", "vertical echoed back verbatim");
            assert_eq!(name, "flow", "name echoed back verbatim");
        }
    }

    let row_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM workflow_definition WHERE name = ?")
            .bind("flow")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        row_count as usize, success_count,
        "persisted row count must equal success count (no orphan writes on collision path)"
    );
}

// ── `_in_tx` atomicity tests ─────────────────────────────────────
//
// `mission-workflow-audit-write-transactional` introduced
// [`set_head_in_tx`] / [`archive_in_tx`] / [`rollback_to_in_tx`] so
// the gateway handler can share one sqlx tx with the `audit_record`
// insert. The atomicity contract: if the caller drops the tx without
// committing, NONE of the in-tx writes persist.

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
