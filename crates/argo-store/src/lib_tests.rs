//! Tests for [`super::SessionStore`] — extracted from `lib.rs` to
//! keep the production-code file under the 500-LOC ceiling.
#![cfg(test)]
use super::*;

#[tokio::test]
async fn open_and_migrate() {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    assert_eq!(store.schema_version().await.unwrap(), 3);
}

#[tokio::test]
async fn migrate_round_trip() {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    store.migrate_down_to(0).await.unwrap();
    assert_eq!(store.schema_version().await.unwrap(), 0);
    store.migrate().await.unwrap();
    assert_eq!(store.schema_version().await.unwrap(), 3);
}

#[tokio::test]
async fn ensure_workspace_is_idempotent_by_root_path() {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    let (t1, w1) = store.ensure_workspace("smoke", "/tmp/x").await.unwrap();
    let (t2, w2) = store.ensure_workspace("smoke", "/tmp/x").await.unwrap();
    assert_eq!(t1, t2);
    assert_eq!(w1, w2);
    let (_t3, w3) = store.ensure_workspace("smoke", "/tmp/y").await.unwrap();
    assert_ne!(w1, w3);
}

async fn fresh_store_with_workspace() -> (SessionStore, TenantId, WorkspaceId) {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    let tenant = TenantId::new();
    let ws = WorkspaceId::new();
    sqlx::query(
        "INSERT INTO workspace (id, tenant_id, name, root_path, created_at)
         VALUES (?, ?, 'test', '.', strftime('%s','now'))",
    )
    .bind(ws.0.as_bytes().as_slice())
    .bind(tenant.0.as_bytes().as_slice())
    .execute(store.pool())
    .await
    .unwrap();
    (store, tenant, ws)
}

#[tokio::test]
async fn spend_ledger_records_and_aggregates_session_total() {
    let (store, tenant, ws) = fresh_store_with_workspace().await;
    let session = store
        .create_session(tenant, ws, "m", "p", None)
        .await
        .unwrap();
    let now = chrono::Utc::now();
    store
        .record_spend(
            tenant,
            session,
            now.timestamp_micros(),
            250_000,
            "anth",
            "haiku",
        )
        .await
        .unwrap();
    store
        .record_spend(
            tenant,
            session,
            now.timestamp_micros(),
            750_000,
            "anth",
            "haiku",
        )
        .await
        .unwrap();
    let counters = store
        .load_spend_counters(tenant, session, now)
        .await
        .unwrap();
    assert_eq!(counters.session_used, 1_000_000);
    assert_eq!(counters.day_used, 1_000_000);
    assert_eq!(counters.month_used, 1_000_000);
}

#[tokio::test]
async fn spend_ledger_isolates_by_tenant() {
    let (store, tenant_a, ws_a) = fresh_store_with_workspace().await;
    let session_a = store
        .create_session(tenant_a, ws_a, "m", "p", None)
        .await
        .unwrap();
    let tenant_b = TenantId::new();
    let ws_b = WorkspaceId::new();
    sqlx::query(
        "INSERT INTO workspace (id, tenant_id, name, root_path, created_at)
         VALUES (?, ?, 'b', '.', strftime('%s','now'))",
    )
    .bind(ws_b.0.as_bytes().as_slice())
    .bind(tenant_b.0.as_bytes().as_slice())
    .execute(store.pool())
    .await
    .unwrap();
    let session_b = store
        .create_session(tenant_b, ws_b, "m", "p", None)
        .await
        .unwrap();
    let now = chrono::Utc::now();
    store
        .record_spend(
            tenant_b,
            session_b,
            now.timestamp_micros(),
            5_000_000,
            "p",
            "m",
        )
        .await
        .unwrap();
    let a = store
        .load_spend_counters(tenant_a, session_a, now)
        .await
        .unwrap();
    assert_eq!(a.session_used, 0);
    assert_eq!(a.day_used, 0);
    assert_eq!(a.month_used, 0);
}

#[tokio::test]
async fn spend_ledger_day_window_excludes_yesterday() {
    let (store, tenant, ws) = fresh_store_with_workspace().await;
    let session = store
        .create_session(tenant, ws, "m", "p", None)
        .await
        .unwrap();
    let now = chrono::Utc::now();
    let yesterday = now - chrono::Duration::days(2);
    store
        .record_spend(tenant, session, yesterday.timestamp_micros(), 123, "p", "m")
        .await
        .unwrap();
    let counters = store
        .load_spend_counters(tenant, session, now)
        .await
        .unwrap();
    assert_eq!(counters.session_used, 123);
    assert_eq!(counters.day_used, 0);
}

#[tokio::test]
async fn create_session_and_append_messages() {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    let tenant = TenantId::new();
    let ws = WorkspaceId::new();
    sqlx::query(
        "INSERT INTO workspace (id, tenant_id, name, root_path, created_at)
         VALUES (?, ?, 'test', '.', strftime('%s','now'))",
    )
    .bind(ws.0.as_bytes().as_slice())
    .bind(tenant.0.as_bytes().as_slice())
    .execute(store.pool())
    .await
    .unwrap();
    let session = store
        .create_session(tenant, ws, "claude-haiku", "anthropic", Some("test"))
        .await
        .unwrap();
    store
        .append_message(
            session,
            MessageRole::User,
            r#"[{"type":"text","text":"hi"}]"#,
        )
        .await
        .unwrap();
    store
        .append_message(
            session,
            MessageRole::Assistant,
            r#"[{"type":"text","text":"hello"}]"#,
        )
        .await
        .unwrap();
    assert_eq!(store.message_count(session).await.unwrap(), 2);
}

#[tokio::test]
async fn list_sessions_recency_order() {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    let tenant = TenantId::new();
    let ws = WorkspaceId::new();
    sqlx::query(
        "INSERT INTO workspace (id, tenant_id, name, root_path, created_at)
         VALUES (?, ?, 'test', '.', strftime('%s','now'))",
    )
    .bind(ws.0.as_bytes().as_slice())
    .bind(tenant.0.as_bytes().as_slice())
    .execute(store.pool())
    .await
    .unwrap();
    for i in 0..3 {
        store
            .create_session(tenant, ws, "model", "provider", Some(&format!("s{i}")))
            .await
            .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(2)).await;
    }
    let list = store.list_sessions(tenant, 10).await.unwrap();
    assert_eq!(list.len(), 3);
    assert_eq!(list[0].title.as_deref(), Some("s2"));
}

#[tokio::test]
async fn fts5_table_exists() {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    let n: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='message_fts'",
    )
    .fetch_one(store.pool())
    .await
    .unwrap();
    assert_eq!(n, 1);
}
