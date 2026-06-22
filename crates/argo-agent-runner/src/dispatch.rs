//! Tool dispatch: registry assembly, capability-checked calls,
//! and MCP dispatch wiring.
use argo_capability::CapabilityEngine;
use argo_identity::MemoryNs;
use argo_mcp::McpContent;
use argo_memory::MemoryStore;
use argo_orchestrator::{HandoffId, OrchestratorStore, SessionId as OrchSessionId};
use argo_provider::ToolSchema as ProviderToolSchema;
use argo_store::SessionStore;
use argo_tool::{
    builtin_tools_with_runtime_and_dispatchers, ProcessRegistry, ToolContext, ToolRegistry,
    ToolSchema,
};
use argo_types::{
    ArgoResult, Capability, ContentBlock, Principal, SpawnKind, TenantId, ToolCallRequest,
    ToolCallResult, WorkspaceId,
};
use serde_json::Value;
use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;

#[path = "dispatch_mcp_pool.rs"]
mod dispatch_mcp_pool;
pub(crate) use dispatch_mcp_pool::McpConnectionPool;

/// Role name bound to a Genesis-hired teammate carrying ONLY its
/// derived least-privilege capability set (DEC-025 capability fence).
/// Distinct from `owner` so the binding is auditable + so the broad
/// owner grants (`ToolDispatch "*"`, `NetEgress "**"`, `SecretRead "*"`)
/// are never in this principal's effective set.
const GENESIS_WORKER_ROLE: &str = "genesis-worker";

