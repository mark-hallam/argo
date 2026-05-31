//! Real-runner assembly for `routes_workflow_execute` — splits the
//! production [`WorkerSessionRunner`] wiring out of the parent route so
//! that file stays under the 500-LOC budget.
//!
//! Flag-gated by `ARGO_WORKFLOW_REAL_RUNNER` (default OFF): the live
//! `/workflows/.../execute` route keeps its deterministic `MockRunner`
//! echo behaviour unless the operator opts in, mirroring the
//! `ARGO_ORCH_LLM_EXECUTOR` pattern. When ON, the three
//! `argo_agent_runner::workflow_adapters` adapters are assembled into a
//! real runner that dispatches MCP tools, emits step-level audit rows,
//! and gates each tool on a capability check.

use argo_agent_runner::workflow_adapters::{
    AuditWorkflowEmitter, CapabilityWorkflowChecker, McpToolDispatcher,
};
use argo_audit::AuditSink;
use argo_capability::CapabilityEngine;
use argo_mcp::{McpClientPool, PoolConfig as McpPoolConfig};
use argo_store::SessionStore;
use argo_types::{Principal, TenantId};
use argo_workflow::WorkerSessionRunner;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::sync::Arc;

/// Env flag that swaps the demo-safe `MockRunner` for the real
/// [`WorkerSessionRunner`]. Default OFF.
const REAL_RUNNER_FLAG: &str = "ARGO_WORKFLOW_REAL_RUNNER";

/// `true` iff `ARGO_WORKFLOW_REAL_RUNNER` is `1`/`true`.
pub(super) fn real_runner_enabled() -> bool {
    std::env::var(REAL_RUNNER_FLAG)
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

/// Assemble a production [`WorkerSessionRunner`] from the three
/// `argo_agent_runner::workflow_adapters` adapters, bound to
/// `(principal, tenant)`. The MCP dispatcher resolves server names from
/// every on-disk vertical's `[mcp_servers]` section (mirroring the live
/// agent-turn registration); the capability engine is bound to the
/// request principal so a refused tool never reaches the wire.
pub(super) fn build_real_runner(
    store: &Arc<SessionStore>,
    principal: &Principal,
    tenant: TenantId,
) -> Result<WorkerSessionRunner, String> {
    let pool = McpClientPool::new(McpPoolConfig::default());
    let dispatcher = Arc::new(McpToolDispatcher::new(pool, collect_mcp_servers()));
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
