//! Pool-level tests for `worker_archive`. The handler layer
//! exercises the full Worker round-trip — these tests focus on
//! the SQL invariants: insert/delete atomicity, error paths, and
//! list ordering.

use super::*;
use crate::SessionStore;
use sqlx::SqlitePool;

/// Identity schema lives in argo-identity (cyclic dep would block us
/// from calling `IdentityStore::ensure_schema()` here), so the test
/// helper applies the same migration file directly. v1 creates
/// `role` without the worker_id / charter / kpis_json / escalation
/// columns, so we mirror `ensure_role_columns` from argo-identity to
/// add them after the IF NOT EXISTS create-table is a no-op.
const IDENTITY_SCHEMA_SQL: &str =
    include_str!("../migrations/12__worker_identity.sql");

async fn pool() -> SqlitePool {
    let store = SessionStore::open(":memory:").await.expect("open store");
    store.migrate().await.expect("migrate");
    for stmt in IDENTITY_SCHEMA_SQL.split_terminator(';') {
        let sql = stmt.trim();
        if sql.is_empty() {
            continue;
        }
        sqlx::query(sql)
            .execute(store.pool())
            .await
            .expect("apply identity schema");
    }
    for (name, ty) in [
        ("worker_id", "TEXT"),
        ("charter", "TEXT"),
        ("kpis_json", "TEXT"),
        ("escalation", "TEXT"),
    ] {
        let _ = sqlx::query(&format!("ALTER TABLE role ADD COLUMN {name} {ty}"))
            .execute(store.pool())
            .await;
    }
    ensure_schema(store.pool()).await.expect("apply archive schema");
    store.pool().clone()
}

