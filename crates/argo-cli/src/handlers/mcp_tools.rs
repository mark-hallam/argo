//! `argo mcp tools <server>` — discover the tools an MCP server exposes
//! (`claude-mcp-tool-discovery-vis`).
//!
//! Resolves the named server's stdio transport via the requested
//! vertical's `[mcp_servers]` declaration, calls `tools/list` over
//! the connection (reusing the per-process pool from
//! [`super::worker_actions_executor_mcp::default_pool`]), and renders
//! the result as markdown (default) or NDJSON.
//!
//! Soft-dep on `codex-mcp-server-registry-cli`: when that lands the
//! lookup will fall back to the SQL registry; today the per-vertical
//! manifest is the only configuration site, so `--vertical` is
//! required.
use argo_mcp::{McpClient, McpToolSchema};
use argo_types::{ArgoError, ArgoResult, TenantId};
use argo_vertical::{load_dir, McpServerDeclaration};
use regex::Regex;
use std::sync::Arc;
use uuid::Uuid;

use super::worker_actions_executor_mcp::{default_pool, shared_client_pool};

/// Inputs for [`run`].
#[derive(Debug)]
pub struct ToolsArgs {
    /// MCP server id as declared in the vertical's `[mcp_servers]`.
    pub server_id: String,
    /// Vertical id whose manifest declares the server.
    pub vertical: String,
    /// `markdown` (default) or `json` (NDJSON one-tool-per-line).
    pub format: String,
    /// Equivalent to `--format json`.
    pub json: bool,
    /// Optional regex narrowing tools by name.
    pub filter: Option<String>,
    /// Omit descriptions in either output mode.
    pub schema_only: bool,
}

/// Top-level handler — resolves the server, calls `tools/list`,
/// renders.
pub async fn run(args: ToolsArgs) -> ArgoResult<()> {
    let server_decl = resolve_server_declaration(&args.vertical, &args.server_id)?;
    let regex = compile_filter(args.filter.as_deref())?;
    let tools = list_tools(&args.server_id, &server_decl).await?;
    let filtered = apply_filter(tools, regex.as_ref());
    let chosen = if args.json {
        "json"
    } else {
        args.format.as_str()
    };
    match chosen {
        "json" => render_ndjson(&filtered, args.schema_only),
        "markdown" => {
            print!(
                "{}",
                render_markdown(&args.server_id, &args.vertical, &filtered, args.schema_only)
            );
            Ok(())
        }
        other => Err(ArgoError::Provider(format!(
            "unknown --format '{other}' — use 'markdown' or 'json'"
        ))),
    }
}

/// Walk the workspace's `verticals/` directory, find the requested
/// vertical, and return its declaration for `server_id` from the
/// `[mcp_servers]` table.
fn resolve_server_declaration(
    vertical: &str,
    server_id: &str,
) -> ArgoResult<McpServerDeclaration> {
    let dir = argo_vertical::default_dir();
    let (registry, errs) = load_dir(&dir)?;
    if !errs.is_empty() {
        let summary: Vec<String> = errs
            .iter()
            .map(|e| format!("{}: {}", e.dir.display(), e.error))
            .collect();
        eprintln!(
            "argo mcp tools: {} vertical(s) failed to load: {}",
            errs.len(),
            summary.join(", ")
        );
    }
    let loaded = registry.get(vertical).ok_or_else(|| {
        ArgoError::Provider(format!(
            "vertical '{vertical}' not found under '{}'",
            dir.display()
        ))
    })?;
    let servers = loaded.manifest.mcp_servers.as_ref().ok_or_else(|| {
        ArgoError::Provider(format!(
            "vertical '{vertical}' has no [mcp_servers] section — \
             declare the server before discovering its tools"
        ))
    })?;
    let decl = servers.servers.get(server_id).ok_or_else(|| {
        let known: Vec<String> = servers.servers.keys().cloned().collect();
        let known_msg = if known.is_empty() {
            "no servers declared".to_string()
        } else {
            format!("declared: {}", known.join(", "))
        };
        ArgoError::Provider(format!(
            "vertical '{vertical}' [mcp_servers] has no entry '{server_id}' ({known_msg})"
        ))
    })?;
    Ok(decl.clone())
}

