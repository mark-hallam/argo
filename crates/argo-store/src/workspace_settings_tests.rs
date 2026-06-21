//! Tests for the generic `workspace_settings` key/value store
//! (theatre item `T1`). Each test applies the versioned migration
//! runner up to v85 so the `workspace_settings` table exists exactly
//! as the live runtime would create it.

use super::{
    get_group, tenant_duty_autofire_enabled, upsert_group, AUTONOMY_GROUP_KEY,
};
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::SqlitePool;

async fn migrated_pool() -> SqlitePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("pool");
    crate::migrations::run_up_to(&pool, 85)
        .await
        .expect("migrations up to v85");
    pool
}

#[tokio::test]
async fn get_returns_none_when_no_row() {
    let pool = migrated_pool().await;
    let stored = get_group(&pool, "tenant-a", "workspace")
        .await
        .expect("get");
    assert_eq!(stored, None, "an unseeded panel must read back as None");
}

#[tokio::test]
async fn upsert_then_get_round_trips() {
    let pool = migrated_pool().await;
    upsert_group(&pool, "tenant-a", "providers", "{\"primary_provider\":\"openai\"}", 10)
        .await
        .expect("upsert");
    let stored = get_group(&pool, "tenant-a", "providers")
        .await
        .expect("get");
    assert_eq!(stored.as_deref(), Some("{\"primary_provider\":\"openai\"}"));
}

#[tokio::test]
async fn upsert_overwrites_existing_row() {
    let pool = migrated_pool().await;
    upsert_group(&pool, "tenant-a", "notifications", "{\"email_enabled\":true}", 1)
        .await
        .expect("first upsert");
    // Second upsert on the same (tenant, group) must REPLACE, not error
    // on the primary key.
    upsert_group(&pool, "tenant-a", "notifications", "{\"email_enabled\":false}", 2)
        .await
        .expect("second upsert");

    let stored = get_group(&pool, "tenant-a", "notifications")
        .await
        .expect("get");
    assert_eq!(stored.as_deref(), Some("{\"email_enabled\":false}"));

    // Exactly one row — the upsert replaced rather than appended.
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM workspace_settings \
         WHERE tenant_id = 'tenant-a' AND group_key = 'notifications'",
    )
    .fetch_one(&pool)
    .await
    .expect("count");
    assert_eq!(count, 1);
}

#[tokio::test]
async fn other_tenant_sees_none_not_tenant_a_values() {
    let pool = migrated_pool().await;
    upsert_group(&pool, "tenant-a", "workspace", "{\"name\":\"A workspace\"}", 5)
        .await
        .expect("tenant-a upsert");

    // Tenant B must NOT see tenant A's row — it gets None and falls
    // back to its own defaults at the handler layer.
    let other = get_group(&pool, "tenant-b", "workspace")
        .await
        .expect("tenant-b get");
    assert_eq!(other, None, "cross-tenant read must not leak tenant-a's row");

    // Tenant B can write its own row under the same group_key without
    // colliding with tenant A's.
    upsert_group(&pool, "tenant-b", "workspace", "{\"name\":\"B workspace\"}", 6)
        .await
        .expect("tenant-b upsert");
    let a = get_group(&pool, "tenant-a", "workspace")
        .await
        .expect("tenant-a get");
    let b = get_group(&pool, "tenant-b", "workspace")
        .await
        .expect("tenant-b get");
    assert_eq!(a.as_deref(), Some("{\"name\":\"A workspace\"}"));
    assert_eq!(b.as_deref(), Some("{\"name\":\"B workspace\"}"));
}

#[tokio::test]
async fn duty_autofire_defaults_off_for_unseeded_tenant() {
    // Operator standing rule: a tenant that never opted in is OFF.
    let pool = migrated_pool().await;
    assert!(
        !tenant_duty_autofire_enabled(&pool, "tenant-a")
            .await
            .expect("read"),
        "an unseeded tenant must default to autonomy OFF"
    );
}

#[tokio::test]
async fn duty_autofire_reads_explicit_opt_in_and_opt_out() {
    let pool = migrated_pool().await;
    // Explicit opt-in.
    upsert_group(&pool, "tenant-a", AUTONOMY_GROUP_KEY, "{\"duty_autofire\":true}", 1)
        .await
        .expect("opt-in");
    assert!(tenant_duty_autofire_enabled(&pool, "tenant-a")
        .await
        .expect("read on"));
    // Explicit opt-out flips it back OFF.
    upsert_group(&pool, "tenant-a", AUTONOMY_GROUP_KEY, "{\"duty_autofire\":false}", 2)
        .await
        .expect("opt-out");
    assert!(!tenant_duty_autofire_enabled(&pool, "tenant-a")
        .await
        .expect("read off"));
}

#[tokio::test]
async fn duty_autofire_fail_closes_on_missing_field_or_garbage() {
    let pool = migrated_pool().await;
    // Saved autonomy panel that omits the field → OFF (fail-closed).
    upsert_group(&pool, "t1", AUTONOMY_GROUP_KEY, "{\"other\":1}", 1)
        .await
        .expect("seed missing-field");
    assert!(!tenant_duty_autofire_enabled(&pool, "t1").await.expect("t1"));
    // Unparseable JSON → OFF (fail-closed), never an error.
    upsert_group(&pool, "t2", AUTONOMY_GROUP_KEY, "not json", 1)
        .await
        .expect("seed garbage");
    assert!(!tenant_duty_autofire_enabled(&pool, "t2").await.expect("t2"));
}

#[tokio::test]
async fn duty_autofire_is_tenant_scoped() {
    let pool = migrated_pool().await;
    // Tenant A opts in; tenant B must stay OFF — one customer opting in
    // never flips it for another.
    upsert_group(&pool, "tenant-a", AUTONOMY_GROUP_KEY, "{\"duty_autofire\":true}", 1)
        .await
        .expect("a opt-in");
    assert!(tenant_duty_autofire_enabled(&pool, "tenant-a").await.expect("a"));
    assert!(
        !tenant_duty_autofire_enabled(&pool, "tenant-b").await.expect("b"),
        "tenant-a's opt-in must not leak to tenant-b"
    );
}

#[tokio::test]
async fn distinct_groups_under_one_tenant_coexist() {
    let pool = migrated_pool().await;
    upsert_group(&pool, "t", "workspace", "{\"name\":\"w\"}", 1)
        .await
        .expect("workspace");
    upsert_group(&pool, "t", "providers", "{\"primary_provider\":\"anthropic\"}", 1)
        .await
        .expect("providers");
    upsert_group(&pool, "t", "notifications", "{\"email_enabled\":true}", 1)
        .await
        .expect("notifications");

    assert_eq!(
        get_group(&pool, "t", "workspace").await.expect("w").as_deref(),
        Some("{\"name\":\"w\"}")
    );
    assert_eq!(
        get_group(&pool, "t", "providers").await.expect("p").as_deref(),
        Some("{\"primary_provider\":\"anthropic\"}")
    );
    assert_eq!(
        get_group(&pool, "t", "notifications").await.expect("n").as_deref(),
        Some("{\"email_enabled\":true}")
    );
}
