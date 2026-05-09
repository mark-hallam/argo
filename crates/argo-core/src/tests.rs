//! End-to-end walking-skeleton tests against an in-memory store + a
//! mock provider transport. Covers the §4.F gate:
//!
//! > user prompt → openai-compatible call → file_read tool dispatch →
//! > assistant final answer
//!
//! Steer-channel tests live in `tests_steer.rs`; resume-on-restart
//! tests live in `tests_resume.rs`. Both modules are loaded as
//! children below and reuse the helpers defined here.
use super::*;
use argo_capability::CapabilityEngine;
use argo_provider::{
    ProviderRequest, ProviderResponse, ProviderResponseDelta, Sampling, StopReason, Transport,
};
use argo_store::SessionStore;
use argo_tool::{builtin_tools, ToolRegistry};
use argo_types::{
    ArgoError, ArgoResult, CapabilitySet, Principal, SessionId, TenantId, ToolCallId,
    ToolCallRequest, WorkspaceId,
};
use async_trait::async_trait;
use futures::stream::BoxStream;
use parking_lot::Mutex;
use std::sync::Arc;

/// Scripted transport: emits the next entry per `complete()` call.
struct ScriptedTransport {
    script: Mutex<std::collections::VecDeque<ProviderResponse>>,
    seen: std::sync::atomic::AtomicUsize,
}

impl ScriptedTransport {
    fn new(script: Vec<ProviderResponse>) -> Arc<Self> {
        Arc::new(Self {
            script: Mutex::new(script.into_iter().collect()),
            seen: std::sync::atomic::AtomicUsize::new(0),
        })
    }
}

#[async_trait]
impl Transport for ScriptedTransport {
    fn id(&self) -> &str {
        "mock-provider"
    }
    fn capabilities(&self) -> &CapabilitySet {
        static EMPTY: std::sync::OnceLock<CapabilitySet> = std::sync::OnceLock::new();
        EMPTY.get_or_init(CapabilitySet::empty)
    }
    async fn complete(&self, _req: &ProviderRequest) -> ArgoResult<ProviderResponse> {
        self.seen.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.script
            .lock()
            .pop_front()
            .ok_or_else(|| ArgoError::Provider("script exhausted".into()))
    }
    async fn complete_stream(
        &self,
        _req: &ProviderRequest,
    ) -> ArgoResult<BoxStream<'static, ArgoResult<ProviderResponseDelta>>> {
        Err(ArgoError::Provider("not used".into()))
    }
}

fn text_response(text: &str) -> ProviderResponse {
    ProviderResponse {
        text: Some(text.into()),
        reasoning: None,
        tool_calls: vec![],
        stop_reason: StopReason::EndTurn,
        tokens_in: Some(10),
        tokens_out: Some(3),
        provider_meta: serde_json::Value::Null,
    }
}

fn tool_call_response(name: &str, args: serde_json::Value) -> ProviderResponse {
    ProviderResponse {
        text: None,
        reasoning: None,
        tool_calls: vec![ToolCallRequest {
            id: ToolCallId::new(),
            provider_call_id: Some("call_test".into()),
            name: name.into(),
            arguments: args,
        }],
        stop_reason: StopReason::ToolUse,
        tokens_in: Some(15),
        tokens_out: Some(8),
        provider_meta: serde_json::Value::Null,
    }
}

async fn fresh_deps(transport: Arc<dyn Transport>) -> (AgentDeps, SessionId, std::path::PathBuf) {
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    let tenant = TenantId::new();
    let ws = WorkspaceId::new();
    let workspace_root = tempfile::tempdir().unwrap().keep();
    sqlx::query(
        "INSERT INTO workspace (id, tenant_id, name, root_path, created_at)
         VALUES (?, ?, 'test', ?, strftime('%s','now'))",
    )
    .bind(ws.0.as_bytes().as_slice())
    .bind(tenant.0.as_bytes().as_slice())
    .bind(workspace_root.to_string_lossy().to_string())
    .execute(store.pool())
    .await
    .unwrap();
    let session_id = store
        .create_session(tenant, ws, "model", "provider", Some("t"), None)
        .await
        .unwrap();
    let mut registry = ToolRegistry::new();
    for t in builtin_tools(workspace_root.clone()) {
        registry.register(t);
    }
    let engine = CapabilityEngine::with_defaults();
    engine
        .bind(
            Principal::Local {
                os_user: "u".into(),
            },
            "owner".into(),
        )
        .unwrap();
    let audit = Arc::new(AuditSink::new(store.clone()));
    let steer = Arc::new(SteerChannel::new());
    let deps = AgentDeps {
        store,
        transport,
        registry: Arc::new(registry),
        capability_engine: Arc::new(engine),
        audit,
        steer,
    };
    (deps, session_id, workspace_root)
}

fn run_opts(workspace_root: std::path::PathBuf) -> RunTurnOptions {
    RunTurnOptions {
        max_iterations: 8,
        system: Some("be terse".into()),
        principal: Principal::Local {
            os_user: "u".into(),
        },
        workspace_root,
        model: "mock".into(),
        sampling: Sampling::default(),
    }
}

