use super::*;
use crate::SessionStore;
use sqlx::SqlitePool;

const IDENTITY_SCHEMA_SQL: &str = include_str!("../migrations/12__worker_identity.sql");

async fn pool() -> SqlitePool {
    let store = SessionStore::open(":memory:").await.expect("open store");
    store.migrate().await.expect("migrate");
    for stmt in IDENTITY_SCHEMA_SQL.split_terminator(';') {
        let sql = stmt.trim();
        if !sql.is_empty() {
            let result = sqlx::query(sql).execute(store.pool()).await;
            match result {
                Ok(_) => {}
                Err(e) if e.to_string().contains("already exists") => {}
                Err(e) => panic!("identity schema: {e}"),
            }
        }
    }
    ensure_schema(store.pool()).await.expect("presence schema");
    store.pool().clone()
}

async fn seed_worker(pool: &SqlitePool, worker_id: &str, tenant_id: &str, persona: &str) {
    let persona_id = format!("persona-{worker_id}");
    sqlx::query(
        "INSERT INTO persona
            (id, worker_id, tenant_id, name, voice_preamble, avatar, bio,
             builder_attribution, provenance_chain_json)
         VALUES (?, ?, ?, ?, ?, NULL, NULL, ?, ?)",
    )
    .bind(&persona_id)
    .bind(worker_id)
    .bind(tenant_id)
    .bind(persona)
    .bind("op@example.test")
    .bind("op@example.test")
    .bind("[]")
    .execute(pool)
    .await
    .expect("seed persona");

    sqlx::query(
        "INSERT INTO worker
            (id, tenant_id, vertical_id, persona_id, role_id, memory_scope,
             auto_approval_json, channels_json, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(worker_id)
    .bind(tenant_id)
    .bind("support")
    .bind(persona_id)
    .bind(format!("worker:{worker_id}"))
    .bind("worker_self")
    .bind("{}")
    .bind("[]")
    .bind(0_i64)
    .execute(pool)
    .await
    .expect("seed worker");
}

async fn set_last_active(pool: &SqlitePool, worker_id: &str, at: Option<i64>) {
    sqlx::query("UPDATE worker SET last_active_at = ? WHERE id = ?")
        .bind(at)
        .bind(worker_id)
        .execute(pool)
        .await
        .expect("set last_active_at");
}

#[tokio::test]
async fn schema_is_idempotent() {
    let pool = pool().await;
    ensure_schema(&pool).await.unwrap();
    ensure_schema(&pool).await.unwrap();
    assert!(column_exists(&pool, "worker", "last_active_at").await.unwrap());
}

#[tokio::test]
async fn touch_presence_sets_epoch_seconds() {
    let pool = pool().await;
    let worker = "11111111-1111-1111-1111-111111111111";
    seed_worker(&pool, worker, "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa", "Maya").await;
    let before = Utc::now().timestamp();
    touch_presence(&pool, worker).await.unwrap();
    let after = Utc::now().timestamp();
    let touched: i64 = sqlx::query_scalar("SELECT last_active_at FROM worker WHERE id = ?")
        .bind(worker)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!((before..=after).contains(&touched));
}

#[tokio::test]
async fn touch_presence_unknown_worker_noops() {
    let pool = pool().await;
    touch_presence(&pool, "22222222-2222-2222-2222-222222222222")
        .await
        .unwrap();
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM worker")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn fleet_orders_recent_first_with_nulls_last_and_marks_idle() {
    let pool = pool().await;
    let tenant = "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb";
    let active = "33333333-3333-3333-3333-333333333333";
    let idle = "44444444-4444-4444-4444-444444444444";
    let never = "55555555-5555-5555-5555-555555555555";
    seed_worker(&pool, idle, tenant, "Idle").await;
    seed_worker(&pool, active, tenant, "Active").await;
    seed_worker(&pool, never, tenant, "Never").await;
    let now = 1_700_000_000_i64;
    set_last_active(&pool, idle, Some(now - IDLE_THRESHOLD_SECS - 1)).await;
    set_last_active(&pool, active, Some(now - 60)).await;

    let rows = fleet_at(&pool, Some(tenant), 10, None, now).await.unwrap();
    let ids: Vec<&str> = rows.iter().map(|r| r.worker_id.as_str()).collect();
    assert_eq!(ids, vec![active, idle, never]);
    assert_eq!(rows[0].status, "active");
    assert_eq!(rows[1].status, "idle");
    assert_eq!(rows[2].status, "idle");
    assert!(rows[2].last_active_at.is_none());
    assert!(rows[2].last_active_at_rfc3339.is_none());
    assert_eq!(rows[0].persona.as_deref(), Some("Active"));
}

#[tokio::test]
async fn fleet_filters_since_and_limit() {
    let pool = pool().await;
    let tenant = "cccccccc-cccc-cccc-cccc-cccccccccccc";
    let recent = "66666666-6666-6666-6666-666666666666";
    let old = "77777777-7777-7777-7777-777777777777";
    let never = "88888888-8888-8888-8888-888888888888";
    seed_worker(&pool, recent, tenant, "Recent").await;
    seed_worker(&pool, old, tenant, "Old").await;
    seed_worker(&pool, never, tenant, "Never").await;
    set_last_active(&pool, recent, Some(200)).await;
    set_last_active(&pool, old, Some(100)).await;

    let rows = fleet_at(&pool, Some(tenant), 10, Some(150), 200).await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].worker_id, recent);

    let limited = fleet_at(&pool, Some(tenant), 1, None, 200).await.unwrap();
    assert_eq!(limited.len(), 1);
    assert_eq!(limited[0].worker_id, recent);
}