/// Acquire (or reuse) a connection from the process-wide pool and
/// call `tools/list`.
async fn list_tools(
    server_id: &str,
    decl: &McpServerDeclaration,
) -> ArgoResult<Vec<McpToolSchema>> {
    if decl.transport != "stdio" {
        return Err(ArgoError::Provider(format!(
            "MCP server '{server_id}' uses transport '{}' but tool \
             discovery only supports 'stdio' today",
            decl.transport
        )));
    }
    let pool = default_pool();
    let client: Arc<McpClient> = {
        let mut guard = pool.lock().await;
        if let Some(existing) = guard.get(server_id) {
            existing.clone()
        } else {
            // mission-mcp-pool-wire-dispatch: cold spawn routed through
            // the process-wide warm [`McpClientPool`]. Discovery is an
            // operator CLI command with no per-call tenant scope, so a
            // nil [`TenantId`] keeps every `argo mcp tools` invocation
            // under one pool key (the underlying child stays warm for
            // subsequent action-level dispatches against `server_id`).
            let command = decl.command.clone();
            let args = decl.args.clone();
            let arc = shared_client_pool()
                .acquire(
                    TenantId(Uuid::nil()),
                    server_id.to_string(),
                    || async move { McpClient::connect_stdio(&command, &args).await },
                )
                .await
                .map_err(|e| {
                    ArgoError::Provider(format!(
                        "connect MCP server '{server_id}' (command '{}'): {e}",
                        decl.command
                    ))
                })?
                .client_arc();
            guard.insert(server_id.to_string(), arc.clone());
            arc
        }
    };
    client.list_tools().await.map_err(|e| {
        ArgoError::Provider(format!(
            "list_tools on MCP server '{server_id}': {e}"
        ))
    })
}

/// Compile `--filter` eagerly so an invalid pattern surfaces before
/// we spawn a subprocess.
pub(super) fn compile_filter(pat: Option<&str>) -> ArgoResult<Option<Regex>> {
    match pat {
        Some(p) => Regex::new(p).map(Some).map_err(|e| {
            ArgoError::Provider(format!("invalid --filter regex '{p}': {e}"))
        }),
        None => Ok(None),
    }
}

/// Narrow tools by `--filter` regex over the tool name. `None`
/// returns `tools` untouched.
pub(super) fn apply_filter(
    tools: Vec<McpToolSchema>,
    regex: Option<&Regex>,
) -> Vec<McpToolSchema> {
    match regex {
        Some(re) => tools.into_iter().filter(|t| re.is_match(&t.name)).collect(),
        None => tools,
    }
}

/// One JSON object per line, written to stdout. `schema_only` drops
/// the description to keep the line terse for piping.
pub(super) fn render_ndjson(
    tools: &[McpToolSchema],
    schema_only: bool,
) -> ArgoResult<()> {
    for t in tools {
        let line = ndjson_line(t, schema_only)?;
        println!("{line}");
    }
    Ok(())
}

pub(super) fn ndjson_line(tool: &McpToolSchema, schema_only: bool) -> ArgoResult<String> {
    let mut obj = serde_json::Map::new();
    obj.insert("name".into(), serde_json::Value::String(tool.name.clone()));
    obj.insert("input_schema".into(), tool.input_schema.clone());
    if !schema_only {
        obj.insert(
            "description".into(),
            serde_json::Value::String(tool.description.clone()),
        );
    }
    serde_json::to_string(&serde_json::Value::Object(obj))
        .map_err(|e| ArgoError::Provider(format!("serialize tool '{}': {e}", tool.name)))
}

/// Per-tool markdown sections. Header carries the count + identifier
/// pair so an operator can grep the output back to the call.
pub(super) fn render_markdown(
    server_id: &str,
    vertical: &str,
    tools: &[McpToolSchema],
    schema_only: bool,
) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "# MCP tools — `{server_id}` (vertical `{vertical}`)\n\n"
    ));
    out.push_str(&format!(
        "{} tool{} discovered.\n\n",
        tools.len(),
        if tools.len() == 1 { "" } else { "s" }
    ));
    if tools.is_empty() {
        return out;
    }
    for t in tools {
        out.push_str(&format!("## `{}`\n\n", t.name));
        if !schema_only && !t.description.is_empty() {
            out.push_str(&t.description);
            if !t.description.ends_with('\n') {
                out.push('\n');
            }
            out.push('\n');
        }
        out.push_str("```json\n");
        let pretty = serde_json::to_string_pretty(&t.input_schema)
            .unwrap_or_else(|_| t.input_schema.to_string());
        out.push_str(&pretty);
        if !pretty.ends_with('\n') {
            out.push('\n');
        }
        out.push_str("```\n\n");
    }
    out
}

#[cfg(test)]
#[path = "mcp_tools_tests.rs"]
mod tests;
