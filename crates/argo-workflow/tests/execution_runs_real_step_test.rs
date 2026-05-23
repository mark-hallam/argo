//! Integration test for `argo-workflow` execution layer (brief
//! `eval-workflow-execution-runs-real-step`).
//!
//! Round-8 mission-vs-code-gap manifest, `argo-workflow` row:
//!
//! > "argo-workflow (runtime) — DAG / executor / checkpoint primitives.
//! > The workflow runner. Mission claim 3 needs the SPA build canvas →
//! > save → publish → execute loop end-to-end. Round 7 confirmed CRUD
//! > layer is solid; execution layer is unaudited."
//!
//! The brief asks: given a saved workflow with a single `tool_call`
//! step against a mock MCP server, does the executor (1) invoke the
//! tool, (2) write an audit_record row, (3) write a workflow_run
//! checkpoint, plus a cross-tenant negative and a failure-injection?
//!
//! # Pre-flight finding: the executor seam is abstract; the wire is broken.
//!
//! **Per the brief's "If pre-flight reveals … the wire is broken →
//! ship `#[ignore]`'d + FINDINGS-ONLY" branch.** The executor surface
//! exists and has a clean test seam (`TaskRunner` trait), but no
//! production `TaskRunner` impl exists anywhere in the workspace —
//! every caller binds the in-tree `MockRunner::echo()`. The
//! executor crate has zero references to `tenant_id`, `audit_record`,
//! or anything in `argo-mcp`. The three load-bearing contracts the
//! brief wants pinned cannot be exercised against current
//! `origin/main`.
//!
//! ## Surface map (read at `origin/main` `6fb89c6e3`)
//!
//! Present in `crates/argo-workflow/src/`:
//! - `Executor<R: TaskRunner>::execute(ExecutionRequest) ->
//!   ExecutionReport` — topological DAG walker.
//! - `trait TaskRunner { async fn run(&self, &TaskNode, &inputs) ->
//!   TaskOutcome; }` — the only injection seam.
//! - `MockRunner::echo() / .fail(id)` — only `TaskRunner` impl in the
//!   entire workspace (verified via `rg 'impl TaskRunner' crates/`).
//! - `trait CheckpointStore` + `InMemoryCheckpointStore` —
//!   optional checkpoint backend, opt-in via
//!   `Executor::execute_with_checkpoints`.
//!
//! Present elsewhere but **not wired through the executor**:
//! - `workflow_run` SQL table (migration 18) — written by
//!   `argo-cli/src/handlers/workflow.rs::run_workflow` after the
//!   executor returns, NOT by the executor itself. The CLI handler
//!   binds `Executor::new(MockRunner::echo())` and notes in its
//!   header comment: *"v1 wiring uses the `MockRunner` so workflows
//!   are end-to-end testable without the production worker dispatch
//!   path; a follow-up brief swaps in a `WorkerSessionRunner`."*
//!   That follow-up brief has not landed.
//! - `workflow_test_run` SQL table (migration 130 / v78, PR #2056) —
//!   server-side history for the builder's "Test run" overlay,
//!   written by an NDJSON gateway handler, not by the executor.
//! - `audit_record` SQL table — `argo-audit`'s sink. No call site
//!   in `argo-workflow` (verified via `rg 'audit_record\|AuditSink'
//!   crates/argo-workflow/`).
//! - `argo-mcp::McpClient` — no reference from `argo-workflow`
//!   (verified via `rg 'argo[_-]mcp\|McpClient' crates/argo-workflow/`).
//!
//! ## What's missing for the brief's contracts to pin
//!
//! 1. **No tenant identity on the executor surface.** The brief asks
//!    for "try to run tenant_a's workflow with tenant_b's principal →
//!    assert refusal at executor pre-flight". `ExecutionRequest` has
//!    no `tenant_id` / `Principal` field, and `Executor::execute` has
//!    no pre-flight predicate to fail. A new field on
//!    `ExecutionRequest` plus an `AuthorizationCheck` trait would be
//!    needed.
//! 2. **No MCP / tool-call wire.** The brief asks the mock MCP server
//!    to "receive the tool call". `TaskRunner::run` takes a
//!    `TaskNode` whose `worker` field is a string ID — the trait does
//!    not invoke MCP, audit, or any external service. A
//!    `WorkerSessionRunner: TaskRunner` (the named follow-up that
//!    hasn't landed) would need to call into `argo-mcp`.
//! 3. **No audit emission from the executor.** `Executor::execute`
//!    returns a `BTreeMap<task_id, TaskVerdict>` but writes nothing
//!    to `audit_record`. The verdicts live only in the returned
//!    report and the optional checkpoint store.
//!
//! ## Smoke that DOES pass today
//!
//! The one non-ignored test below confirms that the surface the
//! executor _does_ export — DAG walking + verdict emission via the
//! abstract `TaskRunner` seam — still works as documented. It is a
//! surface-existence guard against future refactors hollowing out
//! the executor's public API.