/// Build a tool registry + capability engine for the active turn.
///
/// Runtime-backed tools are registered with the stores from the active
/// turn: `memory` when a workspace exists, `delegate_task` through the
/// orchestrator store, and `session_search` through the session store.
/// `memory_namespace`, when `Some`, binds the `memory` tool to a worker
/// [`MemoryNs`] so the `query_scored` / `*_team` actions reach the
/// per-worker namespace store; sessions without a `worker_id` pass
/// `None` and the tool falls back to the workspace-global path for
/// legacy `list`/`read`/`write`.
///
/// ## Capability binding (DEC-025 Genesis fence)
///
/// - `worker_profile = None` → the principal is bound to the all-access
///   `owner` role (the historical default for ad-hoc + operator-trusted
///   catalog-vertical sessions), plus the `BackgroundProcess` spawn
///   grant.
/// - `worker_profile = Some(profile)` → the principal is bound to a
///   **least-privilege** [`GENESIS_WORKER_ROLE`] carrying ONLY the
///   profile's derived `ToolDispatch` grants. This stops a Genesis-hired
///   teammate inheriting the owner's `*` keys: anything not in the
///   profile is denied at the existing `engine.check` dispatch seam,
///   with **zero new enforcement code**. The broad `BackgroundProcess`
///   spawn grant is intentionally NOT extended to a fenced worker.
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_dispatch_ctx(
    cwd: &Path,
    memory: Option<(Arc<MemoryStore>, WorkspaceId, Option<String>)>,
    memory_namespace: Option<MemoryNs>,
    orchestrator: Arc<OrchestratorStore>,
    session_store: Arc<SessionStore>,
    worker_profile: Option<&argo_capability::WorkerCapabilityProfile>,
    // DEC-041 (D), first-principles capability model: true when this session
    // acts AS A WORKER (an AI teammate), false only for a bare human operator
    // session. `owner` is the human's identity (wide-open) and must NEVER bind
    // to a worker session — a worker holding `owner` is privilege-escalation,
    // exploitable via ANY path (an autonomous duty fire OR a prompt-injected
    // interactive chat), not just the autonomous one. So: a worker session with
    // no derived profile fences to deny-all (fail-closed); only a bare human
    // session keeps `owner`.
    is_worker_session: bool,
    // WAVE-2 ⑥: the in-turn `enqueue_action` delivery backend. `Some` only on
    // the worker job-run path (argo-cli's `BridgeDeliverDispatcher`); `None`
    // leaves the tool with its unwired default (still in the schema, errors on
    // use). The injected backend submits actions through the approval gate and
    // never executes — so this does NOT add an auto-deliver path.
    deliver_dispatcher: Option<Arc<dyn argo_tool::DeliverDispatcher>>,
    // The in-turn `clarify` escalation backend (argo-cli's
    // `ClarifyEscalateDispatcher`). `Some` only on the worker job-run path;
    // `None` leaves the tool with its unwired default (still in the schema).
    clarify_dispatcher: Option<Arc<dyn argo_tool::ClarifyDispatcher>>,
) -> ArgoResult<(ToolRegistry, CapabilityEngine, Principal)> {
    let mut reg = ToolRegistry::new();
    let tools = builtin_tools_with_runtime_and_dispatchers(
        cwd.to_path_buf(),
        memory,
        memory_namespace,
        orchestrator,
        session_store,
        // No production message dispatcher is wired at this seam yet (the
        // `send_message` tool keeps its unwired default here, unchanged).
        None,
        deliver_dispatcher,
        clarify_dispatcher,
    );
    for t in tools {
        reg.register(t);
    }
    // Inject the active tenant's stored credentials when a connector is
    // spawned. The credential vault always lives under the canonical
    // workspace root (`$ARGO_WORKSPACE` → `$ARGO_HOME` → current_dir, via
    // `argo_config::cred::workspace_root`) — the SAME base the gateway/CLI
    // WRITE path (`set_tenant`) uses, so a credential written in prod (under
    // the durable `/data` mount) is read back from the same durable path
    // rather than from a per-worker sandbox `cwd` or the throwaway `/app`
    // image layer. Tenant-scoped at the spawn seam.
    reg.set_mcp_pool(Arc::new(McpConnectionPool::with_workspace(
        argo_config::cred::workspace_root(),
    )));
    register_manifest_mcp_servers(&mut reg);
    let engine = CapabilityEngine::with_workspace_defaults(cwd);
    let os_user = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "argo".into());
    let principal = Principal::Local { os_user };
    match worker_profile {
        Some(profile) => {
            // Genesis-hired teammate: bind a least-privilege role built
            // from the worker's derived profile instead of `owner`.
            // Deny-by-default — anything not in the profile is refused
            // at the dispatch seam.
            engine.add_role(argo_types::CapabilityRole {
                name: GENESIS_WORKER_ROLE.to_string(),
                inherits_from: None,
                capabilities: profile.to_capability_set(),
            })?;
            engine.bind(principal.clone(), GENESIS_WORKER_ROLE.to_string())?;
        }
        None if is_worker_session => {
            // DEC-041 (D): a WORKER session with no derived profile must NEVER
            // bind `owner`. Fence it to a deny-all profile (empty allow-set →
            // only the always-safe minimal teammate tools survive into the
            // capability set), so the agent cannot reach external/mutating tools
            // (mcp gmail/gcal, message send, fs-write outside its scope, etc.).
            // Fail-closed — this closes the privilege-escalation hole for every
            // worker session, autonomous (duty fire) and interactive alike.
            let deny_all = argo_capability::WorkerCapabilityProfile::derive(
                Vec::<String>::new(),
                Vec::<String>::new(),
                Vec::<String>::new(),
                Vec::<String>::new(),
            );
            engine.add_role(argo_types::CapabilityRole {
                name: GENESIS_WORKER_ROLE.to_string(),
                inherits_from: None,
                capabilities: deny_all.to_capability_set(),
            })?;
            engine.bind(principal.clone(), GENESIS_WORKER_ROLE.to_string())?;
        }
        None => {
            // Interactive human session (the operator's own CLI): the human
            // IS the owner, so the wide-open role is correct here.
            engine.bind(principal.clone(), "owner".into())?;
            engine.grant_dynamic(
                principal.clone(),
                Capability::Spawn {
                    spawn_kind: SpawnKind::BackgroundProcess,
                },
            );
        }
    }
    Ok((reg, engine, principal))
}

fn register_manifest_mcp_servers(reg: &mut ToolRegistry) {
    let Ok((verticals, _errors)) = argo_vertical::load_dir(&argo_vertical::default_dir()) else {
        return;
    };
    for (_id, loaded) in verticals.list() {
        let Some(section) = &loaded.manifest.mcp_servers else {
            continue;
        };
        for (name, server) in &section.servers {
            reg.register_mcp_server(
                name.clone(),
                serde_json::json!({
                    "transport": server.transport,
                    "command": server.command,
                    "args": server.args,
                }),
            );
        }
    }
}

