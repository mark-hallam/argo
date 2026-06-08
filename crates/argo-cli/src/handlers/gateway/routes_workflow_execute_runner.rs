//! Real-runner assembly for `routes_workflow_execute` — splits the
//! production [`WorkerSessionRunner`] wiring out of the parent route so
//! that file stays under the 500-LOC budget.
//!
//! NO_THEATER ([DEC-007]/[DEC-029]): a live `/workflows/.../execute` run
//! must dispatch real per-tenant MCP steps on the happy path, not echo.
//! [`build_hybrid_runner`] assembles a [`HybridWorkflowRunner`] that, per
//! step, routes a `tool_call`-shaped step naming a known MCP server
//! through the real [`WorkerSessionRunner`] (the three
//! `argo_agent_runner::workflow_adapters` adapters: MCP dispatch +
//! step-level audit rows + per-step capability check) and degrades every
//! other step (prompt / skill_invoke / unresolvable server) to the
//! deterministic [`MockRunner`] echo. The MockRunner is therefore a
//! **fallback only** — never a silent happy-path echo over a real tool.
//!
//! The DAG built by `vertical_workflow_to_dag` carries a step's MCP
//! server name in `TaskNode::worker` (sourced from the workflow step's
//! `skill` field) and the tool name in `inputs["tool"]`. A step resolves
//! to the real path iff its `worker` matches a server declared in some
//! on-disk vertical's `[mcp_servers]` section AND it names a tool;
//! everything else echoes.

use argo_agent_runner::workflow_adapters::{
    AuditWorkflowEmitter, CapabilityWorkflowChecker, McpToolDispatcher,
};
use argo_audit::AuditSink;
use argo_capability::CapabilityEngine;
use argo_mcp::{McpClientPool, PoolConfig as McpPoolConfig};
use argo_store::SessionStore;
use argo_types::{Principal, TenantId};
use argo_workflow::{MockRunner, TaskNode, TaskOutcome, TaskRunner, WorkerSessionRunner};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

/// Per-step hybrid [`TaskRunner`]: a step that names a known MCP server
/// **and** a tool dispatches for real through [`WorkerSessionRunner`];
/// every other step degrades to the deterministic [`MockRunner`] echo.
///
/// NO_THEATER: a real MCP step is never silently echoed. The echo path
/// is reserved for steps that name no dispatchable tool (prompt /
/// skill_invoke) or an unresolvable server.
pub(super) struct HybridWorkflowRunner {
    real: WorkerSessionRunner,
    mock: MockRunner,
    servers: BTreeSet<String>,
}

impl HybridWorkflowRunner {
    /// `true` iff `task` should take the real MCP-dispatch path: a
    /// non-empty `worker` that resolves to a known server, plus a
    /// non-empty `inputs["tool"]`.
    fn real_dispatchable(&self, task: &TaskNode) -> bool {
        !task.worker.is_empty()
            && self.servers.contains(&task.worker)
            && matches!(task.inputs.get("tool"), Some(Value::String(s)) if !s.is_empty())
    }
}

#[async_trait]
impl TaskRunner for HybridWorkflowRunner {
    async fn run(&self, task: &TaskNode, inputs: &BTreeMap<String, Value>) -> TaskOutcome {
        if self.real_dispatchable(task) {
            self.real.run(task, inputs).await
        } else {
            self.mock.run(task, inputs).await
        }
    }
}

/// Assemble the production [`HybridWorkflowRunner`] bound to
/// `(principal, tenant)`. Collects every on-disk vertical's
/// `[mcp_servers]` once, builds the real [`WorkerSessionRunner`] over a
/// per-tenant [`McpClientPool`], and records the resolvable server names
/// so the per-step router can decide real-vs-echo without re-reading the
/// manifests.
pub(super) fn build_hybrid_runner(
    store: &Arc<SessionStore>,
    principal: &Principal,
    tenant: TenantId,
) -> Result<HybridWorkflowRunner, String> {
    let servers = collect_mcp_servers();
    let server_names: BTreeSet<String> = servers.keys().cloned().collect();
    let real = build_real_runner(store, principal, tenant, servers)?;
    Ok(HybridWorkflowRunner {
        real,
        mock: MockRunner::echo(),
        servers: server_names,
    })
}

/// Assemble a production [`WorkerSessionRunner`] from the three
/// `argo_agent_runner::workflow_adapters` adapters, bound to
/// `(principal, tenant)`. The MCP dispatcher resolves server names from
/// `servers` (every on-disk vertical's `[mcp_servers]` section, mirroring
/// the live agent-turn registration); the capability engine is bound to
/// the request principal so a refused tool never reaches the wire. The
/// per-tenant [`McpClientPool`] keys connections by `(tenant_id, server)`
/// so tenant A's run can never reach tenant B's connector.
fn build_real_runner(
    store: &Arc<SessionStore>,
    principal: &Principal,
    tenant: TenantId,
    servers: BTreeMap<String, Value>,
) -> Result<WorkerSessionRunner, String> {
    let pool = McpClientPool::new(McpPoolConfig::default());
    let dispatcher = Arc::new(McpToolDispatcher::new(pool, servers));
    let sink = AuditSink::new_with_tenant(Arc::clone(store), tenant.to_string());
    let audit = Arc::new(AuditWorkflowEmitter::new(sink));
    let cwd = std::env::current_dir().map_err(|e| format!("cwd: {e}"))?;
    let engine = CapabilityEngine::with_workspace_defaults(&cwd);
    let role = resolve_role_str(principal);
    engine
        .bind(principal.clone(), role.to_string())
        .map_err(|e| format!("bind capability role: {e}"))?;
    let capabilities = Arc::new(CapabilityWorkflowChecker::new(Arc::new(engine)));
    Ok(WorkerSessionRunner::new(
        dispatcher,
        audit,
        capabilities,
        principal.clone(),
        tenant,
    ))
}

/// Best-effort role label for the capability binding. The principal was
/// already authorized for `WorkflowEdit` upstream, so this only picks the
/// role whose grants the per-step `ToolDispatch` check is evaluated
/// against.
fn resolve_role_str(principal: &Principal) -> &'static str {
    match principal {
        Principal::Local { .. } | Principal::Forge => "owner",
        Principal::Gateway { .. } | Principal::Acp { .. } => "user",
    }
}

/// Collect `server name → stdio transport` for every on-disk vertical's
/// `[mcp_servers]` section. Mirrors
/// `argo_agent_runner::dispatch::register_manifest_mcp_servers` so the
/// workflow runner resolves the same servers the live agent turn does.
fn collect_mcp_servers() -> BTreeMap<String, Value> {
    let mut out = BTreeMap::new();
    let Ok((verticals, _errors)) = argo_vertical::load_dir(&argo_vertical::default_dir()) else {
        return out;
    };
    for (_id, loaded) in verticals.list() {
        let Some(section) = &loaded.manifest.mcp_servers else {
            continue;
        };
        for (name, server) in &section.servers {
            out.insert(
                name.clone(),
                json!({
                    "transport": server.transport,
                    "command": server.command,
                    "args": server.args,
                }),
            );
        }
    }
    out
}

#[cfg(test)]
#[path = "routes_workflow_execute_runner_tests.rs"]
mod tests;
