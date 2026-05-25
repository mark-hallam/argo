//! Regression pins for tenant-bound skill vertical queries.

use argo_store::skill_query::{distinct_skill_verticals, search_by_vertical};
use argo_store::SessionStore;
use argo_types::{SessionId, TenantId, WorkspaceId};
use uuid::Uuid;

async fn store() -> SessionStore {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    store
}

async fn insert_skill(
    store: &SessionStore,
    tenant_id: &str,
    skill_id: Uuid,
    name: &str,
    vertical_id: &str,
    created_at: i64,
) {
    sqlx::query(
        "INSERT INTO skill \
         (id, name, version, body, frontmatter, source, hash, created_at, vertical_id, tenant_id) \
         VALUES (?, ?, '1.0.0', '', '', 'inline', randomblob(8), ?, ?, ?)",
    )
    .bind(skill_id.as_bytes().as_slice())
    .bind(name)
    .bind(created_at)
    .bind(vertical_id)
    .bind(tenant_id)
    .execute(store.pool())
    .await
    .unwrap();
}

async fn insert_session(
    store: &SessionStore,
    tenant_id: TenantId,
    workspace_id: WorkspaceId,
    session_id: SessionId,
    vertical_id: &str,
) {
    sqlx::query(
        "INSERT INTO session (id, tenant_id, workspace_id, model, provider, started_at, vertical_id) \
         VALUES (?, ?, ?, 'mock', 'mock', 1, ?)",
    )
    .bind(session_id.0.as_bytes().as_slice())
    .bind(tenant_id.0.as_bytes().as_slice())
    .bind(workspace_id.0.as_bytes().as_slice())
    .bind(vertical_id)
    .execute(store.pool())
    .await
    .unwrap();
}

#[tokio::test]
async fn direct_and_distinct_queries_do_not_cross_tenant_leak() {
    let store = store().await;
    let tenant_a = TenantId::new().to_string();
    let tenant_b = TenantId::new().to_string();
    let skill_a = Uuid::now_v7();
    let skill_b = Uuid::now_v7();

    insert_skill(
        &store,
        &tenant_a,
        skill_a,
        "tenant-a-skill",
        "shared-vertical",
        10,
    )
    .await;
    insert_skill(
        &store,
        &tenant_b,
        skill_b,
        "tenant-b-skill",
        "shared-vertical",
        20,
    )
    .await;
    insert_skill(
        &store,
        "__migration_placeholder__",
        Uuid::now_v7(),
        "legacy-skill",
        "legacy-vertical",
        30,
    )
    .await;

    let rows = search_by_vertical(store.pool(), &tenant_a, "shared-vertical", 10, false)
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].skill_name, "tenant-a-skill");

    let verticals = distinct_skill_verticals(store.pool(), &tenant_a)
        .await
        .unwrap();
    assert_eq!(verticals, vec!["shared-vertical".to_string()]);
}

#[tokio::test]
async fn indirect_query_binds_session_tenant() {
    let store = store().await;
    let (tenant_a, workspace_a) = store
        .ensure_workspace("tenant-a", "/tmp/sq-a")
        .await
        .unwrap();
    let (tenant_b, workspace_b) = store
        .ensure_workspace("tenant-b", "/tmp/sq-b")
        .await
        .unwrap();
    let tenant_a_str = tenant_a.to_string();
    let tenant_b_str = tenant_b.to_string();
    let session_a = SessionId::new();
    let session_b = SessionId::new();
    let skill_a = Uuid::now_v7();
    let skill_b = Uuid::now_v7();

    insert_session(&store, tenant_a, workspace_a, session_a, "shared-vertical").await;
    insert_session(&store, tenant_b, workspace_b, session_b, "shared-vertical").await;
    insert_skill(
        &store,
        &tenant_a_str,
        skill_a,
        "attached-to-tenant-a",
        "other-vertical",
        10,
    )
    .await;
    insert_skill(
        &store,
        &tenant_b_str,
        skill_b,
        "attached-to-tenant-b",
        "other-vertical",
        20,
    )
    .await;

    sqlx::query("INSERT INTO session_skill (session_id, skill_id, attached_at) VALUES (?, ?, 11)")
        .bind(session_a.0.as_bytes().as_slice())
        .bind(skill_a.as_bytes().as_slice())
        .execute(store.pool())
        .await
        .unwrap();
    sqlx::query("INSERT INTO session_skill (session_id, skill_id, attached_at) VALUES (?, ?, 22)")
        .bind(session_b.0.as_bytes().as_slice())
        .bind(skill_b.as_bytes().as_slice())
        .execute(store.pool())
        .await
        .unwrap();

    let rows = search_by_vertical(store.pool(), &tenant_a_str, "shared-vertical", 10, true)
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].skill_name, "attached-to-tenant-a");
    assert_eq!(rows[0].last_activity_micros, 11);
}
