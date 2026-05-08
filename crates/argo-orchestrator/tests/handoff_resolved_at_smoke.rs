//! Integration smoke for handoff `resolved_at` persistence.

use argo_identity::WorkerId;
use argo_orchestrator::{
    EscalationStep, Handoff, HandoffId, HandoffStatus, OrchestratorStore, Session,
};
use argo_store::SessionStore;
use argo_types::TenantId;
use sqlx::Row;
use std::sync::Arc;

async fn seeded_handoff() -> (Arc<SessionStore>, OrchestratorStore, TenantId, Handoff) {
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    let repo = OrchestratorStore::new(store.clone());
    repo.ensure_schema().await.unwrap();
    let tenant = TenantId::new();
    let session = Session::new(tenant, "resolved-at");
    repo.insert_session(&session).await.unwrap();
    let handoff = Handoff::new(
        session.id,
        WorkerId::new(),
        EscalationStep::Worker(WorkerId::new()),
        "finish the customer handoff",
        serde_json::json!({"case": "aeon"}),
    );
    repo.insert_handoff(tenant, &handoff).await.unwrap();
    (store, repo, tenant, handoff)
}

async fn resolved_at(store: &SessionStore, id: &HandoffId) -> Option<i64> {
    sqlx::query_scalar("SELECT resolved_at FROM handoff WHERE id = ?")
        .bind(id.as_str())
        .fetch_one(store.pool())
        .await
        .unwrap()
}

#[tokio::test]
async fn ensure_schema_adds_resolved_at_column_and_partial_index() {
    let (store, _repo, _tenant, _handoff) = seeded_handoff().await;
    let columns = sqlx::query("PRAGMA table_info(handoff)")
        .fetch_all(store.pool())
        .await
        .unwrap();
    assert!(columns.iter().any(|row| {
        row.try_get::<String, _>("name").ok().as_deref() == Some("resolved_at")
    }));
    let index_sql: String = sqlx::query_scalar(
        "SELECT sql FROM sqlite_master WHERE type = 'index' AND name = 'idx_handoff_resolved_at'",
    )
    .fetch_one(store.pool())
    .await
    .unwrap();
    assert!(index_sql.contains("WHERE resolved_at IS NOT NULL"));
}

#[tokio::test]
async fn update_handoff_resolved_at_writes_and_rewrites_timestamp() {
    let (store, repo, _tenant, handoff) = seeded_handoff().await;
    repo.update_handoff_resolved_at(&handoff.id, 11)
        .await
        .unwrap();
    assert_eq!(resolved_at(&store, &handoff.id).await, Some(11));
    repo.update_handoff_resolved_at(&handoff.id, 22)
        .await
        .unwrap();
    assert_eq!(resolved_at(&store, &handoff.id).await, Some(22));
}

#[tokio::test]
async fn unsettled_handoff_stays_null_until_status_settles() {
    let (store, repo, tenant, handoff) = seeded_handoff().await;
    assert_eq!(resolved_at(&store, &handoff.id).await, None);
    repo.update_handoff_status(tenant, &handoff.id, HandoffStatus::Settled)
        .await
        .unwrap();
    assert!(resolved_at(&store, &handoff.id).await.is_some());
}
