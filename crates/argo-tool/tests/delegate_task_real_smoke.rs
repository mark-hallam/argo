//! Integration smoke for the wired `delegate_task` tool.
//!
//! Brief: `mission-delegate-task-make-real`. Open an in-memory
//! [`SessionStore`], stand up an [`OrchestratorStore`] over it,
//! pre-insert a session row, then drive the wired
//! [`DelegateTask`] through the public [`Tool`] trait and confirm
//! the orchestrator now holds a Pending [`Handoff`] for the target
//! worker.

use std::sync::Arc;

use argo_identity::WorkerId;
use argo_orchestrator::{EscalationStep, HandoffStatus, OrchestratorStore, Session, SessionId};
use argo_store::SessionStore;
use argo_tool::{DelegateTask, Tool, ToolContext};
use argo_types::{ContentBlock, Principal, TenantId};

fn text_block(result: &argo_types::ToolCallResult) -> String {
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
async fn delegate_task_dispatch_emits_pending_handoff_in_orchestrator_store() {
    let session_store = SessionStore::open(":memory:").await.expect("open store");
    session_store.migrate().await.expect("migrate");
    let store = Arc::new(OrchestratorStore::new(Arc::new(session_store)));
    store.ensure_schema().await.expect("ensure_schema");

    let tenant = TenantId(uuid::Uuid::nil());
    let session_id = SessionId::new();
    let mut session = Session::new(tenant, "delegate-smoke");
    session.id = session_id.clone();
    store.insert_session(&session).await.expect("insert_session");

    let target = WorkerId::new();
    let tool = DelegateTask::new(Arc::clone(&store));
    let mut ctx = ToolContext::new(
        Principal::Local {
            os_user: "smoke".into(),
        },
        std::path::PathBuf::from("/tmp"),
    );
    ctx.session_id = session_id.clone();

    let result = tool
        .dispatch(
            serde_json::json!({
                "worker_id": target.0.to_string(),
                "task": "follow up on the AEON-77 ticket",
                "payload": {"case_id": "AEON-77"},
            }),
            &ctx,
        )
        .await
        .expect("dispatch");
    assert!(!result.is_error, "expected success: {:?}", result.content);
    let handoff_id_text = text_block(&result);
    assert!(!handoff_id_text.is_empty(), "handoff id must surface");

    let handoffs = store
        .list_handoffs_for_session(tenant, &session_id)
        .await
        .expect("list_handoffs_for_session");
    assert_eq!(handoffs.len(), 1, "exactly one handoff persisted");
    let h = &handoffs[0];
    assert_eq!(h.id.to_string(), handoff_id_text);
    assert_eq!(h.status, HandoffStatus::Pending);
    assert!(matches!(h.to, EscalationStep::Worker(w) if w == target));
    assert_eq!(h.expected_outcome, "follow up on the AEON-77 ticket");
    assert_eq!(
        h.payload.get("case_id").and_then(|v| v.as_str()),
        Some("AEON-77"),
    );
}
