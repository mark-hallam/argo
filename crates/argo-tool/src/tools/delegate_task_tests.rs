//! Unit tests for [`super::DelegateTask`]. Split from `delegate_task.rs`
//! per the `memory_tests.rs` / `clarify_tests.rs` convention so the
//! tool file stays comfortably under the 500-LOC ceiling.

use super::*;
use argo_orchestrator::{HandoffStatus, OrchestratorStore, Session};
use argo_store::SessionStore;
use argo_types::{ArgoError, ContentBlock, Principal, TenantId};
use std::sync::Arc;

async fn wired_store_with_session(session_id: &argo_orchestrator::SessionId) -> Arc<OrchestratorStore> {
    let session_store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    session_store.migrate().await.unwrap();
    let store = Arc::new(OrchestratorStore::new(session_store));
    store.ensure_schema().await.unwrap();
    let mut session = Session::new(TenantId(uuid::Uuid::nil()), "delegate-task-tests");
    session.id = session_id.clone();
    store.insert_session(&session).await.unwrap();
    store
}

fn ctx_with(session_id: argo_orchestrator::SessionId) -> ToolContext {
    let mut c = ToolContext::new(
        Principal::Local {
            os_user: "delegate".into(),
        },
        std::path::PathBuf::from("/tmp"),
    );
    c.session_id = session_id;
    c
}

fn text_block(result: &ToolCallResult) -> String {
    result
        .content
        .iter()
        .find_map(|b| match b {
            ContentBlock::Text { text } => Some(text.clone()),
            _ => None,
        })
        .unwrap_or_default()
}

#[tokio::test]
async fn dispatch_success_emits_pending_handoff() {
    let session_id = argo_orchestrator::SessionId::new();
    let store = wired_store_with_session(&session_id).await;
    let tool = DelegateTask::new(Arc::clone(&store));
    let target = WorkerId::new();
    let ctx = ctx_with(session_id.clone());

    let r = tool
        .dispatch(
            serde_json::json!({
                "worker_id": target.0.to_string(),
                "task": "verify customer balance",
                "payload": {"case_id": "AEON-123"},
            }),
            &ctx,
        )
        .await
        .unwrap();
    assert!(!r.is_error, "dispatch: {:?}", r.content);
    let handoff_id = text_block(&r);
    assert!(!handoff_id.is_empty());

    let handoffs = store
        .list_handoffs_for_session(TenantId(uuid::Uuid::nil()), &session_id)
        .await
        .unwrap();
    assert_eq!(handoffs.len(), 1);
    assert_eq!(handoffs[0].id.to_string(), handoff_id);
    assert_eq!(handoffs[0].status, HandoffStatus::Pending);
    assert!(matches!(handoffs[0].to, EscalationStep::Worker(w) if w == target));
    assert_eq!(handoffs[0].expected_outcome, "verify customer balance");
}

#[tokio::test]
async fn dispatch_rejects_missing_target() {
    let session_id = argo_orchestrator::SessionId::new();
    let store = wired_store_with_session(&session_id).await;
    let tool = DelegateTask::new(store);
    let ctx = ctx_with(session_id);

    let err = tool
        .dispatch(serde_json::json!({"task": "do the thing"}), &ctx)
        .await
        .unwrap_err();
    let ArgoError::Tool { tool: name, msg } = err else {
        panic!("expected Tool error");
    };
    assert_eq!(name, "delegate_task");
    assert!(
        msg.contains("invalid arguments"),
        "msg should mention invalid arguments: {msg}"
    );
}

#[tokio::test]
async fn dispatch_returns_error_on_invalid_worker_id() {
    let session_id = argo_orchestrator::SessionId::new();
    let store = wired_store_with_session(&session_id).await;
    let tool = DelegateTask::new(store);
    let ctx = ctx_with(session_id);

    let r = tool
        .dispatch(
            serde_json::json!({
                "worker_id": "not-a-uuid",
                "task": "do the thing",
            }),
            &ctx,
        )
        .await
        .unwrap();
    assert!(r.is_error, "expected dispatch error for invalid worker_id");
    assert!(text_block(&r).contains("invalid worker_id"));
}

#[tokio::test]
async fn dispatch_returns_not_wired_when_store_absent() {
    let tool = DelegateTask::default();
    let ctx = ctx_with(argo_orchestrator::SessionId::new());

    let r = tool
        .dispatch(
            serde_json::json!({
                "worker_id": WorkerId::new().0.to_string(),
                "task": "anything",
            }),
            &ctx,
        )
        .await
        .unwrap();
    assert!(r.is_error);
    assert!(
        text_block(&r).contains("not wired"),
        "expected 'not wired' diagnostic: {}",
        text_block(&r),
    );
}

#[tokio::test]
async fn dispatch_persists_deadline_when_provided() {
    let session_id = argo_orchestrator::SessionId::new();
    let store = wired_store_with_session(&session_id).await;
    let tool = DelegateTask::new(Arc::clone(&store));
    let ctx = ctx_with(session_id.clone());
    let target = WorkerId::new();

    let r = tool
        .dispatch(
            serde_json::json!({
                "worker_id": target.0.to_string(),
                "task": "finish before lunch",
                "deadline_ms": 60_000u64,
            }),
            &ctx,
        )
        .await
        .unwrap();
    assert!(!r.is_error);

    let handoffs = store
        .list_handoffs_for_session(TenantId(uuid::Uuid::nil()), &session_id)
        .await
        .unwrap();
    let h = handoffs.first().unwrap();
    let deadline = h.deadline.expect("deadline should be set");
    let now = chrono::Utc::now();
    assert!(deadline > now);
    assert!(deadline <= now + chrono::Duration::seconds(120));
}
