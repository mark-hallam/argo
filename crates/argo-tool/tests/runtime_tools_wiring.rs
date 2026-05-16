//! Runtime constructor coverage for tools needing agent-runner stores.

use std::path::PathBuf;
use std::sync::Arc;

use argo_capability::CapabilityEngine;
use argo_identity::WorkerId;
use argo_memory::MemoryStore;
use argo_orchestrator::{OrchestratorStore, Session, SessionId as OrchSessionId};
use argo_store::SessionStore;
use argo_tool::{builtin_tools_with_runtime, ToolContext, ToolRegistry};
use argo_types::{MessageRole, Principal, TenantId, ToolCallResult};

fn assert_ok(tool: &str, result: &ToolCallResult) {
    let body = format!("{:?}", result.content);
    assert!(!result.is_error, "{tool} errored: {body}");
    assert!(!body.contains("not wired"), "{tool} returned {body}");
}

#[tokio::test]
async fn runtime_constructor_wires_memory_delegate_and_session_search() {
    let root = PathBuf::from("/tmp/argo-runtime-tools-wiring");
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    let (tenant, workspace) = store
        .ensure_workspace("runtime", "/tmp/runtime")
        .await
        .unwrap();
    let session = store
        .create_session(tenant, workspace, "m", "p", None, None)
        .await
        .unwrap();
    store
        .append_message(
            session,
            MessageRole::User,
            r#"[{"type":"text","text":"runtime wiring needle"}]"#,
        )
        .await
        .unwrap();
    let orchestrator = Arc::new(OrchestratorStore::new(Arc::clone(&store)));
    orchestrator.ensure_schema().await.unwrap();
    let orch_session = OrchSessionId::new();
    let mut row = Session::new(TenantId(uuid::Uuid::nil()), "runtime");
    row.id = orch_session.clone();
    orchestrator.insert_session(&row).await.unwrap();
    let memory = Arc::new(MemoryStore::new(Arc::clone(&store)));

    let mut registry = ToolRegistry::new();
    for tool in builtin_tools_with_runtime(
        root.clone(),
        Some((memory, workspace, None)),
        None,
        orchestrator,
        Arc::clone(&store),
    ) {
        registry.register(tool);
    }
    let principal = Principal::Local {
        os_user: "runtime-tools-it".into(),
    };
    let engine = CapabilityEngine::with_defaults();
    engine.bind(principal.clone(), "owner".into()).unwrap();
    let mut ctx = ToolContext::new(principal, root);
    ctx.session_id = orch_session;

    for (name, args) in [
        (
            "memory",
            serde_json::json!({"action": "write", "entry": "MEMORY.md", "content": "wired"}),
        ),
        ("session_search", serde_json::json!({"query": "needle"})),
        (
            "delegate_task",
            serde_json::json!({"worker_id": WorkerId::new().to_string(), "task": "follow up"}),
        ),
    ] {
        let result = registry.dispatch(name, args, &ctx, &engine).await.unwrap();
        assert_ok(name, &result);
    }
}
