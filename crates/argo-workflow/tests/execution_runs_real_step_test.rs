//! Integration test for `argo-workflow` execution layer.
//!
//! `eval-workflow-execution-runs-real-step` (#2086) landed three
//! `#[ignore]`'d FINDINGS-ONLY tests pinning the contracts the
//! executor seam was missing: (1) MCP + audit invocation,
//! (2) cross-tenant refusal, (3) failure-injection. The
//! `wire-worker-session-runner` brief ships `WorkerSessionRunner`
//! which satisfies those contracts; the three tests are now
//! un-ignored and exercise the real runner against stub
//! `ToolDispatcher` / `WorkflowAuditEmitter` /
//! `WorkflowCapabilityChecker` impls (see crate-level runner docs
//! for the dep-cycle carve-out).

use argo_workflow::{
    ExecutionRequest, Executor, MockRunner, StepAuditContext, TaskNode, TaskOutcome, TaskRunner,
    TaskVerdict, ToolDispatchError, ToolDispatcher, WorkerSessionRunner, Workflow,
    WorkflowAuditEmitter, WorkflowCapabilityChecker,
};
use argo_types::{Principal, TenantId};
use async_trait::async_trait;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

// ============================================================
// Existing surface-existence smokes — kept from #2086.
// ============================================================

/// Helper: a 1-task workflow asking worker `mcp-tool-caller` to run
/// a single `tool_call`-shaped step.
fn single_step_tool_call_workflow() -> Workflow {
    let mut inputs = BTreeMap::new();
    inputs.insert("tool".into(), serde_json::Value::String("echo".into()));
    inputs.insert(
        "args".into(),
        serde_json::json!({"msg": "hello-from-test"}),
    );
    Workflow {
        id: "single-step-tool-call".into(),
        description: "1-step workflow exercising the executor's TaskRunner seam".into(),
        tasks: vec![TaskNode {
            id: "call".into(),
            worker: "mcp-tool-caller".into(),
            persona: None,
            inputs,
            outputs: BTreeMap::new(),
            depends_on: vec![],
            cache_key: None,
        }],
        edges: vec![],
    }
}

#[tokio::test]
async fn executor_surface_walks_a_single_task_and_emits_a_verdict() {
    let workflow = single_step_tool_call_workflow();
    let exec = Executor::new(MockRunner::echo());
    let report = exec
        .execute(ExecutionRequest::new(workflow))
        .await
        .expect("executor must accept a validated 1-task workflow");
    assert!(report.ok);
    assert_eq!(report.task_verdicts.len(), 1);
    match report.task_verdicts.get("call").expect("verdict") {
        TaskVerdict::Success { outputs } => {
            assert_eq!(
                outputs.get("tool"),
                Some(&serde_json::Value::String("echo".into())),
            );
            assert_eq!(
                outputs.get("echo"),
                Some(&serde_json::Value::String("call".into())),
            );
        }
        other => panic!("expected Success verdict, got {other:?}"),
    }
}

type CallLog = Arc<Mutex<Vec<(String, BTreeMap<String, serde_json::Value>)>>>;

struct RecordingRunner {
    calls: CallLog,
}

impl RecordingRunner {
    fn new() -> (Self, CallLog) {
        let calls: CallLog = Arc::new(Mutex::new(Vec::new()));
        (
            Self {
                calls: Arc::clone(&calls),
            },
            calls,
        )
    }
}

#[async_trait]
impl TaskRunner for RecordingRunner {
    async fn run(
        &self,
        task: &TaskNode,
        inputs: &BTreeMap<String, serde_json::Value>,
    ) -> TaskOutcome {
        if let Ok(mut g) = self.calls.lock() {
            g.push((task.id.clone(), inputs.clone()));
        }
        TaskOutcome::Success {
            outputs: BTreeMap::new(),
        }
    }
}