use argo_workflow::{
    ExecutionReport, ExecutionRequest, Executor, MockRunner, TaskNode, TaskOutcome, TaskRunner,
    TaskVerdict, Workflow,
};
use async_trait::async_trait;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

/// Helper: a 1-task workflow asking worker `mcp-tool-caller` to run
/// a single `tool_call`-shaped step. Matches the brief's shape:
/// "minimal workflow with ONE step (a `tool_call` against a mock MCP
/// server)".
fn single_step_tool_call_workflow() -> Workflow {
    let mut inputs = BTreeMap::new();
    inputs.insert(
        "tool".into(),
        serde_json::Value::String("echo".into()),
    );
    inputs.insert(
        "payload".into(),
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

/// Surface-existence smoke. NOT ignored. Pins that the executor's
/// abstract DAG-walker + `TaskRunner`-seam wire-up still works end
/// to end, so a future refactor that hollows out the seam fails
/// loudly here even before the WorkerSessionRunner follow-up lands.
#[tokio::test]
async fn executor_surface_walks_a_single_task_and_emits_a_verdict() {
    let workflow = single_step_tool_call_workflow();
    let exec = Executor::new(MockRunner::echo());
    let report: ExecutionReport = exec
        .execute(ExecutionRequest::new(workflow))
        .await
        .expect("executor must accept a validated 1-task workflow");
    assert!(report.ok, "single-task workflow must complete ok: {report:?}");
    assert_eq!(
        report.task_verdicts.len(),
        1,
        "exactly one task verdict expected"
    );
    let verdict = report
        .task_verdicts
        .get("call")
        .expect("verdict for 'call' must be present");
    match verdict {
        TaskVerdict::Success { outputs } => {
            // MockRunner echoes inputs into outputs verbatim plus the
            // synthetic `echo` key. Pinning both confirms the input-
            // resolution + verdict-emission seam works.
            assert_eq!(
                outputs.get("tool"),
                Some(&serde_json::Value::String("echo".into())),
                "MockRunner must echo inputs into outputs"
            );
            assert_eq!(
                outputs.get("echo"),
                Some(&serde_json::Value::String("call".into())),
                "MockRunner must emit synthetic `echo` keyed by task id"
            );
        }
        other => panic!("expected Success verdict, got {other:?}"),
    }
}

/// Shared per-invocation record. Each tuple is `(task_id, resolved
/// inputs handed to the runner)`.
type CallLog = Arc<Mutex<Vec<(String, BTreeMap<String, serde_json::Value>)>>>;

/// Recording runner: captures every `TaskRunner::run` invocation so
/// tests can assert what the executor *would* hand to a real runner.
/// Stand-in for the `WorkerSessionRunner`/MCP-client seam the
/// follow-up brief will introduce. The log is held in an `Arc` so
/// the test can keep a handle alongside the executor (the executor
/// takes ownership of its runner via `pub(crate)` field).
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

/// Lightweight pin: even before MCP is wired through, the executor
/// must hand each task's resolved inputs to the runner. This is the
/// contract a future `WorkerSessionRunner` will rely on to forward
/// the payload to MCP. NOT ignored — exercises real seam behavior.
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
    assert_eq!(calls.len(), 1, "exactly one runner invocation expected");
    let (id, inputs) = &calls[0];
    assert_eq!(id, "call");
    assert_eq!(
        inputs.get("tool"),
        Some(&serde_json::Value::String("echo".into())),
        "executor must pass the YAML-declared `tool` input through verbatim"
    );
    assert_eq!(
        inputs.get("payload"),
        Some(&serde_json::json!({"msg": "hello-from-test"})),
        "executor must pass the YAML-declared `payload` through verbatim"
    );
}

// ============================================================
// FINDINGS-ONLY tests below. All `#[ignore]`'d. Each documents
// one of the brief's three load-bearing contracts and what
// would need to be wired before it can be un-ignored.
// ============================================================

/// **FINDINGS-ONLY.** Brief contract: "The mock MCP server received
/// the tool call. The executor produced an `audit_record` row with
/// the tool's actual response."
///
/// **Why ignored:** `argo-workflow` has zero references to
/// `argo-mcp` or `AuditSink`. The executor's only injection seam
/// (`TaskRunner`) doesn't know about MCP or audit. No production
/// `TaskRunner` impl exists in the workspace — `MockRunner` is the
/// only one. To un-ignore: land the `WorkerSessionRunner`
/// (referenced in the comment header of
/// `crates/argo-cli/src/handlers/workflow.rs`) so that the runner
/// invokes `McpClient::call_tool` and writes an `audit_record` row
/// with the tool's response. The brief's suggested path —
/// `mockito` + in-test JSON-RPC handler — depends on that wire-up
/// existing first.
#[tokio::test]
#[ignore = "FINDINGS-ONLY: no production TaskRunner impl wires MCP or audit (see module docs)"]
async fn happy_path_executor_invokes_mcp_and_writes_audit_row() {
    unreachable!(
        "Cannot run until `WorkerSessionRunner: TaskRunner` exists and \
         emits `audit_record` rows from `argo-workflow`. Re-check this \
         when the executor's `TaskRunner` seam grows an MCP-aware impl."
    );
}

/// **FINDINGS-ONLY.** Brief contract: "Cross-tenant negative: try to
/// run tenant_a's workflow with tenant_b's principal → assert refusal
/// at executor pre-flight."
///
/// **Why ignored:** `argo-workflow` has zero references to
/// `tenant_id` or `Principal`. `ExecutionRequest` carries only the
/// workflow + a generated run id. There is no executor pre-flight
/// predicate to refuse a cross-tenant principal. To un-ignore:
/// add a `principal: Principal` (or equivalent tenant-bearing
/// identity) to `ExecutionRequest` plus an `AuthorizationCheck` trait
/// that the executor calls before walking the DAG.
#[tokio::test]
#[ignore = "FINDINGS-ONLY: ExecutionRequest carries no tenant identity (see module docs)"]
async fn cross_tenant_executor_refuses_foreign_principal() {
    unreachable!(
        "Cannot run until `ExecutionRequest` (or a wrapping API) carries a \
         tenant-bearing principal and the executor consults it pre-flight."
    );
}

/// **FINDINGS-ONLY.** Brief contract: "Failure-injection: mock MCP
/// returns 500 → assert workflow_run row marks step FAILED (not
/// silently swallowed), audit row still written with the failure."
///
/// **Why ignored:** Same wire-broken root cause as the happy-path
/// test — there is no MCP-aware runner to inject a 500 into, and
/// the executor does not write `audit_record` rows. The `Executor`
/// *does* surface a `Failed { reason }` verdict via `TaskOutcome`,
/// and a downstream `workflow_run.task_verdicts_json` row in the
/// CLI handler captures it — so the in-process verdict-propagation
/// half of this contract is functional (covered by
/// `failed_task_skips_downstream` in `executor.rs`). What is missing
/// is the *audit* arm of the contract: a 500 from MCP must produce
/// an `audit_record` row with the failure, not be swallowed silently
/// at the runner boundary.
#[tokio::test]
#[ignore = "FINDINGS-ONLY: no MCP runner + no audit wire from executor (see module docs)"]
async fn failure_injection_mcp_500_marks_step_failed_and_writes_audit() {
    unreachable!(
        "Cannot run until `WorkerSessionRunner` exists and writes an \
         `audit_record` row on tool-call failure. The DAG-walker side of \
         this (Failed verdict + downstream skip) is already covered by \
         the executor's unit tests."
    );
}
