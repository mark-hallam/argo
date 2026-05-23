//! Mission claim 3 — LIVE regression test:
//!
//! "The platform builds workers. Genesis turns plain-English role →
//! vertical (system prompt, action vocabulary, memory schema, eval
//! seeds)." (MISSION.md.)
//!
//! Source: `docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-manifest.md`
//! row `Claim 3 — The platform builds workers (Genesis)` — Genesis
//! end-to-end smoke is 🟢, and `argo-workflow` runtime is 🟡
//! AUDITED 2026-05-23 with `WorkerSessionRunner` shipped via
//! `wire-worker-session-runner` (#2089).
//!
//! What this test pins is the **seam between the two** — the
//! load-bearing assertion behind mission claim 3:
//!
//! 1. `argo_genesis::plan()` on a plain-English charter returns a
//!    `GenesisPlan` that materialises a valid `verticals/<id>/`
//!    tree (`vertical.toml` + `worker.toml` + `prompts/...` +
//!    `memory/schema.toml` + `eval/seed.jsonl` + `eval-seed.toml`).
//! 2. The generated vertical's action vocabulary carries at least
//!    one auto-approved action that can be dispatched via the
//!    `argo-workflow` `WorkerSessionRunner` against an MCP
//!    server with the vertical's identity as the pool key.
//! 3. Running a 1-task workflow that mirrors one of the vertical's
//!    declared `action_type` names against a stub MCP dispatcher
//!    + stub audit emitter + stub capability checker yields a
//!      `Success` verdict AND exactly one `workflow.step.executed`
//!      audit row carrying the tool's response.
//! 4. The cross-tenant negative: a foreign-tenant principal cannot
//!    dispatch the same step — the runner refuses at the
//!    capability gate, the dispatcher is never reached, and the
//!    audit row is `workflow.step.failed` with a
//!    `capability denied` reason.
//!
//! Scope deliberately narrow — Genesis does NOT today produce a
//! `Workflow` DAG (its output is a vertical manifest + action
//! vocabulary). The test bridges the two by constructing a
//! workflow that names one of the vertical's actions as the MCP
//! tool, which mirrors how an operator-authored vertical's actions
//! reach the dispatcher in production. The mock MCP stays
//! constant across both paths so the assertion is on the seam,
//! not on Genesis output content.

use argo_genesis::{plan, write_vertical};
use argo_types::{Principal, TenantId};
use argo_vertical::{ActionRiskLevel, VerticalManifest};
use argo_workflow::{
    ExecutionRequest, Executor, StepAuditContext, TaskNode, TaskVerdict, ToolDispatchError,
    ToolDispatcher, WorkerSessionRunner, Workflow, WorkflowAuditEmitter,
    WorkflowCapabilityChecker,
};
use async_trait::async_trait;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

/// Plain-English charter chosen to deterministically classify into
/// `Archetype::MarketingIntern` (the verbatim "marketing intern"
/// example from `MISSION.md`).
const CHARTER: &str =
    "Be my marketing intern. Each week pick a topic, draft a campaign brief, \
     and run a follow-up sequence to my warm leads. I want to approve before \
     anything goes out.";

const PERSONA: &str = "maya";

// ----------------------------------------------------------------
// Stubs (`WorkerSessionRunner` dependency seams).
//
// These mirror the stubs in `execution_runs_real_step_test.rs` but
// stay file-local so the two suites can evolve independently.
// ----------------------------------------------------------------

#[derive(Debug, Clone)]
struct StubCall {
    tenant_id: TenantId,
    server: String,
    tool: String,
    args: serde_json::Value,
}

struct StubDispatcher {
    calls: Arc<Mutex<Vec<StubCall>>>,
    response: serde_json::Value,
}

impl StubDispatcher {
    fn ok(response: serde_json::Value) -> Arc<Self> {
        Arc::new(Self {
            calls: Arc::new(Mutex::new(Vec::new())),
            response,
        })
    }

    fn calls(&self) -> Vec<StubCall> {
        self.calls.lock().expect("dispatcher calls lock").clone()
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
            args,
        });
        Ok(self.response.clone())
    }
}

#[derive(Debug, Clone)]
struct AuditEvent {
    kind: &'static str,
    task_id: String,
    tool: String,
    response_or_reason: serde_json::Value,
    tenant_id: TenantId,
    principal_label: String,
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
        self.events.lock().expect("audit events lock").clone()
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
            response_or_reason: response.clone(),
            tenant_id: *ctx.tenant_id,
            principal_label: principal_label_for(ctx.principal),
        });
        Ok(())
    }

    async fn step_failed(&self, ctx: &StepAuditContext<'_>, reason: &str) {
        self.events.lock().expect("events lock").push(AuditEvent {
            kind: "workflow.step.failed",
            task_id: ctx.task_id.to_string(),
            tool: ctx.tool.to_string(),
            response_or_reason: serde_json::Value::String(reason.to_string()),
            tenant_id: *ctx.tenant_id,
            principal_label: principal_label_for(ctx.principal),
        });
    }
}

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
        let granted = self
            .grants
            .iter()
            .any(|(p, t)| p == principal && t == tool);
        if granted {
            Ok(())
        } else {
            Err(format!(
                "principal {} has no ToolDispatch grant for {tool}",
                principal_label_for(principal),
            ))
        }
    }
}

