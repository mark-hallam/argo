//! Unit tests for the per-step [`HybridWorkflowRunner`].
//!
//! These pin the NO_THEATER routing contract WITHOUT a live MCP
//! subprocess by injecting stub `argo_workflow` seams (a recording
//! [`ToolDispatcher`], a recording [`WorkflowAuditEmitter`], an allow-all
//! [`WorkflowCapabilityChecker`]) into the real [`WorkerSessionRunner`]
//! that the hybrid wraps:
//!
//! 1. A `tool_call`-shaped step whose `worker` resolves to a known MCP
//!    server takes the REAL path — the dispatcher is invoked (keyed by
//!    the request tenant), the output carries the tool reply under
//!    `response` (NOT a MockRunner `echo`), and exactly one real
//!    per-step audit row lands.
//! 2. A prompt/skill step (no `tool`) degrades to the deterministic
//!    MockRunner echo — the dispatcher is never touched, no real audit
//!    row lands.
//! 3. A step naming an UNKNOWN server degrades to echo even though it
//!    carries a tool (server-resolution gate).
//!
//! The end-to-end happy-path MCP dispatch over a real stdio subprocess is
//! covered by `crates/argo-mcp/tests/*` + the `argo-workflow` stub-seam
//! tests; here we pin the gateway-side hybrid routing.

use super::{build_hybrid_runner, HybridWorkflowRunner};
use argo_types::{Principal, TenantId};
use argo_workflow::{
    MockRunner, StepAuditContext, TaskNode, TaskOutcome, TaskRunner, ToolDispatchError,
    ToolDispatcher, WorkerSessionRunner, WorkflowAuditEmitter, WorkflowCapabilityChecker,
};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

/// Recording [`ToolDispatcher`] stub — captures the tenant + (server,
/// tool) it was called with and returns a canned reply. Never spawns a
/// subprocess.
struct StubDispatcher {
    seen_tenant: Mutex<Option<TenantId>>,
    calls: Mutex<Vec<(String, String)>>,
}

impl StubDispatcher {
    fn new() -> Self {
        Self {
            seen_tenant: Mutex::new(None),
            calls: Mutex::new(Vec::new()),
        }
    }
}

#[async_trait]
impl ToolDispatcher for StubDispatcher {
    async fn call(
        &self,
        tenant_id: &TenantId,
        server: &str,
        tool: &str,
        _args: Value,
    ) -> Result<Value, ToolDispatchError> {
        *self.seen_tenant.lock().expect("tenant lock") = Some(*tenant_id);
        self.calls
            .lock()
            .expect("calls lock")
            .push((server.to_string(), tool.to_string()));
        Ok(json!({ "dispatched": true, "server": server, "tool": tool }))
    }
}

/// Recording [`WorkflowAuditEmitter`] stub — records the task ids it saw
/// an executed/failed row for.
struct RecordingAudit {
    executed: Mutex<Vec<String>>,
    failed: Mutex<Vec<String>>,
}

impl RecordingAudit {
    fn new() -> Self {
        Self {
            executed: Mutex::new(Vec::new()),
            failed: Mutex::new(Vec::new()),
        }
    }
}

#[async_trait]
impl WorkflowAuditEmitter for RecordingAudit {
    async fn step_executed(
        &self,
        ctx: &StepAuditContext<'_>,
        _response: &Value,
    ) -> Result<(), String> {
        self.executed
            .lock()
            .expect("executed lock")
            .push(ctx.task_id.to_string());
        Ok(())
    }

    async fn step_failed(&self, ctx: &StepAuditContext<'_>, _reason: &str) {
        self.failed
            .lock()
            .expect("failed lock")
            .push(ctx.task_id.to_string());
    }
}

/// Allow-all [`WorkflowCapabilityChecker`] — the cap gate is exercised in
/// `argo_agent_runner::workflow_adapters_tests`; here we isolate routing.
struct AllowAllCaps;

impl WorkflowCapabilityChecker for AllowAllCaps {
    fn check_dispatch(&self, _principal: &Principal, _tool: &str) -> Result<(), String> {
        Ok(())
    }
}

fn gateway_principal() -> Principal {
    Principal::Gateway {
        adapter: "web".into(),
        identity: "operator-1".into(),
    }
}

/// Build a hybrid runner whose real path uses the recording stubs. The
/// private fields are reachable because this is a child module of the one
/// that defines [`HybridWorkflowRunner`].
fn hybrid_with_stubs(
    servers: &[&str],
    dispatcher: Arc<StubDispatcher>,
    audit: Arc<RecordingAudit>,
    tenant: TenantId,
) -> HybridWorkflowRunner {
    let real = WorkerSessionRunner::new(
        dispatcher,
        audit,
        Arc::new(AllowAllCaps),
        gateway_principal(),
        tenant,
    );
    HybridWorkflowRunner {
        real,
        mock: MockRunner::echo(),
        servers: servers.iter().map(|s| s.to_string()).collect(),
    }
}