#[tokio::test]
async fn happy_path_text_only_turn() {
    let transport = ScriptedTransport::new(vec![text_response("hello back")]);
    let (deps, session_id, ws) = fresh_deps(transport).await;
    let agent = Agent::new(deps);
    let outcome = agent
        .run_turn(session_id, "hi".into(), run_opts(ws))
        .await
        .unwrap();
    assert_eq!(outcome.final_text, "hello back");
    assert_eq!(outcome.iterations, 1);
    assert_eq!(outcome.tokens_in, 10);
    assert_eq!(outcome.tokens_out, 3);
}

#[tokio::test]
async fn happy_path_user_to_file_read_to_final_answer() {
    let dir = tempfile::tempdir().unwrap();
    tokio::fs::write(dir.path().join("hello.txt"), "secret value")
        .await
        .unwrap();
    let transport = ScriptedTransport::new(vec![
        // Iteration 1: assistant calls file_read.
        tool_call_response("file_read", serde_json::json!({"path": "hello.txt"})),
        // Iteration 2: assistant produces the final text.
        text_response("the file contains: secret value"),
    ]);
    let (deps, session_id, _) = fresh_deps(transport).await;
    let agent = Agent::new(deps);
    let opts = run_opts(dir.keep());
    let outcome = agent
        .run_turn(session_id, "what's in hello.txt?".into(), opts)
        .await
        .unwrap();
    assert_eq!(outcome.final_text, "the file contains: secret value");
    assert_eq!(outcome.iterations, 2);
}

#[tokio::test]
async fn audit_log_captures_turn_boundaries_and_tool_dispatch() {
    let dir = tempfile::tempdir().unwrap();
    tokio::fs::write(dir.path().join("a.txt"), "x")
        .await
        .unwrap();
    let transport = ScriptedTransport::new(vec![
        tool_call_response("file_read", serde_json::json!({"path": "a.txt"})),
        text_response("done"),
    ]);
    let (deps, session_id, _) = fresh_deps(transport).await;
    let agent = Agent::new(deps);
    let _ = agent
        .run_turn(session_id, "do it".into(), run_opts(dir.keep()))
        .await
        .unwrap();
    // Expect: turn_start + 2 assistant_message + 1 tool_call + 1 tool_result + turn_end = 6 rows.
    let n = agent.deps().audit.count_for(session_id).await.unwrap();
    assert_eq!(n, 6);
}

#[tokio::test]
async fn max_iterations_halts_with_budget_exhausted() {
    // Endless tool-call loop — agent must halt at max_iterations.
    let dir = tempfile::tempdir().unwrap();
    tokio::fs::write(dir.path().join("a.txt"), "x")
        .await
        .unwrap();
    let mut script = Vec::new();
    for _ in 0..10 {
        script.push(tool_call_response(
            "file_read",
            serde_json::json!({"path": "a.txt"}),
        ));
    }
    let transport = ScriptedTransport::new(script);
    let (deps, session_id, _) = fresh_deps(transport).await;
    let agent = Agent::new(deps);
    let mut opts = run_opts(dir.keep());
    opts.max_iterations = 3;
    let err = agent
        .run_turn(session_id, "loop forever".into(), opts)
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        ArgoError::BudgetExhausted {
            kind: argo_types::BudgetKind::Iterations
        }
    ));
}

#[tokio::test]
async fn capability_denied_propagates_through_loop() {
    // Bind a guest principal — guest doesn't have FsRead{**}.
    let dir = tempfile::tempdir().unwrap();
    tokio::fs::write(dir.path().join("x.txt"), "x")
        .await
        .unwrap();
    let transport = ScriptedTransport::new(vec![tool_call_response(
        "file_read",
        serde_json::json!({"path": "x.txt"}),
    )]);
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
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
    let session_id = store
        .create_session(tenant, ws, "model", "provider", Some("t"), None)
        .await
        .unwrap();
    let mut registry = ToolRegistry::new();
    for t in builtin_tools(dir.path().to_path_buf()) {
        registry.register(t);
    }
    let engine = CapabilityEngine::with_defaults();
    engine
        .bind(
            Principal::Local {
                os_user: "g".into(),
            },
            "guest".into(),
        )
        .unwrap();
    let audit = Arc::new(AuditSink::new(store.clone()));
    let deps = AgentDeps {
        store,
        transport,
        registry: Arc::new(registry),
        capability_engine: Arc::new(engine),
        audit,
        steer: Arc::new(SteerChannel::new()),
    };
    let mut opts = run_opts(dir.keep());
    opts.principal = Principal::Local {
        os_user: "g".into(),
    };
    let agent = Agent::new(deps);
    let err = agent
        .run_turn(session_id, "read x.txt".into(), opts)
        .await
        .unwrap_err();
    assert!(matches!(err, ArgoError::RbacDenied { .. }));
}

#[tokio::test]
async fn messages_persist_to_session_store() {
    let transport = ScriptedTransport::new(vec![text_response("hi")]);
    let (deps, session_id, ws) = fresh_deps(transport).await;
    let agent = Agent::new(deps);
    let _ = agent
        .run_turn(session_id, "hi".into(), run_opts(ws))
        .await
        .unwrap();
    let n = agent.deps().store.message_count(session_id).await.unwrap();
    // user + assistant = 2 messages persisted for a text-only turn.
    assert_eq!(n, 2);
}

#[path = "tests_steer.rs"]
mod steer_tests;

#[path = "tests_resume.rs"]
mod resume_tests;