// ----------------------------------------------------------------
// Helpers
// ----------------------------------------------------------------

/// Pick the first auto-approved action (`requires_approval = false`)
/// from a Genesis-produced manifest. This is the action the test
/// uses as the MCP tool name — it's the only kind a worker can
/// dispatch without an operator-in-the-loop approval, which is
/// what an autonomous runtime path exercises.
fn first_auto_approved_action(manifest: &VerticalManifest) -> (String, ActionRiskLevel) {
    let actions = manifest
        .actions
        .as_ref()
        .expect("Genesis-produced vertical must carry an [actions] section");
    actions
        .actions
        .iter()
        .find_map(|(name, decl)| {
            if !decl.requires_approval {
                Some((name.clone(), decl.risk))
            } else {
                None
            }
        })
        .expect("Genesis seed-vocabulary must include at least one auto-approved action")
}

/// Build a 1-task workflow whose worker = the generated vertical_id
/// (the MCP pool key) and whose `inputs.tool` = the action name.
/// The args carry the action's risk level so the dispatcher can
/// assert it was wired through.
fn single_action_workflow(vertical_id: &str, action: &str, risk: ActionRiskLevel) -> Workflow {
    let mut inputs = BTreeMap::new();
    inputs.insert("tool".into(), serde_json::Value::String(action.to_string()));
    inputs.insert(
        "args".into(),
        serde_json::json!({
            "action_type": action,
            "risk": format!("{risk:?}").to_lowercase(),
        }),
    );
    Workflow {
        id: format!("claim3-{vertical_id}-{action}"),
        description: format!("claim-3 LIVE: run {action} against {vertical_id}"),
        tasks: vec![TaskNode {
            id: "step1".into(),
            worker: vertical_id.to_string(),
            persona: None,
            inputs,
            outputs: BTreeMap::new(),
            depends_on: vec![],
            cache_key: None,
        }],
        edges: vec![],
    }
}

// ----------------------------------------------------------------
// Tests
// ----------------------------------------------------------------

/// Claim-3 LIVE happy path: drive Genesis on a plain-English
/// charter, persist the vertical, build a 1-task workflow
/// mirroring one of the vertical's auto-approved actions, and run
/// it through `WorkerSessionRunner` against a stub MCP dispatcher.
///
/// Asserts the four contracts named in the module docstring.
#[tokio::test]
async fn genesis_plan_writes_vertical_then_workflow_runs_against_mock_mcp() {
    // 1. Genesis: plain-English → GenesisPlan → vertical tree.
    let p = plan(CHARTER, PERSONA).expect("Genesis pipeline plans cleanly");
    let dir = tempfile::tempdir().expect("tempdir");
    let out = dir.path().join(&p.vertical_id);
    write_vertical(&p, &out).expect("write_vertical succeeds");

    // Round-trip-load via the production consumer so we know the
    // generated tree is well-formed before we run a workflow
    // against it.
    let (registry, errors) =
        argo_vertical::load_dir(dir.path()).expect("load_dir returns a registry");
    assert!(
        errors.is_empty(),
        "Genesis output must round-trip through argo_vertical::load_dir without errors; got {errors:?}",
    );
    let loaded = registry
        .get(&p.vertical_id)
        .expect("generated vertical present in registry");

    // 2. Pick an auto-approved action to dispatch.
    let (action_name, action_risk) = first_auto_approved_action(&loaded.manifest);

    // 3. Build the runner. `vertical_id` IS the MCP server / pool
    //    key (production wires the same way — `worker` is the
    //    MCP server name in `argo-workflow`'s task shape).
    let principal = Principal::Local {
        os_user: "operator-alpha".into(),
    };
    let tenant_id = TenantId::new();

    let canned_response = serde_json::json!({
        "ok": true,
        "tool": action_name,
        "echoed_persona": p.persona,
    });
    let dispatcher = StubDispatcher::ok(canned_response.clone());
    let audit = StubAudit::new();
    let capabilities = StubCapabilities::granting(principal.clone(), &action_name);

    let runner = WorkerSessionRunner::new(
        dispatcher.clone(),
        audit.clone(),
        capabilities,
        principal.clone(),
        tenant_id,
    );
    let executor = Executor::new(runner);

    // 4. Execute. Asserts on the resulting verdict + dispatcher
    //    record + audit row.
    let workflow = single_action_workflow(&p.vertical_id, &action_name, action_risk);
    let report = executor
        .execute(ExecutionRequest::new(workflow))
        .await
        .expect("workflow executes");

    assert!(
        report.ok,
        "claim-3 LIVE: a Genesis-produced vertical's auto-approved action must run through \
         WorkerSessionRunner without surfacing a failed verdict; got report.ok=false, \
         verdicts={:?}",
        report.task_verdicts,
    );
    match report.task_verdicts.get("step1").expect("verdict present") {
        TaskVerdict::Success { outputs } => {
            assert_eq!(
                outputs.get("response"),
                Some(&canned_response),
                "the dispatcher's response must surface under outputs.response so downstream \
                 ${{tasks.step1.out.response}} references resolve",
            );
        }
        other => panic!("expected Success verdict, got {other:?}"),
    }

    // Dispatcher saw exactly one call, keyed by tenant_id + the
    // vertical_id (server) + the action name (tool).
    let calls = dispatcher.calls();
    assert_eq!(calls.len(), 1, "exactly one MCP dispatch expected");
    assert_eq!(calls[0].tenant_id, tenant_id);
    assert_eq!(calls[0].server, p.vertical_id);
    assert_eq!(calls[0].tool, action_name);
    assert_eq!(
        calls[0].args["action_type"],
        serde_json::Value::String(action_name.clone()),
    );

    // Audit: exactly one workflow.step.executed row carrying the
    // dispatcher's response.
    let events = audit.events();
    assert_eq!(events.len(), 1, "exactly one audit event expected");
    assert_eq!(events[0].kind, "workflow.step.executed");
    assert_eq!(events[0].task_id, "step1");
    assert_eq!(events[0].tool, action_name);
    assert_eq!(events[0].tenant_id, tenant_id);
    assert_eq!(events[0].principal_label, "local:operator-alpha");
    assert_eq!(events[0].response_or_reason, canned_response);
}

