//! Tool dispatch: registry assembly, capability-checked calls,
//! and MCP dispatch wiring.
use argo_capability::CapabilityEngine;
use argo_mcp::{McpClient, McpContent};
use argo_memory::MemoryStore;
use argo_provider::ToolSchema as ProviderToolSchema;
use argo_tool::{builtin_tools, builtin_tools_with_memory, ToolContext, ToolRegistry, ToolSchema};
use argo_types::{
    ArgoResult, Capability, ContentBlock, Principal, ToolCallRequest, ToolCallResult, WorkspaceId,
};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;

/// Per-turn MCP clients keyed by server name.
#[derive(Debug, Default)]
pub(crate) struct McpConnectionPool {
    clients: tokio::sync::Mutex<HashMap<String, Arc<McpClient>>>,
}

impl McpConnectionPool {
    async fn client(&self, name: &str, transport: &Value) -> Result<Arc<McpClient>, String> {
        if let Some(client) = self.clients.lock().await.get(name).cloned() {
            return Ok(client);
        }
        let (command, args) = parse_stdio_transport(transport)?;
        let client = Arc::new(
            McpClient::connect_stdio(command, &args)
                .await
                .map_err(|e| format!("connect MCP server '{name}': {e}"))?,
        );
        self.clients
            .lock()
            .await
            .insert(name.to_string(), client.clone());
        Ok(client)
    }
}

/// Build a tool registry + capability engine that grants the local
/// OS user the `owner` role. When `memory` is `Some((store, ws,
/// vertical_id))`, the registered `memory` tool is wired through the
/// supplied `MemoryStore` (per claude-memory-tool-wire); otherwise it
/// stays a stub that errors with the historical "memory store not
/// wired" message at dispatch.
pub(crate) fn build_dispatch_ctx(
    cwd: &Path,
    memory: Option<(Arc<MemoryStore>, WorkspaceId, Option<String>)>,
) -> ArgoResult<(ToolRegistry, CapabilityEngine, Principal)> {
    let mut reg = ToolRegistry::new();
    let tools = match memory {
        Some((store, ws, vertical_id)) => {
            builtin_tools_with_memory(cwd.to_path_buf(), store, ws, vertical_id)
        }
        None => builtin_tools(cwd.to_path_buf()),
    };
    for t in tools {
        reg.register(t);
    }
    reg.set_mcp_pool(Arc::new(McpConnectionPool::default()));
    register_manifest_mcp_servers(&mut reg);
    let engine = CapabilityEngine::with_defaults();
    let os_user = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "argo".into());
    let principal = Principal::Local { os_user };
    engine.bind(principal.clone(), "owner".into())?;
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
pub(crate) async fn dispatch_one(
    registry: &ToolRegistry,
    engine: &CapabilityEngine,
    principal: &Principal,
    cwd: &Path,
    call: &ToolCallRequest,
) -> ToolCallResult {
    let started = std::time::Instant::now();
    if registry.get(&call.name).is_none() {
        if let Some(r) = dispatch_mcp(registry, engine, principal, call, started).await {
            return r;
        }
    }
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
            elapsed_ms(started),
        ),
    }
}

async fn dispatch_mcp(
    registry: &ToolRegistry,
    engine: &CapabilityEngine,
    principal: &Principal,
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
        if let Some(e) = discover_mcp_tools(registry, &pool).await {
            if registry.mcp_route(&call.name).is_none() {
                return Some(argo_tool::error_text(call.id, e, elapsed_ms(started)));
            }
        }
    }
    let (server_name, transport) = registry.mcp_route(&call.name)?;
    let client = match pool.client(&server_name, &transport).await {
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

async fn discover_mcp_tools(registry: &ToolRegistry, pool: &McpConnectionPool) -> Option<String> {
    let mut first_err = None;
    for (server_name, transport) in registry.undiscovered_mcp_servers() {
        let client = match pool.client(&server_name, &transport).await {
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

fn elapsed_ms(started: std::time::Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
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