#[tokio::test]
async fn executor_hands_resolved_inputs_to_the_runner_seam() {
    let workflow = single_step_tool_call_workflow();
    let (runner, calls) = RecordingRunner::new();
    let exec = Executor::new(runner);
    let _ = exec
        .execute(ExecutionRequest::new(workflow))
        .await
        .expect("executes");
    let calls = calls.lock().expect("lock");
    assert_eq!(calls.len(), 1);
    let (id, inputs) = &calls[0];
    assert_eq!(id, "call");
    assert_eq!(
        inputs.get("tool"),
        Some(&serde_json::Value::String("echo".into())),
    );
    assert_eq!(
        inputs.get("args"),
        Some(&serde_json::json!({"msg": "hello-from-test"})),
    );
}

// ============================================================
// Brief contract tests — un-ignored as of `wire-worker-session-runner`.
// ============================================================

/// Recorded dispatcher invocation (kept in a Mutex'd Vec for assertions).
#[derive(Debug, Clone)]
struct StubCall {
    tenant_id: TenantId,
    server: String,
    tool: String,
    args: serde_json::Value,
}

/// In-test dispatcher returning either a canned success response or
/// a canned transport error.
struct StubDispatcher {
    calls: Arc<Mutex<Vec<StubCall>>>,
    behavior: DispatcherBehavior,
}

#[derive(Clone)]
enum DispatcherBehavior {
    Ok(serde_json::Value),
    Transport(String),
}

impl StubDispatcher {
    fn ok(value: serde_json::Value) -> Arc<Self> {
        Arc::new(Self {
            calls: Arc::new(Mutex::new(Vec::new())),
            behavior: DispatcherBehavior::Ok(value),
        })
    }

    fn fail(message: impl Into<String>) -> Arc<Self> {
        Arc::new(Self {
            calls: Arc::new(Mutex::new(Vec::new())),
            behavior: DispatcherBehavior::Transport(message.into()),
        })
    }

    fn calls(&self) -> Vec<StubCall> {
        self.calls.lock().expect("calls lock").clone()
    }
}

#[async_trait]
impl ToolDispatcher for StubDispatcher {
    async fn call(
        &self,
        tenant_id: &TenantId,
        server: &str,
        tool: &str,
        args: serde_json::Value,
    ) -> Result<serde_json::Value, ToolDispatchError> {
        self.calls.lock().expect("calls lock").push(StubCall {
            tenant_id: *tenant_id,
            server: server.to_string(),
            tool: tool.to_string(),
            args: args.clone(),
        });
        match &self.behavior {
            DispatcherBehavior::Ok(v) => Ok(v.clone()),
            DispatcherBehavior::Transport(m) => Err(ToolDispatchError::Transport(m.clone())),
        }
    }
}

/// Recorded audit emission ((kind, task_id, tool, reason_or_response)).
#[derive(Debug, Clone)]
struct AuditEvent {
    kind: &'static str,
    task_id: String,
    tool: String,
    reason_or_response: serde_json::Value,
    principal_label: String,
    tenant_id: TenantId,
}

struct StubAudit {
    events: Arc<Mutex<Vec<AuditEvent>>>,
}

impl StubAudit {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            events: Arc::new(Mutex::new(Vec::new())),
        })
    }

    fn events(&self) -> Vec<AuditEvent> {
        self.events.lock().expect("events lock").clone()
    }
}

fn principal_label_for(p: &Principal) -> String {
    match p {
        Principal::Local { os_user } => format!("local:{os_user}"),
        Principal::Gateway { adapter, identity } => format!("{adapter}:{identity}"),
        Principal::Acp { harness, identity } => format!("{harness}:{identity}"),
        Principal::Forge => "forge".to_string(),
    }
}

#[async_trait]
impl WorkflowAuditEmitter for StubAudit {
    async fn step_executed(
        &self,
        ctx: &StepAuditContext<'_>,
        response: &serde_json::Value,
    ) -> Result<(), String> {
        self.events.lock().expect("events lock").push(AuditEvent {
            kind: "workflow.step.executed",
            task_id: ctx.task_id.to_string(),
            tool: ctx.tool.to_string(),
            reason_or_response: response.clone(),
            principal_label: principal_label_for(ctx.principal),
            tenant_id: *ctx.tenant_id,
        });
        Ok(())
    }

