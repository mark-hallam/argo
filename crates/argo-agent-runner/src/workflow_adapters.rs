//! Production adapters that bridge the abstract [`argo_workflow`] runner
//! seams to the real `argo-mcp` / `argo-audit` / `argo-capability`
//! subsystems, so the DAG executor can run a workflow with a real
//! [`argo_workflow::WorkerSessionRunner`] instead of the test-only
//! `MockRunner`.
//!
//! ## Why here (crate placement)
//!
//! `argo-workflow` deliberately depends on **only** `argo-types` — adding
//! `argo-audit` / `argo-capability` there would form the cycle
//! `argo-workflow → argo-audit → argo-store → argo-runtime → argo-workflow`.
//! So the runner exposes three trait seams ([`ToolDispatcher`],
//! [`WorkflowAuditEmitter`], [`WorkflowCapabilityChecker`]) and the
//! production impls live one level up. `argo-agent-runner` already depends
//! on all three target crates **and** sits outside the store↔runtime
//! cycle, so it is the correct home (verified 2026-05-31:
//! `argo-runtime` deps only `argo-types`+`argo-workflow`, so the adapters
//! could *not* live there).
//!
//! ## The three adapters
//!
//! - [`McpToolDispatcher`] — resolves a workflow step's `server` name to a
//!   stdio transport, acquires a pooled [`McpClient`] keyed on
//!   `(tenant_id, server)`, and invokes the tool. Mirrors the live
//!   agent-turn dispatch path (`dispatch.rs`).
//! - [`AuditWorkflowEmitter`] — writes one
//!   [`AuditKind::WorkflowStepExecuted`] row per successful step and one
//!   [`AuditKind::WorkflowStepFailed`] row per failure, through a
//!   tenant-bound [`AuditSink`].
//! - [`CapabilityWorkflowChecker`] — gates each step's tool on a
//!   [`Capability::ToolDispatch`] check against a bound
//!   [`CapabilityEngine`], so a principal the engine refuses never reaches
//!   the MCP wire.

use argo_audit::{AuditKind, AuditSink};
use argo_capability::CapabilityEngine;
use argo_mcp::{McpClient, McpClientPool};
use argo_types::{Capability, Principal, TenantId};
use argo_workflow::{
    StepAuditContext, ToolDispatchError, ToolDispatcher, WorkflowAuditEmitter,
    WorkflowCapabilityChecker,
};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::sync::Arc;

/// Render a [`Principal`] to the stable `kind:identity` label used in
/// audit `principal_id` columns (mirrors the gateway's `principal_to_actor`).
fn principal_label(principal: &Principal) -> String {
    match principal {
        Principal::Gateway { adapter, identity } => format!("{adapter}:{identity}"),
        Principal::Local { os_user } => format!("local:{os_user}"),
        Principal::Acp { harness, identity } => format!("{harness}:{identity}"),
        Principal::Forge => "forge".to_string(),
    }
}

/// Parse a `{"transport":"stdio","command":"…","args":[…]}` value into
/// `(command, args)`. Self-contained (does not reach into `dispatch.rs`)
/// so this module never collides with the live agent-turn dispatch path.
fn parse_stdio_transport(transport: &Value) -> Result<(String, Vec<String>), String> {
    let kind = transport
        .get("transport")
        .and_then(Value::as_str)
        .unwrap_or("stdio");
    if kind != "stdio" {
        return Err(format!("unsupported MCP transport '{kind}'"));
    }
    let command = transport
        .get("command")
        .and_then(Value::as_str)
        .ok_or_else(|| "MCP stdio transport missing command".to_string())?
        .to_string();
    let args = transport
        .get("args")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    Ok((command, args))
}

/// Production [`ToolDispatcher`] over a per-tenant [`McpClientPool`].
///
/// Holds the `server name → transport` map for the active workflow's
/// vertical so a step naming `server` resolves to a concrete stdio
/// command. Connection reuse + per-tenant isolation come from the pool.
///
/// # Examples
///
/// ```
/// use argo_agent_runner::workflow_adapters::McpToolDispatcher;
/// use argo_mcp::{McpClientPool, PoolConfig};
/// use std::collections::BTreeMap;
///
/// let pool = McpClientPool::new(PoolConfig::default());
/// let mut servers = BTreeMap::new();
/// servers.insert(
///     "notes".to_string(),
///     serde_json::json!({ "transport": "stdio", "command": "notes-mcp", "args": [] }),
/// );
/// let _dispatcher = McpToolDispatcher::new(pool, servers);
/// ```
pub struct McpToolDispatcher {
    pool: Arc<McpClientPool>,
    servers: BTreeMap<String, Value>,
}

impl McpToolDispatcher {
    /// Build a dispatcher over `pool`, resolving server names through
    /// `servers` (each value a stdio transport descriptor).
    #[must_use]
    pub fn new(pool: Arc<McpClientPool>, servers: BTreeMap<String, Value>) -> Self {
        Self { pool, servers }
    }
}

