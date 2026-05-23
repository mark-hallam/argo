//! Production [`TaskRunner`] that gates MCP tool dispatch on a
//! capability check + emits step-level audit rows
//! (`wire-worker-session-runner`).
//!
//! Three load-bearing contracts pinned by this module:
//!
//! 1. **Capability check fires BEFORE MCP dispatch.** A principal that
//!    the bound [`WorkflowCapabilityChecker`] refuses never reaches
//!    the MCP wire — the runner returns [`TaskOutcome::Failed`] with a
//!    `capability denied` reason and emits a step-failed audit row.
//! 2. **Audit row is emitted regardless of outcome.** Success ⇒ one
//!    `workflow.step.executed` row; failure (capability denial OR MCP
//!    error) ⇒ one `workflow.step.failed` row. Forensic readers can
//!    pin every step the runner walked.
//! 3. **Tenant identity is bound at construction.** A foreign-tenant
//!    principal is refused by the bound capability checker because
//!    the checker is itself tenant-scoped — callers build one per
//!    `(tenant_id, principal)` request.
//!
//! ## Crate-dependency carve-out
//!
//! The brief proposed `argo-workflow → argo-audit / argo-capability /
//! argo-mcp` Cargo dependencies. Those edges form a cycle on
//! `origin/main`:
//!
//! ```text
//! argo-workflow → argo-audit → argo-store → argo-runtime → argo-workflow
//! ```
//!
//! Adding `argo-audit` as a dep breaks `cargo check` workspace-wide
//! with E0466 (`cyclic package dependency`). To preserve the runner
//! living in `argo-workflow` (per the brief) without splitting the
//! crate graph, this module defines three thin trait seams
//! ([`ToolDispatcher`], [`WorkflowAuditEmitter`],
//! [`WorkflowCapabilityChecker`]) that production callers — sitting
//! one level up at `argo-cli` / `argo-runtime` — implement around
//! `argo_audit::AuditSink`, `argo_capability::CapabilityEngine`, and
//! `argo_mcp::McpClientPool`. The contract tests inject stub impls.
//!
//! ## Task shape
//!
//! The DAG executor's [`TaskNode`] is intentionally generic.
//! `WorkerSessionRunner` reads the following from each task:
//!
//! - `task.worker` — MCP server / manifest id (pool key)
//! - `task.inputs["tool"]` — string, the MCP tool name to invoke
//! - `task.inputs["args"]` — JSON, the tool arguments (defaults to
//!   `{}` when absent)
use crate::dag::TaskNode;
use crate::executor::{TaskOutcome, TaskRunner};
use argo_types::{Principal, TenantId};
use async_trait::async_trait;
use std::collections::BTreeMap;
use std::sync::Arc;
use thiserror::Error;

/// Errors a [`ToolDispatcher`] surfaces. Used by the runner to
/// distinguish a malformed-step contract bug from an MCP-transport
/// error — both become `Failed` verdicts, but with distinguishable
/// `reason` strings.
#[derive(Debug, Error)]
pub enum ToolDispatchError {
    /// The dispatcher could not reach the named server. Production
    /// impls map MCP transport errors here.
    #[error("dispatch transport: {0}")]
    Transport(String),
    /// The tool ran but reported `isError = true` (application-level
    /// failure). Payload carries the upstream message.
    #[error("dispatch tool-error: {0}")]
    ToolError(String),
    /// The dispatcher rejected the request as malformed (e.g. unknown
    /// server). Distinct from the runner's own pre-dispatch shape
    /// validation, which produces an inline reason without consulting
    /// the dispatcher.
    #[error("dispatch bad-request: {0}")]
    BadRequest(String),
}

/// Abstract MCP-dispatch boundary the runner depends on. Production
/// wraps `argo_mcp::McpClientPool`; tests inject a canned-response
/// stub so the contract tests do not need a real MCP subprocess.
#[async_trait]
pub trait ToolDispatcher: Send + Sync {
    /// Invoke `tool` on `server` with `args`, scoped to `tenant_id`
    /// (the pool key). Returns the tool's response as a JSON value
    /// suitable for `TaskOutcome::Success.outputs`.
    async fn call(
        &self,
        tenant_id: &TenantId,
        server: &str,
        tool: &str,
        args: serde_json::Value,
    ) -> Result<serde_json::Value, ToolDispatchError>;
}

