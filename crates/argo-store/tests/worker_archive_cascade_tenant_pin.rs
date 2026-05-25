use argo_store::worker_archive::{archive_worker, ensure_schema};
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};

const WORKER_ID: &str = "11111111-1111-1111-1111-111111111111";
const TENANT_A: &str = "22222222-2222-2222-2222-222222222222";
const TENANT_B: &str = "33333333-3333-3333-3333-333333333333";

#[tokio::test]
async fn archive_cascade_deletes_only_matching_tenant_child_rows() {
    let pool = pool().await;
    seed_synthetic_worker_clash(&pool).await;

    archive_worker(&pool, WORKER_ID, TENANT_A, None, None, "{}")
        .await
        .expect("archive tenant A worker");

    assert_eq!(text_child_rows(&pool, "persona", TENANT_A).await, 0);
    assert_eq!(text_child_rows(&pool, "persona", TENANT_B).await, 1);
    assert_eq!(role_rows(&pool, TENANT_A).await, 0);
    assert_eq!(role_rows(&pool, TENANT_B).await, 1);
    assert_eq!(text_child_rows(&pool, "duty", TENANT_A).await, 0);
    assert_eq!(text_child_rows(&pool, "duty", TENANT_B).await, 1);

    let live_workers: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM worker WHERE id = ? AND tenant_id = ?")
            .bind(WORKER_ID)
            .bind(TENANT_A)
            .fetch_one(&pool)
            .await
            .expect("count live worker");
    assert_eq!(live_workers, 0);

    let archive_rows: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM worker_archive WHERE id = ? AND tenant_id = ?")
            .bind(WORKER_ID)
            .bind(TENANT_A)
            .fetch_one(&pool)
            .await
            .expect("count archive row");
    assert_eq!(archive_rows, 1);
}

async fn pool() -> SqlitePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("open sqlite memory pool");
    for sql in [
        "CREATE TABLE worker (id TEXT PRIMARY KEY, tenant_id TEXT NOT NULL)",
        "CREATE TABLE persona (
            id TEXT PRIMARY KEY,
            worker_id TEXT NOT NULL,
            tenant_id TEXT NOT NULL
        )",
        "CREATE TABLE role (
            tenant_id BLOB NOT NULL,
            name TEXT NOT NULL,
            worker_id TEXT,
            PRIMARY KEY (tenant_id, name)
        )",
        "CREATE TABLE duty (
            id TEXT PRIMARY KEY,
            worker_id TEXT NOT NULL,
            tenant_id TEXT NOT NULL
        )",
    ] {
        sqlx::query(sql).execute(&pool).await.expect("create table");
    }
    ensure_schema(&pool).await.expect("worker archive schema");
    pool
}

async fn seed_synthetic_worker_clash(pool: &SqlitePool) {
    sqlx::query("INSERT INTO worker (id, tenant_id) VALUES (?, ?)")
        .bind(WORKER_ID)
        .bind(TENANT_A)
        .execute(pool)
        .await
        .expect("seed live worker");

    for (suffix, tenant) in [("a", TENANT_A), ("b", TENANT_B)] {
        sqlx::query("INSERT INTO persona (id, worker_id, tenant_id) VALUES (?, ?, ?)")
            .bind(format!("persona-{suffix}"))
            .bind(WORKER_ID)
            .bind(tenant)
            .execute(pool)
            .await
            .expect("seed persona");
        sqlx::query("INSERT INTO role (tenant_id, name, worker_id) VALUES (?, ?, ?)")
            .bind(tenant.as_bytes())
            .bind(format!("role-{suffix}"))
            .bind(WORKER_ID)
            .execute(pool)
            .await
            .expect("seed role");
        sqlx::query("INSERT INTO duty (id, worker_id, tenant_id) VALUES (?, ?, ?)")
            .bind(format!("duty-{suffix}"))
            .bind(WORKER_ID)
            .bind(tenant)
            .execute(pool)
            .await
            .expect("seed duty");
    }
}

async fn text_child_rows(pool: &SqlitePool, table: &str, tenant: &str) -> i64 {
    let sql = format!("SELECT COUNT(*) FROM {table} WHERE worker_id = ? AND tenant_id = ?");
    sqlx::query_scalar(&sql)
        .bind(WORKER_ID)
        .bind(tenant)
        .fetch_one(pool)
        .await
        .expect("count text child rows")
}

async fn role_rows(pool: &SqlitePool, tenant: &str) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM role WHERE worker_id = ? AND tenant_id = ?")
        .bind(WORKER_ID)
        .bind(tenant.as_bytes())
        .fetch_one(pool)
        .await
        .expect("count role rows")
}
