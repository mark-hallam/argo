//! Runtime wire-up for the in-core `memory` tool's namespace binding.
//!
//! Proves that the agent-runner's `builtin_tools_with_runtime`
//! constructor — the production path consumed by `build_dispatch_ctx`
//! — actually binds the tool to a `MemoryNs` when one is supplied,
//! so `query_scored` can reach the per-worker namespace store and a
//! second worker cannot see the first worker's private rows. Without
//! the wire, the tool falls back to `Memory::new` and `query_scored`
//! returns the "not bound to a worker namespace" error.
//!
//! Brief: `codex-memory-tool-namespace-wiring` (F-03 P1).

use std::path::PathBuf;
use std::sync::Arc;

use argo_identity::{MemoryNs, MemoryScope, WorkerId};
use argo_memory::{MemoryFile, MemoryLayer, MemoryStore};
use argo_orchestrator::OrchestratorStore;
use argo_store::SessionStore;
use argo_tool::{builtin_tools_with_runtime, Tool, ToolContext};
use argo_types::{ContentBlock, Principal, TenantId, ToolCallResult, WorkspaceId};

struct Fixture {
    memory: Arc<MemoryStore>,
    orchestrator: Arc<OrchestratorStore>,
    session_store: Arc<SessionStore>,
    workspace: WorkspaceId,
    tenant: TenantId,
}

async fn fresh() -> Fixture {
    let session_store = Arc::new(SessionStore::open(":memory:").await.expect("open"));
    session_store.migrate().await.expect("migrate");
    let (tenant, workspace) = session_store
        .ensure_workspace("ns-runtime", "/tmp/ns-runtime")
        .await
        .expect("ensure_workspace");
    let memory = Arc::new(MemoryStore::new(Arc::clone(&session_store)));
    let orchestrator = Arc::new(OrchestratorStore::new(Arc::clone(&session_store)));
    Fixture {
        memory,
        orchestrator,
        session_store,
        workspace,
        tenant,
    }
}

fn memory_tool(
    fx: &Fixture,
    namespace: Option<MemoryNs>,
) -> Arc<dyn Tool> {
    let tools = builtin_tools_with_runtime(
        PathBuf::from("/tmp/argo-memory-namespace-runtime"),
        Some((Arc::clone(&fx.memory), fx.workspace, None)),
        namespace,
        Arc::clone(&fx.orchestrator),
        Arc::clone(&fx.session_store),
    );
    tools
        .into_iter()
        .find(|t| t.name() == "memory")
        .expect("memory tool registered")
}

fn ctx(tenant: TenantId) -> ToolContext {
    ToolContext::new_with_tenant(
        Principal::Local {
            os_user: "ns-runtime-it".into(),
        },
        PathBuf::from("/tmp/argo-memory-namespace-runtime"),
        tenant,
    )
}

fn text_body(result: &ToolCallResult) -> String {
    result
        .content
        .iter()
        .find_map(|b| match b {
            ContentBlock::Text { text } => Some(text.clone()),
            _ => None,
        })
        .unwrap_or_default()
}

async fn seed_indexed(memory: &MemoryStore, ns: &MemoryNs, ws: WorkspaceId, body: &str) {
    memory
        .append_in_namespace_with_layer(
            ns,
            ws,
            MemoryFile::Memory,
            body.to_string(),
            None,
            MemoryLayer::Indexed,
        )
        .await
        .expect("seed indexed row");
}

#[tokio::test]
async fn runtime_namespace_bind_lets_query_scored_succeed() {
    let fx = fresh().await;
    let worker = WorkerId::new();
    let ns = MemoryNs::new(fx.tenant, worker, MemoryScope::WorkerSelf);

    seed_indexed(&fx.memory, &ns, fx.workspace, "alpha alpha alpha").await;
    seed_indexed(&fx.memory, &ns, fx.workspace, "alpha beta").await;

    let tool = memory_tool(&fx, Some(ns));
    let result = tool
        .dispatch(
            serde_json::json!({"action": "query_scored", "query": "alpha", "limit": 3}),
            &ctx(fx.tenant),
        )
        .await
        .expect("dispatch ok");
    assert!(
        !result.is_error,
        "query_scored must succeed under a bound namespace: {:?}",
        result.content,
    );
    let body = text_body(&result);
    let hits: Vec<serde_json::Value> =
        serde_json::from_str(&body).expect("query_scored envelope must be JSON");
    assert!(!hits.is_empty(), "expected at least one hit, got: {body}");
}

#[tokio::test]
async fn runtime_namespace_isolates_workers_in_same_tenant() {
    let fx = fresh().await;
    let worker_a = WorkerId::new();
    let worker_b = WorkerId::new();
    let ns_a = MemoryNs::new(fx.tenant, worker_a, MemoryScope::WorkerSelf);
    let ns_b = MemoryNs::new(fx.tenant, worker_b, MemoryScope::WorkerSelf);

    // Worker A persists a private note in its own namespace.
    seed_indexed(&fx.memory, &ns_a, fx.workspace, "private alpha note").await;

    let tool_a = memory_tool(&fx, Some(ns_a.clone()));
    let tool_b = memory_tool(&fx, Some(ns_b));

    // Worker A finds its own row.
    let a = tool_a
        .dispatch(
            serde_json::json!({"action": "query_scored", "query": "alpha"}),
            &ctx(fx.tenant),
        )
        .await
        .expect("dispatch ok");
    assert!(!a.is_error, "A query: {:?}", a.content);
    let a_hits: Vec<serde_json::Value> =
        serde_json::from_str(&text_body(&a)).expect("A envelope JSON");
    assert_eq!(
        a_hits.len(),
        1,
        "A must see its own row, got: {a_hits:?}"
    );

    // Worker B in the same tenant must not see A's private row.
    let b = tool_b
        .dispatch(
            serde_json::json!({"action": "query_scored", "query": "alpha"}),
            &ctx(fx.tenant),
        )
        .await
        .expect("dispatch ok");
    assert!(!b.is_error, "B query: {:?}", b.content);
    let b_hits: Vec<serde_json::Value> =
        serde_json::from_str(&text_body(&b)).expect("B envelope JSON");
    assert!(
        b_hits.is_empty(),
        "B must not see A's worker_self row, got: {b_hits:?}",
    );
}

#[tokio::test]
async fn runtime_namespace_unbound_errors_with_actionable_hint() {
    let fx = fresh().await;
    let tool = memory_tool(&fx, None);
    let err = tool
        .dispatch(
            serde_json::json!({"action": "query_scored", "query": "anything"}),
            &ctx(fx.tenant),
        )
        .await
        .expect_err("query_scored without a bound namespace must error");
    let msg = err.to_string();
    assert!(
        msg.contains("Memory::with_namespace"),
        "error must name the with_namespace constructor: {msg}",
    );
}