#[async_trait]
impl ToolDispatcher for McpToolDispatcher {
    async fn call(
        &self,
        tenant_id: &TenantId,
        server: &str,
        tool: &str,
        args: Value,
    ) -> Result<Value, ToolDispatchError> {
        let transport = self.servers.get(server).ok_or_else(|| {
            ToolDispatchError::BadRequest(format!("unknown MCP server '{server}'"))
        })?;
        let (command, cmd_args) =
            parse_stdio_transport(transport).map_err(ToolDispatchError::BadRequest)?;
        let guard = self
            .pool
            .acquire(*tenant_id, server.to_string(), || async move {
                McpClient::connect_stdio(&command, &cmd_args).await
            })
            .await
            .map_err(|e| ToolDispatchError::Transport(format!("connect '{server}': {e}")))?;
        let result = guard
            .client_arc()
            .call_tool(tool, args)
            .await
            .map_err(|e| ToolDispatchError::Transport(e.to_string()))?;
        if result.is_error {
            let msg = serde_json::to_string(&result.content)
                .unwrap_or_else(|_| "MCP tool reported isError".to_string());
            return Err(ToolDispatchError::ToolError(msg));
        }
        serde_json::to_value(&result)
            .map_err(|e| ToolDispatchError::Transport(format!("serialize tool result: {e}")))
    }
}

/// Assemble the JSON payload written for a workflow step audit row.
fn step_payload(ctx: &StepAuditContext<'_>, key: &str, value: Value) -> Value {
    json!({
        "tenant_id": ctx.tenant_id.to_string(),
        "task_id": ctx.task_id,
        "worker": ctx.worker,
        "tool": ctx.tool,
        key: value,
    })
}

/// Production [`WorkflowAuditEmitter`] over a tenant-bound [`AuditSink`].
///
/// Emits [`AuditKind::WorkflowStepExecuted`] on success (carrying the
/// tool response) and [`AuditKind::WorkflowStepFailed`] on failure
/// (carrying the reason). The success arm propagates a write error so the
/// runner can fail a step that left no forensic trace; the failure arm is
/// best-effort.
///
/// # Examples
///
/// ```no_run
/// use argo_agent_runner::workflow_adapters::AuditWorkflowEmitter;
/// use argo_audit::AuditSink;
/// # fn demo(sink: AuditSink) {
/// let _emitter = AuditWorkflowEmitter::new(sink);
/// # }
/// ```
pub struct AuditWorkflowEmitter {
    sink: AuditSink,
}

impl AuditWorkflowEmitter {
    /// Wrap an already tenant-bound [`AuditSink`]
    /// (e.g. [`AuditSink::new_with_tenant`]).
    #[must_use]
    pub fn new(sink: AuditSink) -> Self {
        Self { sink }
    }
}

#[async_trait]
impl WorkflowAuditEmitter for AuditWorkflowEmitter {
    async fn step_executed(
        &self,
        ctx: &StepAuditContext<'_>,
        response: &Value,
    ) -> Result<(), String> {
        let payload = step_payload(ctx, "response", response.clone());
        self.sink
            .write(
                None,
                AuditKind::WorkflowStepExecuted,
                payload,
                Some(principal_label(ctx.principal)),
            )
            .await
            .map_err(|e| e.to_string())
    }

    async fn step_failed(&self, ctx: &StepAuditContext<'_>, reason: &str) {
        let payload = step_payload(ctx, "reason", Value::String(reason.to_string()));
        // Best-effort: a failed step must still surface its original
        // reason even if the forensic row cannot be written.
        let _ = self
            .sink
            .write(
                None,
                AuditKind::WorkflowStepFailed,
                payload,
                Some(principal_label(ctx.principal)),
            )
            .await;
    }
}

/// Production [`WorkflowCapabilityChecker`] over a bound [`CapabilityEngine`].
///
/// The engine is expected to already have the request's principal bound to
/// a role (per `(tenant_id, principal)` request, as the runner contract
/// requires). `check_dispatch` models the requested tool as a
/// [`Capability::ToolDispatch`] glob and defers to
/// [`CapabilityEngine::check`].
///
/// # Examples
///
/// ```
/// use argo_agent_runner::workflow_adapters::CapabilityWorkflowChecker;
/// use argo_capability::CapabilityEngine;
/// use std::sync::Arc;
///
/// let engine = Arc::new(CapabilityEngine::with_defaults());
/// let _checker = CapabilityWorkflowChecker::new(engine);
/// ```
pub struct CapabilityWorkflowChecker {
    engine: Arc<CapabilityEngine>,
}

impl CapabilityWorkflowChecker {
    /// Wrap a [`CapabilityEngine`] whose principal binding is already set.
    #[must_use]
    pub fn new(engine: Arc<CapabilityEngine>) -> Self {
        Self { engine }
    }
}

impl WorkflowCapabilityChecker for CapabilityWorkflowChecker {
    fn check_dispatch(&self, principal: &Principal, tool: &str) -> Result<(), String> {
        let cap = Capability::ToolDispatch {
            tool_name_glob: tool.to_string(),
        };
        self.engine.check(principal, &cap).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
#[path = "workflow_adapters_tests.rs"]
mod workflow_adapters_tests;
