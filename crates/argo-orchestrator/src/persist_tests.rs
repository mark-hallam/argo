//! Tests extracted from `persist.rs` under the
//! `codex-split-files-450-500-wave` brief to keep the parent file
//! under the 420-LOC headroom target. Pulled verbatim — no behavior
//! change.
use super::*;
use crate::{EscalationStep, Handoff, HandoffStatus};
use std::sync::Arc;

async fn repo() -> OrchestratorStore {
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    let repo = OrchestratorStore::new(store);
    repo.ensure_schema().await.unwrap();
    repo.ensure_schema().await.unwrap();
    repo
}
async fn inserted(repo: &OrchestratorStore, tenant: TenantId, title: &str) -> Session {
    let s = Session::new(tenant, title);
    repo.insert_session(&s).await.unwrap();
    s
}
fn handoff(session: &Session, from: WorkerId, to: WorkerId) -> Handoff {
    Handoff::new(
        session.id.clone(),
        from,
        EscalationStep::Worker(to),
        "finish the customer handoff",
        serde_json::json!({"case": "aeon"}),
    )
}

#[tokio::test]
async fn round_trip_session_and_two_handoffs() {
    let repo = repo().await;
    let tenant = TenantId::new();
    let session = inserted(&repo, tenant, "renewal").await;
    let from = WorkerId::new();
    let to = WorkerId::new();
    repo.insert_handoff(tenant, &handoff(&session, from, to))
        .await
        .unwrap();
    repo.insert_handoff(tenant, &handoff(&session, from, to))
        .await
        .unwrap();
    let loaded = repo
        .find_session(tenant, &session.id)
        .await
        .unwrap()
        .unwrap();
    let handoffs = repo
        .list_handoffs_for_session(tenant, &session.id)
        .await
        .unwrap();
    assert_eq!(loaded.status, SessionStatus::Active);
    assert_eq!(handoffs.len(), 2);
    assert!(handoffs.iter().all(|h| h.status == HandoffStatus::Pending));
}

#[tokio::test]
async fn cross_tenant_isolation_for_active_sessions() {
    let repo = repo().await;
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    inserted(&repo, tenant_a, "a").await;
    assert_eq!(repo.list_active_sessions(tenant_b).await.unwrap().len(), 0);
}

#[tokio::test]
async fn add_participant_is_idempotent_and_persists() {
    let repo = repo().await;
    let tenant = TenantId::new();
    let session = inserted(&repo, tenant, "participants").await;
    let worker = WorkerId::new();
    repo.add_participant(tenant, &session.id, worker)
        .await
        .unwrap();
    repo.add_participant(tenant, &session.id, worker)
        .await
        .unwrap();
    let loaded = repo
        .find_session(tenant, &session.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(loaded.participants, vec![worker]);
}

#[tokio::test]
async fn session_status_rejects_terminal_backslide() {
    let repo = repo().await;
    let tenant = TenantId::new();
    let session = inserted(&repo, tenant, "status").await;
    repo.update_session_status(tenant, &session.id, SessionStatus::Resolved)
        .await
        .unwrap();
    assert!(repo
        .update_session_status(tenant, &session.id, SessionStatus::Active)
        .await
        .is_err());
}

#[tokio::test]
async fn handoff_status_rejects_settled_to_pending() {
    let repo = repo().await;
    let tenant = TenantId::new();
    let session = inserted(&repo, tenant, "handoff-status").await;
    let h = handoff(&session, WorkerId::new(), WorkerId::new());
    repo.insert_handoff(tenant, &h).await.unwrap();
    repo.update_handoff_status(tenant, &h.id, HandoffStatus::Settled)
        .await
        .unwrap();
    assert!(repo
        .update_handoff_status(tenant, &h.id, HandoffStatus::Pending)
        .await
        .is_err());
}

#[tokio::test]
async fn pending_handoffs_filter_by_worker() {
    let repo = repo().await;
    let tenant = TenantId::new();
    let session = inserted(&repo, tenant, "pending").await;
    let target = WorkerId::new();
    let other = WorkerId::new();
    repo.insert_handoff(tenant, &handoff(&session, WorkerId::new(), target))
        .await
        .unwrap();
    repo.insert_handoff(tenant, &handoff(&session, WorkerId::new(), other))
        .await
        .unwrap();
    let pending = repo
        .list_pending_handoffs_for_worker(tenant, target)
        .await
        .unwrap();
    assert_eq!(pending.len(), 1);
    assert!(matches!(pending[0].to, EscalationStep::Worker(id) if id == target));
}