/// Lift the registry's tool schemas into the wire form the provider
/// transport expects, dropping any whose name appears in `deny`.
pub(crate) fn provider_schemas_filtered(
    reg: &ToolRegistry,
    deny: &HashSet<String>,
) -> Vec<ProviderToolSchema> {
    reg.schemas()
        .into_iter()
        .filter(|s| !deny.contains(&s.name))
        .map(|s| ProviderToolSchema {
            name: s.name,
            description: s.description,
            parameters: s.parameters,
        })
        .collect()
}

/// Cap-check + dispatch one tool call. Errors during dispatch are
/// turned into a `ToolCallResult { is_error: true, ... }` so the
/// loop can surface them to the model rather than abort the turn.
#[cfg(test)]
pub(crate) async fn dispatch_one(
    registry: &ToolRegistry,
    engine: &CapabilityEngine,
    principal: &Principal,
    cwd: &Path,
    session_id: &OrchSessionId,
    call: &ToolCallRequest,
) -> ToolCallResult {
    let tenant_id = TenantId(uuid::Uuid::from_u128(1));
    dispatch_one_with_tenant(
        registry, engine, principal, tenant_id, cwd, session_id, call,
    )
    .await
}

/// Cap-check + dispatch one tool call under an explicit tenant.
#[cfg(test)]
pub(crate) async fn dispatch_one_with_tenant(
    registry: &ToolRegistry,
    engine: &CapabilityEngine,
    principal: &Principal,
    tenant_id: TenantId,
    cwd: &Path,
    session_id: &OrchSessionId,
    call: &ToolCallRequest,
) -> ToolCallResult {
    dispatch_one_with_runtime(
        registry, engine, principal, tenant_id, cwd, session_id, call, None, None,
    )
    .await
}

/// Cap-check + dispatch one tool call with optional per-session runtime
/// state attached to the tool context.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn dispatch_one_with_runtime(
    registry: &ToolRegistry,
    engine: &CapabilityEngine,
    principal: &Principal,
    tenant_id: TenantId,
    cwd: &Path,
    session_id: &OrchSessionId,
    call: &ToolCallRequest,
    background_processes: Option<Arc<ProcessRegistry>>,
    current_handoff_id: Option<HandoffId>,
) -> ToolCallResult {
    let started = std::time::Instant::now();
    let mut ctx = ToolContext::new_with_tenant(principal.clone(), cwd.to_path_buf(), tenant_id);
    ctx.call_id = call.id;
    ctx.session_id = session_id.clone();
    ctx.background_processes = background_processes;
    ctx.current_handoff_id = current_handoff_id;
    if registry.get(&call.name).is_none() {
        if let Some(r) =
            dispatch_mcp(registry, engine, principal, ctx.tenant_id, call, started).await
        {
            return r;
        }
    }
    match registry
        .dispatch(&call.name, call.arguments.clone(), &ctx, engine)
        .await
    {
        Ok(r) => r,
        Err(e) => argo_tool::error_text(
            call.id,
            format!("dispatch '{}' failed: {e}", call.name),
            elapsed_ms(started),
        ),
    }
}

async fn dispatch_mcp(
    registry: &ToolRegistry,
    engine: &CapabilityEngine,
    principal: &Principal,
    tenant_id: TenantId,
    call: &ToolCallRequest,
    started: std::time::Instant,
) -> Option<ToolCallResult> {
    if let Err(e) = engine.check(
        principal,
        &Capability::ToolDispatch {
            tool_name_glob: call.name.clone(),
        },
    ) {
        return Some(argo_tool::error_text(
            call.id,
            format!("dispatch '{}' failed: {e}", call.name),
            elapsed_ms(started),
        ));
    }
    let pool = mcp_pool(registry)?;
    if registry.mcp_route(&call.name).is_none() {
        if let Some(e) = discover_mcp_tools(registry, &pool, tenant_id).await {
            if registry.mcp_route(&call.name).is_none() {
                return Some(argo_tool::error_text(call.id, e, elapsed_ms(started)));
            }
        }
    }
    let (server_name, transport) = registry.mcp_route(&call.name)?;
    let client = match pool.client(tenant_id, &server_name, &transport).await {
        Ok(c) => c,
        Err(e) => return Some(argo_tool::error_text(call.id, e, elapsed_ms(started))),
    };
    Some(
        match client.call_tool(&call.name, call.arguments.clone()).await {
            Ok(result) => mcp_result_to_tool_result(call.id, result, elapsed_ms(started)),
            Err(e) => argo_tool::error_text(
                call.id,
                format!("MCP tool '{}' failed: {e}", call.name),
                elapsed_ms(started),
            ),
        },
    )
}