/// Cross-tenant negative pinning mission claim 6 ("Multi-tenant by
/// construction") through claim 3's seam — a foreign-tenant
/// principal must not be able to dispatch a workflow built against
/// another tenant's Genesis-produced vertical. The dispatcher is
/// never reached.
#[tokio::test]
async fn foreign_tenant_principal_is_refused_before_mcp_dispatch() {
    let p = plan(CHARTER, PERSONA).expect("Genesis plans");
    let dir = tempfile::tempdir().expect("tempdir");
    let out = dir.path().join(&p.vertical_id);
    write_vertical(&p, &out).expect("write_vertical");
    let (registry, errors) =
        argo_vertical::load_dir(dir.path()).expect("load_dir returns a registry");
    assert!(errors.is_empty(), "round-trip clean; got {errors:?}");
    let loaded = registry
        .get(&p.vertical_id)
        .expect("vertical present");
    let (action_name, action_risk) = first_auto_approved_action(&loaded.manifest);

    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    assert_ne!(tenant_a, tenant_b);

    let principal_a = Principal::Local {
        os_user: "alice-tenant-a".into(),
    };
    let principal_b = Principal::Local {
        os_user: "bob-tenant-b".into(),
    };

    // Grant tenant_a's principal access to the action. Bind the
    // runner to tenant_b's principal — the checker has no binding
    // and must refuse.
    let capabilities = StubCapabilities::granting(principal_a, &action_name);
    let dispatcher = StubDispatcher::ok(serde_json::json!({"never": "called"}));
    let audit = StubAudit::new();

    let runner = WorkerSessionRunner::new(
        dispatcher.clone(),
        audit.clone(),
        capabilities,
        principal_b,
        tenant_b,
    );
    let executor = Executor::new(runner);
    let workflow = single_action_workflow(&p.vertical_id, &action_name, action_risk);
    let report = executor
        .execute(ExecutionRequest::new(workflow))
        .await
        .expect("workflow executes");

    assert!(
        dispatcher.calls().is_empty(),
        "foreign-tenant principal must be refused BEFORE MCP dispatch; got calls={:?}",
        dispatcher.calls(),
    );
    match report.task_verdicts.get("step1").expect("verdict") {
        TaskVerdict::Failed { reason } => {
            assert!(
                reason.starts_with("capability denied"),
                "expected capability-denied reason, got {reason:?}",
            );
        }
        other => panic!("expected Failed verdict, got {other:?}"),
    }
    assert!(!report.ok);

    let events = audit.events();
    assert_eq!(events.len(), 1, "exactly one step-failed audit row");
    assert_eq!(events[0].kind, "workflow.step.failed");
    assert_eq!(events[0].tenant_id, tenant_b);
    let reason_str = events[0]
        .response_or_reason
        .as_str()
        .expect("reason is a string");
    assert!(
        reason_str.contains("capability denied"),
        "got {reason_str:?}",
    );
}
