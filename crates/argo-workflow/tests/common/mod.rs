//! Shared test fixtures for the claim-5 LIVE MCP regression suite.
//!
//! Lives under `tests/common/` so each `tests/<name>.rs` integration
//! binary can `mod common;` and pull the same fakes — avoids
//! duplicating fixture types across the split test files. Keeping
//! this module under the 500-LOC ceiling is load-bearing because
//! `loc-budget` CI walks every `.rs` file under `crates/`.

#![allow(dead_code)]
// Each integration test imports a slightly different subset of these
// fixtures; `dead_code` allowances on this shared module suppress
// noisy warnings without weakening crate-wide lints.

use argo_types::{Principal, TenantId};
use argo_workflow::{
    StepAuditContext, TaskNode, ToolDispatchError, ToolDispatcher, Workflow, WorkflowAuditEmitter,
    WorkflowCapabilityChecker,
};
use async_trait::async_trait;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

// ============================================================
// Fake MCP server: tenant-keyed routing table.
// ============================================================

/// Routing key — what the runner hands the dispatcher per call.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RouteKey {
    pub tenant_id: TenantId,
    pub server: String,
    pub tool: String,
}

#[derive(Debug, Clone)]
pub struct RecordedCall {
    pub key: RouteKey,
    pub args: serde_json::Value,
}

/// Per-route canned response. Mirrors what a real MCP server would
/// emit: either a JSON body, or one of the dispatcher's typed
/// failures.
#[derive(Clone)]
enum RouteResponse {
    Ok(serde_json::Value),
    Tool(String),
    Transport(String),
}

/// In-process fake MCP server pool. Models a real
/// `argo_mcp::McpClientPool` from the runner's point of view — the
/// runner cannot tell them apart at the `ToolDispatcher` seam.
pub struct FakeMcpServerPool {
    routes: Mutex<std::collections::HashMap<RouteKey, RouteResponse>>,
    calls: Mutex<Vec<RecordedCall>>,
}

impl FakeMcpServerPool {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            routes: Mutex::new(std::collections::HashMap::new()),
            calls: Mutex::new(Vec::new()),
        })
    }

    pub fn route_ok(
        self: &Arc<Self>,
        tenant_id: TenantId,
        server: &str,
        tool: &str,
        response: serde_json::Value,
    ) {
        self.set_route(tenant_id, server, tool, RouteResponse::Ok(response));
    }

    pub fn route_tool_error(
        self: &Arc<Self>,
        tenant_id: TenantId,
        server: &str,
        tool: &str,
        message: &str,
    ) {
        self.set_route(
            tenant_id,
            server,
            tool,
            RouteResponse::Tool(message.to_string()),
        );
    }

    pub fn route_transport_error(
        self: &Arc<Self>,
        tenant_id: TenantId,
        server: &str,
        tool: &str,
        message: &str,
    ) {
        self.set_route(
            tenant_id,
            server,
            tool,
            RouteResponse::Transport(message.to_string()),
        );
    }

    fn set_route(&self, tenant_id: TenantId, server: &str, tool: &str, resp: RouteResponse) {
        let key = RouteKey {
            tenant_id,
            server: server.to_string(),
            tool: tool.to_string(),
        };
        if let Ok(mut g) = self.routes.lock() {
            g.insert(key, resp);
        }
    }

    pub fn calls(&self) -> Vec<RecordedCall> {
        self.calls.lock().expect("calls lock").clone()
    }
}

#[async_trait]
impl ToolDispatcher for FakeMcpServerPool {
    async fn call(
        &self,
        tenant_id: &TenantId,
        server: &str,
        tool: &str,
        args: serde_json::Value,
    ) -> Result<serde_json::Value, ToolDispatchError> {
        let key = RouteKey {
            tenant_id: *tenant_id,
            server: server.to_string(),
            tool: tool.to_string(),
        };
        self.calls.lock().expect("calls lock").push(RecordedCall {
            key: key.clone(),
            args,
        });
        let response = self.routes.lock().expect("routes lock").get(&key).cloned();
        match response {
            Some(RouteResponse::Ok(v)) => Ok(v),
            Some(RouteResponse::Tool(m)) => Err(ToolDispatchError::ToolError(m)),
            Some(RouteResponse::Transport(m)) => Err(ToolDispatchError::Transport(m)),
            None => Err(ToolDispatchError::BadRequest(format!(
                "fake MCP server: no route registered for ({tenant_id:?}, {server}, {tool})",
            ))),
        }
    }
}