    async fn step_failed(&self, ctx: &StepAuditContext<'_>, reason: &str) {
        self.events.lock().expect("events lock").push(AuditEvent {
            kind: "workflow.step.failed",
            task_id: ctx.task_id.to_string(),
            tool: ctx.tool.to_string(),
            reason_or_response: serde_json::Value::String(reason.to_string()),
            principal_label: principal_label_for(ctx.principal),
            tenant_id: *ctx.tenant_id,
        });
    }
}

/// In-test capability checker. Maps `(principal, tool)` pairs to a
/// granted-or-denied verdict.
struct StubCapabilities {
    grants: Vec<(Principal, String)>,
}

impl StubCapabilities {
    fn granting(principal: Principal, tool: impl Into<String>) -> Arc<Self> {
        Arc::new(Self {
            grants: vec![(principal, tool.into())],
        })
    }

}

impl WorkflowCapabilityChecker for StubCapabilities {
    fn check_dispatch(&self, principal: &Principal, tool: &str) -> Result<(), String> {
        let has_grant = self
            .grants
            .iter()
            .any(|(p, t)| p == principal && t == tool);
        if has_grant {
            Ok(())
        } else {
            Err(format!(
                "principal {} not bound to ToolDispatch[{tool}]",
                principal_label_for(principal),
            ))
        }
    }
}

/// Brief contract #1: happy path — the runner invokes the dispatcher
/// (the MCP-call seam) and the audit emitter sees a
/// `workflow.step.executed` row carrying the tool's response.
#[tokio::test]
async fn happy_path_runner_invokes_dispatcher_and_writes_audit_row() {
    let principal = Principal::Local {
        os_user: "alice".into(),
    };
    let tenant_id = TenantId::new();
    let dispatcher = StubDispatcher::ok(serde_json::json!({"answer": "hello-from-test"}));
    let audit = StubAudit::new();
    let capabilities = StubCapabilities::granting(principal.clone(), "echo");

    let runner = WorkerSessionRunner::new(
        dispatcher.clone(),
        audit.clone(),
        capabilities,
        principal.clone(),
        tenant_id,
    );
    let exec = Executor::new(runner);
    let report = exec
        .execute(ExecutionRequest::new(single_step_tool_call_workflow()))
        .await
        .expect("execute");

    // Dispatcher saw exactly one invocation with the expected shape.
    let calls = dispatcher.calls();
    assert_eq!(calls.len(), 1, "exactly one MCP dispatch expected");
    assert_eq!(calls[0].tenant_id, tenant_id);
    assert_eq!(calls[0].server, "mcp-tool-caller");
    assert_eq!(calls[0].tool, "echo");
    assert_eq!(calls[0].args, serde_json::json!({"msg": "hello-from-test"}));

    // Verdict carries the dispatcher's response under `outputs.response`.
    match report.task_verdicts.get("call").expect("verdict") {
        TaskVerdict::Success { outputs } => {
            assert_eq!(
                outputs.get("response"),
                Some(&serde_json::json!({"answer": "hello-from-test"})),
            );
        }
        other => panic!("expected Success, got {other:?}"),
    }
    assert!(report.ok);

    // Audit: exactly one executed row, carrying the response.
    let events = audit.events();
    assert_eq!(events.len(), 1, "exactly one audit event expected");
    assert_eq!(events[0].kind, "workflow.step.executed");
    assert_eq!(events[0].task_id, "call");
    assert_eq!(events[0].tool, "echo");
    assert_eq!(events[0].principal_label, "local:alice");
    assert_eq!(events[0].tenant_id, tenant_id);
    assert_eq!(
        events[0].reason_or_response,
        serde_json::json!({"answer": "hello-from-test"})
    );
}

