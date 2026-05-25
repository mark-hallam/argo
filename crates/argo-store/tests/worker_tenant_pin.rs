use argo_store::{worker_budget_write, worker_presence, SessionStore};
use chrono::Utc;
use serde_json::Value;
use sqlx::{Pool, Sqlite};

const TENANT_ID: &str = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa";
const OTHER_TENANT_ID: &str = "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb";
const WORKER_ID: &str = "11111111-1111-1111-1111-111111111111";

async fn fresh_store() -> SessionStore {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    worker_presence::ensure_schema(store.pool()).await.unwrap();
    store
}

async fn seed_worker(pool: &Pool<Sqlite>, tenant_id: &str, worker_id: &str, policy: &str) {
    sqlx::query(
        "INSERT INTO worker (id, tenant_id, vertical_id, persona_id, role_id, \
         memory_scope, auto_approval_json, channels_json, created_at, last_active_at) \
         VALUES (?, ?, 'vert-x', 'persona-x', 'role-x', 'scope-x', ?, '[]', 0, 123)",
    )
    .bind(worker_id)
    .bind(tenant_id)
    .bind(policy)
    .execute(pool)
    .await
    .unwrap();
}

async fn read_policy(pool: &Pool<Sqlite>, worker_id: &str) -> Value {
    let raw: String = sqlx::query_scalar("SELECT auto_approval_json FROM worker WHERE id = ?")
        .bind(worker_id)
        .fetch_one(pool)
        .await
        .unwrap();
    serde_json::from_str(&raw).unwrap()
}

#[tokio::test]
async fn worker_budget_helpers_bind_worker_tenant() {
    let store = fresh_store().await;
    seed_worker(
        store.pool(),
        TENANT_ID,
        WORKER_ID,
        "{\"daily_budget_usd_micros\":1}",
    )
    .await;

    let err = worker_budget_write::set_cap(&store, OTHER_TENANT_ID, WORKER_ID, Some(9), None)
        .await
        .unwrap_err();
    assert!(err.to_string().contains(OTHER_TENANT_ID));
    assert_eq!(
        read_policy(store.pool(), WORKER_ID).await["daily_budget_usd_micros"],
        1
    );

    worker_budget_write::set_cap(&store, TENANT_ID, WORKER_ID, Some(7), None)
        .await
        .unwrap();
    let now = Utc::now().timestamp();
    sqlx::query(
        "INSERT INTO worker_budget (tenant_id, worker_id, action_id, usd_micros, charged_at) \
         VALUES (?, ?, 'same-worker-right-tenant', 7, ?)",
    )
    .bind(TENANT_ID.as_bytes())
    .bind(WORKER_ID)
    .bind(now)
    .execute(store.pool())
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO worker_budget (tenant_id, worker_id, action_id, usd_micros, charged_at) \
         VALUES (?, ?, 'same-worker-wrong-tenant', 13, ?)",
    )
    .bind(OTHER_TENANT_ID.as_bytes())
    .bind(WORKER_ID)
    .bind(now)
    .execute(store.pool())
    .await
    .unwrap();

    let summary = worker_budget_write::summary(&store, TENANT_ID, WORKER_ID, Utc::now())
        .await
        .unwrap();
    assert_eq!(summary.caps.daily_micros, Some(7));
    assert_eq!(summary.today_micros, 7);
    assert_eq!(summary.this_month_micros, 7);
}

#[tokio::test]
async fn worker_presence_touch_binds_worker_tenant() {
    let store = fresh_store().await;
    seed_worker(store.pool(), TENANT_ID, WORKER_ID, "{}").await;

    worker_presence::touch_presence_for_tenant(store.pool(), OTHER_TENANT_ID, WORKER_ID)
        .await
        .unwrap();
    let unchanged: i64 = sqlx::query_scalar("SELECT last_active_at FROM worker WHERE id = ?")
        .bind(WORKER_ID)
        .fetch_one(store.pool())
        .await
        .unwrap();
    assert_eq!(unchanged, 123);

    worker_presence::touch_presence_for_tenant(store.pool(), TENANT_ID, WORKER_ID)
        .await
        .unwrap();
    let touched: i64 = sqlx::query_scalar("SELECT last_active_at FROM worker WHERE id = ?")
        .bind(WORKER_ID)
        .fetch_one(store.pool())
        .await
        .unwrap();
    assert!(touched > 123);
}
