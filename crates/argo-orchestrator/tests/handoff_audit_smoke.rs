//! Round-trip integration smoke for `claude-worker-handoff-record`:
//! every Pending→Acknowledged transition lands exactly one
//! `worker_handoff` audit row in the session chain.

use argo_identity::WorkerId;
use argo_orchestrator::{
    EscalationStep, Handoff, HandoffStatus, OrchestratorStore, Session,
};
use argo_store::SessionStore;
use argo_types::TenantId;
use std::sync::Arc;

async fn fresh_store() -> Arc<SessionStore> {
    let s = Arc::new(SessionStore::open(":memory:").await.unwrap());
    s.migrate().await.unwrap();
    s
}

async fn count_audit_kind(
    pool: &sqlx::Pool<sqlx::Sqlite>,
    sid: &uuid::Uuid,
    kind: &str,
) -> i64 {
    sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM audit_record WHERE session_id = ? AND kind = ?",
    )
    .bind(sid.as_bytes().as_slice())
    .bind(kind)
    .fetch_one(pool)
    .await
    .unwrap()
}

#[tokio::test]
async fn pending_to_acknowledged_writes_one_worker_handoff_row() {
    let store = fresh_store().await;
    let repo = OrchestratorStore::new(store.clone());
    repo.ensure_schema().await.unwrap();
    let tenant = TenantId::new();

    let session = Session::new(tenant, "smoke");
    repo.insert_session(&session).await.unwrap();

    let from = WorkerId::new();
    let to = WorkerId::new();
    let h = Handoff::new(
        session.id.clone(),
        from,
        EscalationStep::Worker(to),
        "verify the listing price",
        serde_json::json!({"sku": "USD-AEON-60"}),
    );
    repo.insert_handoff(tenant, &h).await.unwrap();

    repo.update_handoff_status(tenant, &h.id, HandoffStatus::Acknowledged)
        .await
        .unwrap();

    let sid = uuid::Uuid::parse_str(h.session_id.as_str()).unwrap();
    assert_eq!(
        count_audit_kind(store.pool(), &sid, "worker_handoff").await,
        1,
        "exactly one worker_handoff row after Pending→Acknowledged"
    );

    // Re-ack must not write a second row.
    repo.update_handoff_status(tenant, &h.id, HandoffStatus::Acknowledged)
        .await
        .unwrap();
    assert_eq!(
        count_audit_kind(store.pool(), &sid, "worker_handoff").await,
        1,
        "re-ack must be idempotent"
    );

    // Settle does NOT write a worker_handoff row.
    repo.update_handoff_status(tenant, &h.id, HandoffStatus::Settled)
        .await
        .unwrap();
    assert_eq!(
        count_audit_kind(store.pool(), &sid, "worker_handoff").await,
        1,
        "settle must not append a second row"
    );

    // Payload sanity: from/to/session/reason captured.
    let row: (String, String) = sqlx::query_as(
        "SELECT kind, payload_json FROM audit_record \
         WHERE session_id = ? AND kind = 'worker_handoff' LIMIT 1",
    )
    .bind(sid.as_bytes().as_slice())
    .fetch_one(store.pool())
    .await
    .unwrap();
    assert_eq!(row.0, "worker_handoff");
    let v: serde_json::Value = serde_json::from_str(&row.1).unwrap();
    assert_eq!(v["from_worker"], from.to_string());
    assert_eq!(v["to_kind"], "worker");
    assert_eq!(v["to_worker_id"], to.to_string());
    assert_eq!(v["session_id"], session.id.as_str());
    assert_eq!(v["reason"], "verify the listing price");
    assert!(v["payload_hash"].as_str().unwrap().len() == 64);
}
