//! Tool dispatch — assembles a `ToolRegistry` over the workspace
//! cwd, runs cap-checked dispatch, and returns a canonical
//! `ToolCallResult`. Used per round inside the agent loop.
use argo_capability::CapabilityEngine;
use argo_provider::ToolSchema as ProviderToolSchema;
use argo_tool::{builtin_tools, ToolContext, ToolRegistry};
use argo_types::{ArgoResult, Principal, ToolCallRequest, ToolCallResult};
use std::path::Path;

/// Build a tool registry + capability engine that grants the
/// local OS user the `owner` role. `argo chat` runs against the
/// invoking user's workspace, so a permissive grant is correct
/// here — the daemon path will narrow this once tenants land.
pub(crate) fn build_dispatch_ctx(
    cwd: &Path,
) -> ArgoResult<(ToolRegistry, CapabilityEngine, Principal)> {
    let mut reg = ToolRegistry::new();
    for t in builtin_tools(cwd.to_path_buf()) {
        reg.register(t);
    }
    let engine = CapabilityEngine::with_defaults();
    let os_user = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "argo".into());
    let principal = Principal::Local { os_user };
    engine.bind(principal.clone(), "owner".into())?;
    Ok((reg, engine, principal))
}

/// Lift the registry's tool schemas into the wire form the
/// provider transport expects.
pub(crate) fn provider_schemas(reg: &ToolRegistry) -> Vec<ProviderToolSchema> {
    reg.schemas()
        .into_iter()
        .map(|s| ProviderToolSchema {
            name: s.name,
            description: s.description,
            parameters: s.parameters,
        })
        .collect()
}

/// Cap-check + dispatch one tool call. Errors during dispatch are
/// turned into a `ToolCallResult { is_error: true, … }` so the
/// loop can surface them to the model rather than abort the turn.
pub(crate) async fn dispatch_one(
    registry: &ToolRegistry,
    engine: &CapabilityEngine,
    principal: &Principal,
    cwd: &Path,
    call: &ToolCallRequest,
) -> ToolCallResult {
    let started = std::time::Instant::now();
    let mut ctx = ToolContext::new(principal.clone(), cwd.to_path_buf());
    ctx.call_id = call.id;
    match registry
        .dispatch(&call.name, call.arguments.clone(), &ctx, engine)
        .await
    {
        Ok(r) => r,
        Err(e) => argo_tool::error_text(
            call.id,
            format!("dispatch '{}' failed: {e}", call.name),
            u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
        ),
    }
}
