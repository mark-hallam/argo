//! Tool dispatch — assembles a `ToolRegistry` over the workspace
//! cwd, runs cap-checked dispatch, and returns a canonical
//! `ToolCallResult`. Used per round inside the agent loop.
use argo_capability::CapabilityEngine;
use argo_provider::ToolSchema as ProviderToolSchema;
use argo_tool::{builtin_tools, ToolContext, ToolRegistry};
use argo_types::{ArgoResult, Principal, ToolCallRequest, ToolCallResult};
use std::collections::HashSet;
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
/// provider transport expects, dropping any whose name appears
/// in `deny`. Worker v1 (#721) advertises only the already-
/// allowed set to the model so it doesn't waste turns calling
/// tools the vertical manifest will deny anyway. The per-call
/// deny check in `run_turn_with_tools` is still the hard
/// authoritative guard — this is the discoverability layer.
/// Pass `&HashSet::new()` to behave as an unfiltered lift.
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// Worker v1 (#721): the model-facing schema list must drop
    /// tools the vertical denies, so the agent doesn't waste
    /// turns calling things the dispatcher will reject anyway.
    /// This locks the discoverability filter that prevents the
    /// `delegate_task`-tried-and-denied trace observed in the
    /// first live Anthropic worker run.
    #[test]
    fn filtered_schemas_drop_denied_tools() {
        let mut reg = ToolRegistry::new();
        for t in builtin_tools(PathBuf::from(".")) {
            reg.register(t);
        }
        let unfiltered = provider_schemas_filtered(&reg, &HashSet::new());
        let names: Vec<String> = unfiltered.iter().map(|s| s.name.clone()).collect();
        // Smoke: builtin set actually contains the deny targets we
        // assert on below — otherwise the filter test is vacuous.
        assert!(
            names.iter().any(|n| n == "delegate_task"),
            "builtin set must include delegate_task for this test to be meaningful; got {names:?}"
        );
        assert!(
            names.iter().any(|n| n == "memory"),
            "builtin set must include memory for this test to be meaningful; got {names:?}"
        );

        let mut deny: HashSet<String> = HashSet::new();
        deny.insert("delegate_task".into());
        deny.insert("memory".into());

        let filtered = provider_schemas_filtered(&reg, &deny);
        let filtered_names: Vec<String> = filtered.iter().map(|s| s.name.clone()).collect();
        assert!(
            !filtered_names.iter().any(|n| n == "delegate_task"),
            "delegate_task must be filtered out; got {filtered_names:?}"
        );
        assert!(
            !filtered_names.iter().any(|n| n == "memory"),
            "memory must be filtered out; got {filtered_names:?}"
        );
        // And non-denied tools still surface.
        assert_eq!(
            filtered.len(),
            unfiltered.len() - 2,
            "filtered set must drop exactly the two denied tools",
        );
    }

    /// Empty deny set is the unbound-session path: filter is a
    /// no-op so every registered tool surfaces.
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