/// Per-step audit context handed to a [`WorkflowAuditEmitter`]. Owned
/// by short-lived `&` references so production impls can adapt to
/// whatever `AuditSink::write` shape they want without forcing
/// allocations on the runner side.
pub struct StepAuditContext<'a> {
    /// Tenant the runner was bound to.
    pub tenant_id: &'a TenantId,
    /// Workflow task id the row pertains to.
    pub task_id: &'a str,
    /// `TaskNode::worker` — the MCP server / manifest id.
    pub worker: &'a str,
    /// MCP tool that was invoked (or `<unknown>` for malformed
    /// shape failures).
    pub tool: &'a str,
    /// Principal the runner was bound to.
    pub principal: &'a Principal,
}

/// Abstract audit-emission boundary. Production wraps
/// `argo_audit::AuditSink` with the `WorkflowStepExecuted` /
/// `WorkflowStepFailed` audit kinds (added in `argo-audit` by this
/// brief). The success arm returns `Err(reason)` so the runner can
/// mark the step as failed if the audit row could not land — silently
/// swallowing audit-write errors would defeat the forensic
/// invariant.
#[async_trait]
pub trait WorkflowAuditEmitter: Send + Sync {
    /// Emit a step-executed row. Returns the audit error message if
    /// the underlying sink rejected the write.
    async fn step_executed(
        &self,
        ctx: &StepAuditContext<'_>,
        response: &serde_json::Value,
    ) -> Result<(), String>;

    /// Emit a step-failed row. Best-effort — sink errors here are
    /// dropped so the runner still surfaces the original failure as
    /// the verdict reason.
    async fn step_failed(&self, ctx: &StepAuditContext<'_>, reason: &str);
}

/// Abstract capability-check boundary. Production wraps
/// `argo_capability::CapabilityEngine::check` with a
/// [`Capability::ToolDispatch`][argo_types::Capability::ToolDispatch]
/// for the requested tool. The trait returns `Err(reason)` rather
/// than the engine's typed `ArgoError` so `argo-workflow` does not
/// need to alias `argo-audit`'s error tree.
pub trait WorkflowCapabilityChecker: Send + Sync {
    /// Returns `Err(reason)` if `principal` lacks `ToolDispatch` for
    /// `tool`. The runner converts the error into the
    /// `capability denied: <reason>` step-failed verdict.
    fn check_dispatch(&self, principal: &Principal, tool: &str) -> Result<(), String>;
}

/// Production [`TaskRunner`] for the DAG executor. See module docs.
pub struct WorkerSessionRunner {
    dispatcher: Arc<dyn ToolDispatcher>,
    audit: Arc<dyn WorkflowAuditEmitter>,
    capabilities: Arc<dyn WorkflowCapabilityChecker>,
    principal: Principal,
    tenant_id: TenantId,
}

impl WorkerSessionRunner {
    /// Bind a runner to a `(principal, tenant_id)` pair. The
    /// dispatcher, audit emitter, and capability checker are
    /// expected to already be tenant-scoped per their respective
    /// constructors (e.g. `AuditSink::new_with_tenant`).
    #[must_use]
    pub fn new(
        dispatcher: Arc<dyn ToolDispatcher>,
        audit: Arc<dyn WorkflowAuditEmitter>,
        capabilities: Arc<dyn WorkflowCapabilityChecker>,
        principal: Principal,
        tenant_id: TenantId,
    ) -> Self {
        Self {
            dispatcher,
            audit,
            capabilities,
            principal,
            tenant_id,
        }
    }

    /// Borrow the principal the runner was bound to. Mostly used by
    /// tests asserting the wiring.
    #[must_use]
    pub fn principal(&self) -> &Principal {
        &self.principal
    }

    /// Borrow the tenant id the runner was bound to.
    #[must_use]
    pub fn tenant_id(&self) -> &TenantId {
        &self.tenant_id
    }

    fn parse_step(task: &TaskNode) -> Result<StepSpec<'_>, String> {
        if task.worker.is_empty() {
            return Err("task.worker (MCP server / manifest id) is required".to_string());
        }
        let server = task.worker.as_str();
        let tool = match task.inputs.get("tool") {
            Some(serde_json::Value::String(s)) if !s.is_empty() => s.as_str(),
            Some(_) => return Err("task.inputs[\"tool\"] must be a non-empty string".to_string()),
            None => {
                return Err(
                    "task.inputs[\"tool\"] is required (the MCP tool name)".to_string(),
                )
            }
        };
        let args = task
            .inputs
            .get("args")
            .cloned()
            .unwrap_or_else(|| serde_json::json!({}));
        Ok(StepSpec { server, tool, args })
    }
}