/// Brief contract #2: cross-tenant negative — a principal the
/// capability checker never bound is refused at the runner's pre-
/// dispatch gate. The dispatcher MUST NOT see the call (the load-
/// bearing RBAC-bypass guard).
#[tokio::test]
async fn cross_tenant_runner_refuses_foreign_principal() {
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    assert_ne!(tenant_a, tenant_b);

    let principal_a = Principal::Local {
        os_user: "alice-tenant-a".into(),
    };
    let principal_b = Principal::Local {
        os_user: "bob-tenant-b".into(),
    };

    // Capability checker grants `echo` to tenant_a's principal only;
    // when the runner is bound to tenant_b's principal it must
    // refuse.
    let capabilities = StubCapabilities::granting(principal_a, "echo");
    let dispatcher = StubDispatcher::ok(serde_json::json!({"never": "called"}));
    let audit = StubAudit::new();

    let runner = WorkerSessionRunner::new(
        dispatcher.clone(),
        audit.clone(),
        capabilities,
        principal_b,
        tenant_b,
    );
    let exec = Executor::new(runner);
    let report = exec
        .execute(ExecutionRequest::new(single_step_tool_call_workflow()))
        .await
        .expect("execute");

    // Dispatcher must NOT have been called — capability check fires
    // BEFORE MCP dispatch (the load-bearing assertion).
    assert!(
        dispatcher.calls().is_empty(),
        "cross-tenant principal must be refused before MCP dispatch; got calls={:?}",
        dispatcher.calls()
    );

    // Verdict is Failed with a capability-denied reason.
    match report.task_verdicts.get("call").expect("verdict") {
        TaskVerdict::Failed { reason } => {
            assert!(
                reason.starts_with("capability denied"),
                "expected capability-denied reason, got {reason:?}"
            );
        }
        other => panic!("expected Failed verdict, got {other:?}"),
    }
    assert!(!report.ok);

    // Audit: exactly one failed row.
    let events = audit.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, "workflow.step.failed");
    assert_eq!(events[0].tool, "echo");
    assert_eq!(events[0].tenant_id, tenant_b);
    let reason = events[0]
        .reason_or_response
        .as_str()
        .expect("reason is string");
    assert!(reason.contains("capability denied"), "got {reason:?}");
}

/// Brief contract #3: failure-injection — the dispatcher returns a
/// transport error (the moral equivalent of "mock MCP returns 500").
/// The audit emitter sees a step-failed row carrying the upstream
/// message AND the verdict is `Failed` (`report.ok == false`). The
/// failure is NOT silently swallowed.
#[tokio::test]
async fn failure_injection_dispatcher_500_marks_step_failed_and_writes_audit() {
    let principal = Principal::Local {
        os_user: "alice".into(),
    };
    let tenant_id = TenantId::new();
    let dispatcher = StubDispatcher::fail("upstream MCP server returned 500");
    let audit = StubAudit::new();
    let capabilities = StubCapabilities::granting(principal.clone(), "echo");

    let runner = WorkerSessionRunner::new(
        dispatcher.clone(),
        audit.clone(),
        capabilities,
        principal,
        tenant_id,
    );
    let exec = Executor::new(runner);
    let report = exec
        .execute(ExecutionRequest::new(single_step_tool_call_workflow()))
        .await
        .expect("execute");

    // Dispatcher was called once (failure happened upstream of the
    // runner, not before it).
    assert_eq!(dispatcher.calls().len(), 1);

    // Verdict is Failed, reason carries the upstream message.
    match report.task_verdicts.get("call").expect("verdict") {
        TaskVerdict::Failed { reason } => {
            assert!(
                reason.contains("upstream MCP server returned 500"),
                "expected upstream message in reason, got {reason:?}"
            );
        }
        other => panic!("expected Failed verdict, got {other:?}"),
    }
    assert!(!report.ok, "run ok must be false on MCP failure");

    // Audit: exactly one failed row.
    let events = audit.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, "workflow.step.failed");
    let reason = events[0]
        .reason_or_response
        .as_str()
        .expect("reason is string");
    assert!(
        reason.contains("upstream MCP server returned 500"),
        "got {reason:?}"
    );
}

