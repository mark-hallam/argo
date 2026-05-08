use super::*;
use crate::models::SessionHeader;

async fn store_with_workspace() -> (SessionStore, TenantId, WorkspaceId) {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    let (tenant, workspace) = store.ensure_workspace("worker", "/tmp/worker").await.unwrap();
    (store, tenant, workspace)
}

fn worker() -> WorkerId {
    WorkerId(uuid::Uuid::now_v7())
}

#[tokio::test]
async fn create_session_with_worker_round_trips() {
    let (store, tenant, workspace) = store_with_workspace().await;
    let worker = worker();
    let sid = store
        .create_session_with_worker(SessionCreate {
            tenant_id: tenant,
            workspace_id: workspace,
            model: "model",
            provider: "provider",
            title: Some("title"),
            vertical_id: None,
            worker_id: Some(worker),
        })
        .await
        .unwrap();

    let got = store.get_session(sid).await.unwrap().unwrap();
    assert_eq!(got.worker_id, Some(worker));
    assert_eq!(store.list_sessions(tenant, 10).await.unwrap()[0].worker_id, Some(worker));
}

#[tokio::test]
async fn create_session_without_worker_stays_null() {
    let (store, tenant, workspace) = store_with_workspace().await;
    let sid = store
        .create_session(tenant, workspace, "model", "provider", None, None)
        .await
        .unwrap();
    assert_eq!(store.get_session(sid).await.unwrap().unwrap().worker_id, None);
}

#[tokio::test]
async fn set_session_worker_id_updates_and_clears() {
    let (store, tenant, workspace) = store_with_workspace().await;
    let sid = store
        .create_session(tenant, workspace, "model", "provider", None, None)
        .await
        .unwrap();
    let worker = worker();
    assert_eq!(store.set_session_worker_id(sid, Some(worker)).await.unwrap(), 1);
    assert_eq!(store.get_session(sid).await.unwrap().unwrap().worker_id, Some(worker));
    assert_eq!(store.set_session_worker_id(sid, None).await.unwrap(), 1);
    assert_eq!(store.get_session(sid).await.unwrap().unwrap().worker_id, None);
}

#[tokio::test]
async fn migration_adds_nullable_worker_column_and_partial_index() {
    let (store, tenant, workspace) = store_with_workspace().await;
    let sid = store
        .create_session(tenant, workspace, "model", "provider", None, None)
        .await
        .unwrap();
    store.migrate().await.unwrap();
    let columns: Vec<String> = sqlx::query_scalar("SELECT name FROM pragma_table_info('session')")
        .fetch_all(store.pool())
        .await
        .unwrap();
    let indexes: Vec<String> = sqlx::query_scalar("SELECT name FROM pragma_index_list('session')")
        .fetch_all(store.pool())
        .await
        .unwrap();
    assert!(columns.iter().any(|c| c == "worker_id"));
    assert!(indexes.iter().any(|i| i == "idx_session_worker"));
    let worker_id: Option<Vec<u8>> =
        sqlx::query_scalar("SELECT worker_id FROM session WHERE id = ?")
            .bind(sid.0.as_bytes().as_slice())
            .fetch_one(store.pool())
            .await
            .unwrap();
    assert!(worker_id.is_none());
}

#[tokio::test]
async fn session_header_json_omits_absent_worker_id() {
    let (store, tenant, workspace) = store_with_workspace().await;
    let sid = store
        .create_session(tenant, workspace, "model", "provider", None, None)
        .await
        .unwrap();
    let header: SessionHeader = store.get_session(sid).await.unwrap().unwrap();
    let value = serde_json::to_value(&header).unwrap();
    assert!(value.get("worker_id").is_none());
}

#[tokio::test]
async fn malformed_worker_blob_returns_error() {
    let (store, tenant, workspace) = store_with_workspace().await;
    let sid = SessionId::new();
    sqlx::query(
        "INSERT INTO session (id, tenant_id, workspace_id, model, provider, started_at, worker_id)
         VALUES (?, ?, ?, 'model', 'provider', 1, ?)",
    )
    .bind(sid.0.as_bytes().as_slice())
    .bind(tenant.0.as_bytes().as_slice())
    .bind(workspace.0.as_bytes().as_slice())
    .bind(vec![1_u8, 2])
    .execute(store.pool())
    .await
    .unwrap();
    let err = store.get_session(sid).await.unwrap_err();
    assert!(err.to_string().contains("worker_id blob must be 16 bytes"));
}