async fn discover_mcp_tools(
    registry: &ToolRegistry,
    pool: &McpConnectionPool,
    tenant_id: TenantId,
) -> Option<String> {
    let mut first_err = None;
    for (server_name, transport) in registry.undiscovered_mcp_servers() {
        let client = match pool.client(tenant_id, &server_name, &transport).await {
            Ok(c) => c,
            Err(e) => {
                first_err.get_or_insert(e);
                continue;
            }
        };
        match client.list_tools().await {
            Ok(tools) => registry.record_mcp_tools(
                &server_name,
                tools
                    .into_iter()
                    .map(|t| ToolSchema {
                        name: t.name,
                        description: t.description,
                        parameters: t.input_schema,
                    })
                    .collect(),
            ),
            Err(e) => {
                first_err.get_or_insert(format!("discover MCP tools from '{server_name}': {e}"));
            }
        }
    }
    first_err
}

fn mcp_pool(registry: &ToolRegistry) -> Option<Arc<McpConnectionPool>> {
    registry
        .mcp_pool()
        .and_then(|p| Arc::downcast::<McpConnectionPool>(p).ok())
}

fn mcp_result_to_tool_result(
    id: argo_types::ToolCallId,
    result: argo_mcp::CallToolResult,
    duration_ms: u64,
) -> ToolCallResult {
    let mut content: Vec<ContentBlock> = result
        .content
        .into_iter()
        .map(|c| match c {
            McpContent::Text { text } => ContentBlock::Text { text },
            McpContent::Image { data, mime_type } => ContentBlock::Image {
                url: data.unwrap_or_default(),
                alt: mime_type,
            },
            McpContent::Unknown => ContentBlock::Text {
                text: "unsupported MCP content".into(),
            },
        })
        .collect();
    if content.is_empty() {
        content.push(ContentBlock::Text {
            text: String::new(),
        });
    }
    ToolCallResult {
        id,
        is_error: result.is_error,
        content,
        duration_ms,
        tokens_in: None,
        tokens_out: None,
    }
}

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
        .map(|xs| {
            xs.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    Ok((command, args))
}

#[cfg(test)]
#[path = "dispatch_tenant_isolation_tests.rs"]
mod dispatch_tenant_isolation_tests;

fn elapsed_ms(started: std::time::Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use argo_tool::builtin_tools;
    use std::path::PathBuf;

    #[test]
    fn filtered_schemas_drop_denied_tools() {
        let mut reg = ToolRegistry::new();
        for t in builtin_tools(PathBuf::from(".")) {
            reg.register(t);
        }
        let unfiltered = provider_schemas_filtered(&reg, &HashSet::new());
        let names: Vec<String> = unfiltered.iter().map(|s| s.name.clone()).collect();
        assert!(names.iter().any(|n| n == "delegate_task"));
        assert!(names.iter().any(|n| n == "memory"));

        let mut deny: HashSet<String> = HashSet::new();
        deny.insert("delegate_task".into());
        deny.insert("memory".into());

        let filtered = provider_schemas_filtered(&reg, &deny);
        let filtered_names: Vec<String> = filtered.iter().map(|s| s.name.clone()).collect();
        assert!(!filtered_names.iter().any(|n| n == "delegate_task"));
        assert!(!filtered_names.iter().any(|n| n == "memory"));
        assert_eq!(filtered.len(), unfiltered.len() - 2);
    }

    #[test]
    fn filtered_schemas_no_op_with_empty_deny_set() {
        let mut reg = ToolRegistry::new();
        for t in builtin_tools(PathBuf::from(".")) {
            reg.register(t);
        }
        let filtered = provider_schemas_filtered(&reg, &HashSet::new());
        assert_eq!(reg.schemas().len(), filtered.len());
    }
}