#[derive(Debug)]
struct StepSpec<'a> {
    server: &'a str,
    tool: &'a str,
    args: serde_json::Value,
}

#[async_trait]
impl TaskRunner for WorkerSessionRunner {
    async fn run(
        &self,
        task: &TaskNode,
        _inputs: &BTreeMap<String, serde_json::Value>,
    ) -> TaskOutcome {
        // Step shape — bail before any auth/MCP work if malformed.
        let step = match Self::parse_step(task) {
            Ok(s) => s,
            Err(reason) => {
                let ctx = StepAuditContext {
                    tenant_id: &self.tenant_id,
                    task_id: &task.id,
                    worker: &task.worker,
                    tool: "<unknown>",
                    principal: &self.principal,
                };
                self.audit.step_failed(&ctx, &reason).await;
                return TaskOutcome::Failed { reason };
            }
        };

        let ctx = StepAuditContext {
            tenant_id: &self.tenant_id,
            task_id: &task.id,
            worker: &task.worker,
            tool: step.tool,
            principal: &self.principal,
        };

        // 1. Capability check fires BEFORE MCP dispatch (RBAC). A
        //    foreign-tenant principal is refused here because the
        //    bound checker has no binding for it.
        if let Err(err) = self.capabilities.check_dispatch(&self.principal, step.tool) {
            let reason = format!("capability denied: {err}");
            self.audit.step_failed(&ctx, &reason).await;
            return TaskOutcome::Failed { reason };
        }

        // 2. MCP dispatch, keyed by (tenant_id, server).
        let response = match self
            .dispatcher
            .call(&self.tenant_id, step.server, step.tool, step.args)
            .await
        {
            Ok(v) => v,
            Err(err) => {
                let reason = err.to_string();
                self.audit.step_failed(&ctx, &reason).await;
                return TaskOutcome::Failed { reason };
            }
        };

        // 3. Audit row INSIDE the step's logical transaction — the
        //    step is successful only if the audit write succeeds.
        //    Otherwise surface a Failed verdict (and a second
        //    step_failed row) so the run cannot complete a step that
        //    left no forensic trace.
        if let Err(reason) = self.audit.step_executed(&ctx, &response).await {
            self.audit.step_failed(&ctx, &reason).await;
            return TaskOutcome::Failed { reason };
        }

        // 4. Outputs: the runner-visible response map. Downstream
        //    `${tasks.<id>.out.response}` references resolve against
        //    the upstream tool reply.
        let mut outputs = BTreeMap::new();
        outputs.insert("response".to_string(), response);
        TaskOutcome::Success { outputs }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_step_requires_worker() {
        let mut task = TaskNode {
            id: "t".into(),
            worker: String::new(),
            persona: None,
            inputs: BTreeMap::new(),
            outputs: BTreeMap::new(),
            depends_on: vec![],
            cache_key: None,
        };
        task.inputs
            .insert("tool".into(), serde_json::Value::String("echo".into()));
        let err = WorkerSessionRunner::parse_step(&task).unwrap_err();
        assert!(err.contains("task.worker"), "got {err}");
    }

    #[test]
    fn parse_step_requires_tool_string_input() {
        let task = TaskNode {
            id: "t".into(),
            worker: "srv".into(),
            persona: None,
            inputs: BTreeMap::new(),
            outputs: BTreeMap::new(),
            depends_on: vec![],
            cache_key: None,
        };
        let err = WorkerSessionRunner::parse_step(&task).unwrap_err();
        assert!(err.contains("task.inputs[\"tool\"]"), "got {err}");
    }

    #[test]
    fn parse_step_args_defaults_to_empty_object() {
        let mut task = TaskNode {
            id: "t".into(),
            worker: "srv".into(),
            persona: None,
            inputs: BTreeMap::new(),
            outputs: BTreeMap::new(),
            depends_on: vec![],
            cache_key: None,
        };
        task.inputs
            .insert("tool".into(), serde_json::Value::String("echo".into()));
        let step = WorkerSessionRunner::parse_step(&task).unwrap();
        assert_eq!(step.server, "srv");
        assert_eq!(step.tool, "echo");
        assert_eq!(step.args, serde_json::json!({}));
    }
}