// ============================================================
// Audit emitter: capture every step row with full context.
// ============================================================

#[derive(Debug, Clone)]
pub struct AuditEvent {
    pub kind: &'static str,
    pub task_id: String,
    pub tool: String,
    pub worker: String,
    pub tenant_id: TenantId,
    pub principal_label: String,
    pub reason_or_response: serde_json::Value,
}

pub struct CapturingAudit {
    events: Mutex<Vec<AuditEvent>>,
}

impl CapturingAudit {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            events: Mutex::new(Vec::new()),
        })
    }

    pub fn events(&self) -> Vec<AuditEvent> {
        self.events.lock().expect("events lock").clone()
    }
}

pub fn principal_label(p: &Principal) -> String {
    match p {
        Principal::Local { os_user } => format!("local:{os_user}"),
        Principal::Gateway { adapter, identity } => format!("{adapter}:{identity}"),
        Principal::Acp { harness, identity } => format!("{harness}:{identity}"),
        Principal::Forge => "forge".to_string(),
    }
}

#[async_trait]
impl WorkflowAuditEmitter for CapturingAudit {
    async fn step_executed(
        &self,
        ctx: &StepAuditContext<'_>,
        response: &serde_json::Value,
    ) -> Result<(), String> {
        self.events.lock().expect("events lock").push(AuditEvent {
            kind: "workflow.step.executed",
            task_id: ctx.task_id.to_string(),
            tool: ctx.tool.to_string(),
            worker: ctx.worker.to_string(),
            tenant_id: *ctx.tenant_id,
            principal_label: principal_label(ctx.principal),
            reason_or_response: response.clone(),
        });
        Ok(())
    }

    async fn step_failed(&self, ctx: &StepAuditContext<'_>, reason: &str) {
        self.events.lock().expect("events lock").push(AuditEvent {
            kind: "workflow.step.failed",
            task_id: ctx.task_id.to_string(),
            tool: ctx.tool.to_string(),
            worker: ctx.worker.to_string(),
            tenant_id: *ctx.tenant_id,
            principal_label: principal_label(ctx.principal),
            reason_or_response: serde_json::Value::String(reason.to_string()),
        });
    }
}

// ============================================================
// Capability checker: explicit (principal, tool) grant table.
// ============================================================

pub struct TableCapabilities {
    grants: Vec<(Principal, String)>,
}

impl TableCapabilities {
    pub fn granting<I>(grants: I) -> Arc<Self>
    where
        I: IntoIterator<Item = (Principal, String)>,
    {
        Arc::new(Self {
            grants: grants.into_iter().collect(),
        })
    }
}

impl WorkflowCapabilityChecker for TableCapabilities {
    fn check_dispatch(&self, principal: &Principal, tool: &str) -> Result<(), String> {
        let has = self
            .grants
            .iter()
            .any(|(p, t)| p == principal && t == tool);
        if has {
            Ok(())
        } else {
            Err(format!(
                "principal {} not bound to ToolDispatch[{tool}]",
                principal_label(principal),
            ))
        }
    }
}

// ============================================================
// Workflow helpers.
// ============================================================

pub fn one_step_workflow(server: &str, tool: &str, args: serde_json::Value) -> Workflow {
    let mut inputs = BTreeMap::new();
    inputs.insert(
        "tool".into(),
        serde_json::Value::String(tool.to_string()),
    );
    inputs.insert("args".into(), args);
    Workflow {
        id: format!("claim5-live-{server}-{tool}"),
        description: "Claim-5 LIVE — single-step workflow exercising fake MCP server".into(),
        tasks: vec![TaskNode {
            id: "call".into(),
            worker: server.to_string(),
            persona: None,
            inputs,
            outputs: BTreeMap::new(),
            depends_on: vec![],
            cache_key: None,
        }],
        edges: vec![],
    }
}