fn tool_task(id: &str, worker: &str, tool: Option<&str>) -> TaskNode {
    let mut inputs = BTreeMap::new();
    if let Some(t) = tool {
        inputs.insert("tool".to_string(), Value::String(t.to_string()));
    }
    TaskNode {
        id: id.to_string(),
        worker: worker.to_string(),
        persona: None,
        inputs,
        outputs: BTreeMap::new(),
        depends_on: vec![],
        cache_key: None,
    }
}

#[tokio::test]
async fn tool_call_step_with_known_server_takes_real_path_and_audits() {
    let tenant = TenantId::new();
    let dispatcher = Arc::new(StubDispatcher::new());
    let audit = Arc::new(RecordingAudit::new());
    let runner = hybrid_with_stubs(&["notes"], dispatcher.clone(), audit.clone(), tenant);

    let task = tool_task("step-1", "notes", Some("add"));
    let outcome = runner.run(&task, &BTreeMap::new()).await;

    match outcome {
        TaskOutcome::Success { outputs } => {
            // The real WorkerSessionRunner stores the tool reply under
            // `response`; a MockRunner echo would key it under `echo`.
            assert!(
                outputs.contains_key("response"),
                "real path must store the tool reply under `response`; got {outputs:?}",
            );
            assert!(
                !outputs.contains_key("echo"),
                "real path must NOT be a MockRunner echo; got {outputs:?}",
            );
        }
        other => panic!("expected Success on the real path, got {other:?}"),
    }

    // Exactly one REAL per-step audit row (not an echo row).
    assert_eq!(
        audit.executed.lock().expect("executed").as_slice(),
        &["step-1".to_string()],
    );
    assert!(audit.failed.lock().expect("failed").is_empty());

    // Tenant isolation: the dispatcher was keyed by the request tenant.
    assert_eq!(
        *dispatcher.seen_tenant.lock().expect("tenant"),
        Some(tenant),
        "dispatcher must be invoked with the request tenant",
    );
    assert_eq!(
        dispatcher.calls.lock().expect("calls").as_slice(),
        &[("notes".to_string(), "add".to_string())],
    );
}

#[tokio::test]
async fn prompt_step_without_tool_degrades_to_mock_echo() {
    let tenant = TenantId::new();
    let dispatcher = Arc::new(StubDispatcher::new());
    let audit = Arc::new(RecordingAudit::new());
    let runner = hybrid_with_stubs(&["notes"], dispatcher.clone(), audit.clone(), tenant);

    // A prompt-kind step: empty worker, no tool.
    let task = tool_task("prompt-1", "", None);
    let outcome = runner.run(&task, &BTreeMap::new()).await;

    match outcome {
        TaskOutcome::Success { outputs } => assert!(
            outputs.contains_key("echo"),
            "prompt step must fall back to the MockRunner echo; got {outputs:?}",
        ),
        other => panic!("expected Success on the echo fallback, got {other:?}"),
    }

    // The real seams were never touched — no theater, no real audit row.
    assert!(dispatcher.calls.lock().expect("calls").is_empty());
    assert!(audit.executed.lock().expect("executed").is_empty());
}

#[tokio::test]
async fn tool_call_step_with_unknown_server_degrades_to_mock_echo() {
    let tenant = TenantId::new();
    let dispatcher = Arc::new(StubDispatcher::new());
    let audit = Arc::new(RecordingAudit::new());
    // Server map only knows "notes"; the step names "ghost".
    let runner = hybrid_with_stubs(&["notes"], dispatcher.clone(), audit.clone(), tenant);

    let task = tool_task("step-x", "ghost", Some("add"));
    let outcome = runner.run(&task, &BTreeMap::new()).await;

    match outcome {
        TaskOutcome::Success { outputs } => assert!(
            outputs.contains_key("echo"),
            "unknown-server step must fall back to echo, not error; got {outputs:?}",
        ),
        other => panic!("expected echo Success for unknown server, got {other:?}"),
    }
    assert!(
        dispatcher.calls.lock().expect("calls").is_empty(),
        "an unknown server must never reach the dispatcher",
    );
}

#[tokio::test]
async fn build_hybrid_runner_assembles_against_on_disk_verticals() {
    // Smoke: assembling the production hybrid over the real on-disk
    // verticals + an in-process capability engine must succeed. (No MCP
    // subprocess is spawned at assembly time — only at dispatch.)
    let tenant = TenantId::new();
    let store = Arc::new(
        argo_store::SessionStore::open(":memory:")
            .await
            .expect("open in-memory store"),
    );
    let runner = build_hybrid_runner(&store, &gateway_principal(), tenant);
    assert!(
        runner.is_ok(),
        "hybrid runner assembly failed: {:?}",
        runner.err(),
    );
}