async fn seed_live_worker(pool: &SqlitePool, worker_id: &str, tenant_id: &str) {
    sqlx::query(
        "INSERT INTO persona
            (id, worker_id, tenant_id, name, voice_preamble, avatar, bio,
             builder_attribution, provenance_chain_json)
         VALUES (?, ?, ?, ?, ?, NULL, NULL, ?, ?)",
    )
    .bind(format!("persona-{worker_id}"))
    .bind(worker_id)
    .bind(tenant_id)
    .bind("Maya")
    .bind("op@example.test")
    .bind("op@example.test")
    .bind("[]")
    .execute(pool)
    .await
    .expect("seed persona");

    sqlx::query(
        "INSERT INTO role
            (tenant_id, name, inherits_from, capabilities, worker_id,
             charter, kpis_json, escalation)
         VALUES (?, ?, NULL, '[]', ?, NULL, NULL, NULL)",
    )
    .bind(tenant_id.as_bytes())
    .bind(format!("worker:{worker_id}"))
    .bind(worker_id)
    .execute(pool)
    .await
    .expect("seed role");

    sqlx::query(
        "INSERT INTO worker
            (id, tenant_id, vertical_id, persona_id, role_id, memory_scope,
             auto_approval_json, channels_json, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(worker_id)
    .bind(tenant_id)
    .bind("support")
    .bind(format!("persona-{worker_id}"))
    .bind(format!("worker:{worker_id}"))
    .bind("worker_self")
    .bind("{}")
    .bind("[]")
    .bind(0_i64)
    .execute(pool)
    .await
    .expect("seed worker");

    sqlx::query(
        "INSERT INTO duty
            (id, worker_id, role_id, tenant_id, cron, entry_action, description, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(format!("duty-{worker_id}"))
    .bind(worker_id)
    .bind(format!("worker:{worker_id}"))
    .bind(tenant_id)
    .bind("0 9 * * *")
    .bind("review")
    .bind("morning sweep")
    .bind(0_i64)
    .execute(pool)
    .await
    .expect("seed duty");
}

#[tokio::test]
async fn round_trip_archive_then_rehydrate_succeeds() {
    let pool = pool().await;
    let worker_id = "11111111-1111-1111-1111-111111111111";
    let tenant_id = "22222222-2222-2222-2222-222222222222";
    seed_live_worker(&pool, worker_id, tenant_id).await;

    let frozen = "{\"id\":\"11111111-1111-1111-1111-111111111111\",\"name\":\"Maya\"}";

    archive_worker(&pool, worker_id, tenant_id, Some("alice@team"), Some("done"), frozen)
        .await
        .expect("archive ok");

    let workers: Vec<String> = sqlx::query_scalar("SELECT id FROM worker WHERE id = ?")
        .bind(worker_id)
        .fetch_all(&pool)
        .await
        .unwrap();
    assert!(workers.is_empty(), "live worker row deleted");

    let duties: Vec<String> =
        sqlx::query_scalar("SELECT id FROM duty WHERE worker_id = ?")
            .bind(worker_id)
            .fetch_all(&pool)
            .await
            .unwrap();
    assert!(duties.is_empty(), "duty rows deleted");

    let restored = rehydrate_worker(&pool, worker_id, tenant_id)
        .await
        .expect("rehydrate ok");
    assert_eq!(restored, frozen);

    let archive_after: Vec<String> =
        sqlx::query_scalar("SELECT id FROM worker_archive WHERE id = ?")
            .bind(worker_id)
            .fetch_all(&pool)
            .await
            .unwrap();
    assert!(
        archive_after.is_empty(),
        "archive row removed on rehydrate"
    );
}

#[tokio::test]
async fn archive_errors_when_no_live_worker() {
    let pool = pool().await;
    let err = archive_worker(
        &pool,
        "33333333-3333-3333-3333-333333333333",
        "44444444-4444-4444-4444-444444444444",
        None,
        None,
        "{}",
    )
    .await
    .expect_err("no live worker");
    assert!(err.to_string().contains("no live worker"));
}

#[tokio::test]
async fn archive_errors_when_already_archived() {
    let pool = pool().await;
    let worker_id = "55555555-5555-5555-5555-555555555555";
    let tenant_id = "66666666-6666-6666-6666-666666666666";
    seed_live_worker(&pool, worker_id, tenant_id).await;

    archive_worker(&pool, worker_id, tenant_id, None, None, "{}")
        .await
        .expect("first archive");

    // Re-seeding bypasses the live-row check; we want to hit the
    // "already archived" branch.
    seed_live_worker(&pool, worker_id, tenant_id).await;
    let err = archive_worker(&pool, worker_id, tenant_id, None, None, "{}")
        .await
        .expect_err("already archived");
    assert!(err.to_string().contains("already archived"));
}

#[tokio::test]
async fn rehydrate_errors_when_archive_row_missing() {
    let pool = pool().await;
    let err = rehydrate_worker(
        &pool,
        "77777777-7777-7777-7777-777777777777",
        "88888888-8888-8888-8888-888888888888",
    )
    .await
    .expect_err("nothing to rehydrate");
    assert!(err.to_string().contains("no archive row"));
}

#[tokio::test]
async fn rehydrate_errors_when_live_worker_already_exists() {
    let pool = pool().await;
    let worker_id = "99999999-9999-9999-9999-999999999999";
    let tenant_id = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa";
    seed_live_worker(&pool, worker_id, tenant_id).await;
    archive_worker(&pool, worker_id, tenant_id, None, None, "{}")
        .await
        .expect("first archive");
    // Reseed a live worker into the same id so rehydrate hits the
    // collision branch without touching the archive row.
    seed_live_worker(&pool, worker_id, tenant_id).await;
    let err = rehydrate_worker(&pool, worker_id, tenant_id)
        .await
        .expect_err("collision");
    assert!(err.to_string().contains("already exists"));
}

#[tokio::test]
async fn list_archived_orders_newest_first() {
    let pool = pool().await;
    let tenant_id = "cccccccc-cccc-cccc-cccc-cccccccccccc";

    let id_a = "bbbbbbb1-bbbb-bbbb-bbbb-bbbbbbbbbbbb";
    let id_b = "bbbbbbb2-bbbb-bbbb-bbbb-bbbbbbbbbbbb";
    seed_live_worker(&pool, id_a, tenant_id).await;
    archive_worker(&pool, id_a, tenant_id, None, Some("first"), "{}")
        .await
        .unwrap();
    // Bump archived_at so ordering is unambiguous on systems where
    // two writes can land within the same micro.
    sqlx::query("UPDATE worker_archive SET archived_at = ? WHERE id = ?")
        .bind(1_000_i64)
        .bind(id_a)
        .execute(&pool)
        .await
        .unwrap();

    seed_live_worker(&pool, id_b, tenant_id).await;
    archive_worker(&pool, id_b, tenant_id, None, Some("second"), "{}")
        .await
        .unwrap();
    sqlx::query("UPDATE worker_archive SET archived_at = ? WHERE id = ?")
        .bind(2_000_i64)
        .bind(id_b)
        .execute(&pool)
        .await
        .unwrap();

    let rows = list_archived(&pool, tenant_id).await.expect("list");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].id, id_b);
    assert_eq!(rows[0].reason.as_deref(), Some("second"));
    assert_eq!(rows[1].id, id_a);
    assert_eq!(rows[1].reason.as_deref(), Some("first"));
}

#[tokio::test]
async fn archive_is_atomic_no_partial_writes_on_constraint_violation() {
    let pool = pool().await;
    let worker_id = "dddddddd-dddd-dddd-dddd-dddddddddddd";
    let tenant_id = "eeeeeeee-eeee-eeee-eeee-eeeeeeeeeeee";
    seed_live_worker(&pool, worker_id, tenant_id).await;

    // Seed a stale archive row so the duplicate-id check fails AFTER
    // the live-row check passes. Tx aborts → live rows must remain.
    sqlx::query(
        "INSERT INTO worker_archive
            (id, tenant_id, archived_at, frozen_json)
         VALUES (?, ?, ?, ?)",
    )
    .bind(worker_id)
    .bind(tenant_id)
    .bind(1_i64)
    .bind("{}")
    .execute(&pool)
    .await
    .unwrap();

    let err = archive_worker(&pool, worker_id, tenant_id, None, None, "{}")
        .await
        .expect_err("dup archive");
    assert!(err.to_string().contains("already archived"));

    // Live row still there.
    let live: Option<String> =
        sqlx::query_scalar("SELECT id FROM worker WHERE id = ?")
            .bind(worker_id)
            .fetch_optional(&pool)
            .await
            .unwrap();
    assert!(live.is_some(), "tx aborted; live row preserved");
}

#[tokio::test]
async fn list_archived_filters_by_tenant() {
    let pool = pool().await;
    let tenant_a = "f1111111-1111-1111-1111-111111111111";
    let tenant_b = "f2222222-2222-2222-2222-222222222222";
    let id_a = "f3333333-3333-3333-3333-333333333333";
    let id_b = "f4444444-4444-4444-4444-444444444444";

    seed_live_worker(&pool, id_a, tenant_a).await;
    archive_worker(&pool, id_a, tenant_a, None, None, "{}")
        .await
        .unwrap();
    seed_live_worker(&pool, id_b, tenant_b).await;
    archive_worker(&pool, id_b, tenant_b, None, None, "{}")
        .await
        .unwrap();

    let rows_a = list_archived(&pool, tenant_a).await.unwrap();
    assert_eq!(rows_a.len(), 1);
    assert_eq!(rows_a[0].id, id_a);

    let rows_b = list_archived(&pool, tenant_b).await.unwrap();
    assert_eq!(rows_b.len(), 1);
    assert_eq!(rows_b[0].id, id_b);
}
