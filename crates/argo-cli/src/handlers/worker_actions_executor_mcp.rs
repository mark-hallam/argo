//! Action-level MCP executor (mission-wire-mcp-action-executor).
//!
//! Replaces the C-mcp-wire-era stub in
//! [`worker_actions_executor_stubs::McpExecutor`] with a real impl
//! that resolves the action's manifest declaration → looks up the
//! `[mcp_servers]` entry → acquires a connection from a per-server
//! pool → calls `tools/call` → translates the response into an
//! `ActionOutcome`.
//!
//! The agent-level dispatch path in `argo-agent-runner::dispatch`
//! pools per-turn; the action-level executor is per-process. We
//! keep a `OnceLock` of `tokio::sync::Mutex<HashMap<server_name,
//! Arc<McpClient>>>` so subsequent dispatches against the same
//! server reuse the spawned subprocess.

use super::worker_actions::ActionRecord;
use super::worker_actions_executor::ActionOutcome;
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_mcp::{McpClient, McpContent};
use argo_store::queries::mcp_registry::McpRegistryStore;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use argo_vertical::{ActionDeclaration, McpServerDeclaration};
use async_trait::async_trait;
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tokio::sync::Mutex;

/// Default per-action wallclock budget. The MCP `McpClient` enforces
/// its own per-RPC timeout (30s) for line-by-line transport health;
/// this outer timeout caps the entire dispatch so a degenerate server
/// can't pin a worker forever. The brief calls for `[actions.X.budget]
/// .max_secs` enforcement; until that field exists in the manifest
/// schema, this constant is the floor.
const DEFAULT_ACTION_TIMEOUT: Duration = Duration::from_secs(30);

/// Per-server connection pool. Holds `Arc<McpClient>` so multiple
/// callers can dispatch concurrently against the same server's
/// already-spawned subprocess.
pub(crate) type McpPool = Mutex<HashMap<String, Arc<McpClient>>>;

/// Process-wide default pool — production callers go through this.
/// Tests construct local pools so subprocesses don't leak between
/// integration cases.
pub(crate) fn default_pool() -> &'static McpPool {
    static POOL: OnceLock<McpPool> = OnceLock::new();
    POOL.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Default per-action wallclock budget.
pub(crate) fn default_timeout() -> Duration {
    DEFAULT_ACTION_TIMEOUT
}

/// Dispatch a single MCP-executor action through the connector pool.
///
/// Resolves `executor.server` + `executor.tool` from `decl`, looks up
/// the transport config in the manifest's `[mcp_servers]` map,
/// substitutes any `{{ payload.<path> }}` placeholders in the action's
/// payload, calls the tool, and translates the response.
pub(crate) async fn dispatch_action(
    action: &ActionRecord,
    cwd: &Path,
    decl: &ActionDeclaration,
    servers: &HashMap<String, McpServerDeclaration>,
    pool: &McpPool,
    timeout: Duration,
) -> ArgoResult<ActionOutcome> {
    let server_name = decl.executor.mcp_server().ok_or_else(|| {
        ArgoError::Provider(
            "MCP executor missing `executor.server`; manifest must \
             declare `executor = { kind = \"mcp\", server = \"X\", \
             tool = \"Y\" }`"
                .into(),
        )
    })?;
    let tool_name = decl.executor.mcp_tool().ok_or_else(|| {
        ArgoError::Provider(
            "MCP executor missing `executor.tool`; manifest must \
             declare `executor = { kind = \"mcp\", server = \"X\", \
             tool = \"Y\" }`"
                .into(),
        )
    })?;
    let server_decl = EnvMcpRegistryLookup
        .lookup(server_name, cwd, servers)
        .await?;
    let args = substitute_args(&action.payload, action);
    let client = acquire_client(pool, server_name, &server_decl).await?;
    super::mcp_rate_limit::acquire_for_dispatch(server_name).await?;
    let result = match tokio::time::timeout(timeout, client.call_tool(tool_name, args)).await {
        Ok(r) => r,
        Err(_) => {
            return Err(ArgoError::Provider(format!(
                "MCP action '{}' on server '{}' tool '{}' exceeded \
                 wallclock budget {}s",
                action.action_type,
                server_name,
                tool_name,
                timeout.as_secs(),
            )));
        }
    };
    let bundle_path = cwd
        .join(".skycot")
        .join("outputs")
        .join(&action.job_id)
        .join("actions")
        .join(&action.action_id);
    match result {
        Ok(call) => {
            if call.is_error {
                let body = render_error_text(&call.content);
                Err(ArgoError::Provider(format!(
                    "MCP tool '{tool_name}' on server '{server_name}' \
                     returned error: {body}"
                )))
            } else {
                Ok(ActionOutcome::Completed {
                    bundle_path,
                    result_json: render_completion_json(server_name, tool_name, &call),
                })
            }
        }
        Err(e) => Err(ArgoError::Provider(format!(
            "MCP tool '{tool_name}' on server '{server_name}' failed: {e}"
        ))),
    }
}

/// Sync wrapper used by the `Executor::dispatch` shim in
/// [`super::worker_actions_executor_stubs`]. Resolves the current
/// tokio handle and blocks on the async dispatch path against the
/// process-wide default pool.
pub(crate) fn dispatch_blocking(
    action: &ActionRecord,
    cwd: &Path,
    decl: &ActionDeclaration,
    servers: &HashMap<String, McpServerDeclaration>,
) -> ArgoResult<ActionOutcome> {
    let handle = tokio::runtime::Handle::try_current().map_err(|e| {
        ArgoError::Provider(format!(
            "MCP executor needs a tokio runtime to dispatch: {e}"
        ))
    })?;
    let timeout = default_timeout();
    tokio::task::block_in_place(|| {
        handle.block_on(dispatch_action(
            action,
            cwd,
            decl,
            servers,
            default_pool(),
            timeout,
        ))
    })
}

/// Resolve an MCP server declaration for `server_name`. Two backings:
/// the vertical manifest's `[mcp_servers]` table (default) and the
/// per-tenant SQL registry (gated on `ARGO_MCP_REGISTRY_BACKED=1`).
/// Behind a trait so action-executor tests can stub a deterministic
/// lookup without touching env or disk.
#[async_trait]
trait McpRegistryLookup {
    async fn lookup(
        &self,
        server_name: &str,
        cwd: &Path,
        static_servers: &HashMap<String, McpServerDeclaration>,
    ) -> ArgoResult<McpServerDeclaration>;
}

struct EnvMcpRegistryLookup;

#[async_trait]
impl McpRegistryLookup for EnvMcpRegistryLookup {
    async fn lookup(
        &self,
        server_name: &str,
        cwd: &Path,
        static_servers: &HashMap<String, McpServerDeclaration>,
    ) -> ArgoResult<McpServerDeclaration> {
        if std::env::var("ARGO_MCP_REGISTRY_BACKED").ok().as_deref() == Some("1") {
            lookup_registry_server(cwd, server_name).await
        } else {
            lookup_static_server(server_name, static_servers)
        }
    }
}

fn lookup_static_server(
    server_name: &str,
    servers: &HashMap<String, McpServerDeclaration>,
) -> ArgoResult<McpServerDeclaration> {
    servers.get(server_name).cloned().ok_or_else(|| {
        ArgoError::Provider(format!(
            "MCP executor references server '{server_name}' but the \
             vertical manifest's `[mcp_servers]` table has no such \
             entry — declare it before dispatching this action"
        ))
    })
}

async fn lookup_registry_server(cwd: &Path, server_name: &str) -> ArgoResult<McpServerDeclaration> {
    let home = resolve_home(cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(db_path(&home)).await?;
    store.migrate().await?;
    let name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (tenant, _workspace) = store.ensure_workspace(name, &root).await?;
    let tenant_id = tenant.to_string();
    let row = McpRegistryStore::new(store.pool())
        .get(&tenant_id, server_name)
        .await?
        .ok_or_else(|| {
            ArgoError::Provider(format!(
                "MCP registry has no server '{server_name}' for tenant '{tenant_id}'"
            ))
        })?;
    Ok(McpServerDeclaration {
        transport: row.kind,
        command: row.base_url,
        args: Vec::new(),
    })
}

async fn acquire_client(
    pool: &McpPool,
    server_name: &str,
    server_decl: &McpServerDeclaration,
) -> ArgoResult<Arc<McpClient>> {
    if server_decl.transport != "stdio" {
        return Err(ArgoError::Provider(format!(
            "MCP server '{server_name}' uses transport '{}' but the \
             action-level executor only supports 'stdio' today",
            server_decl.transport
        )));
    }
    let mut guard = pool.lock().await;
    if let Some(client) = guard.get(server_name) {
        return Ok(client.clone());
    }
    let client = McpClient::connect_stdio(&server_decl.command, &server_decl.args)
        .await
        .map_err(|e| {
            ArgoError::Provider(format!(
                "connect MCP server '{server_name}' (command '{}'): {e}",
                server_decl.command
            ))
        })?;
    let arc = Arc::new(client);
    guard.insert(server_name.to_string(), arc.clone());
    Ok(arc)
}

/// Walk a JSON value, substituting `{{ key }}` placeholders inside
/// string scalars. Recognised keys mirror the human-executor
/// template engine: `action_id`, `action_type`, `job_id`, `risk_level`,
/// `platform`, `item_id`, and `payload.<dotted.path>`.
///
/// Non-string values pass through unchanged. Unknown keys render as
/// `(unset)` so operators see the gap rather than a literal placeholder.
pub(crate) fn substitute_args(payload: &Value, action: &ActionRecord) -> Value {
    match payload {
        Value::String(s) => Value::String(render_template(s, action)),
        Value::Array(items) => Value::Array(
            items
                .iter()
                .map(|v| substitute_args(v, action))
                .collect(),
        ),
        Value::Object(map) => {
            let mut out = Map::with_capacity(map.len());
            for (k, v) in map {
                out.insert(k.clone(), substitute_args(v, action));
            }
            Value::Object(out)
        }
        other => other.clone(),
    }
}

fn render_template(body: &str, action: &ActionRecord) -> String {
    let mut out = String::with_capacity(body.len() + 32);
    let mut cursor = 0;
    while cursor < body.len() {
        let Some(start) = body[cursor..].find("{{") else {
            out.push_str(&body[cursor..]);
            break;
        };
        out.push_str(&body[cursor..cursor + start]);
        let open = cursor + start;
        let Some(rel_end) = body[open + 2..].find("}}") else {
            out.push_str(&body[open..]);
            break;
        };
        let close = open + 2 + rel_end;
        let key = body[open + 2..close].trim();
        out.push_str(&lookup_template_key(action, key));
        cursor = close + 2;
    }
    out
}

fn lookup_template_key(action: &ActionRecord, key: &str) -> String {
    match key {
        "action_id" => action.action_id.clone(),
        "action_type" => action.action_type.clone(),
        "job_id" => action.job_id.clone(),
        "risk_level" => action.risk_level.clone(),
        "platform" => action.platform.clone().unwrap_or_else(|| "(unset)".into()),
        "item_id" => action.item_id.clone().unwrap_or_else(|| "(unset)".into()),
        other => {
            if let Some(rest) = other.strip_prefix("payload.") {
                return lookup_payload_field(&action.payload, rest);
            }
            "(unset)".to_string()
        }
    }
}

fn lookup_payload_field(payload: &Value, path: &str) -> String {
    let mut cur = payload;
    for seg in path.split('.') {
        match cur {
            Value::Object(map) => match map.get(seg) {
                Some(v) => cur = v,
                None => return "(unset)".to_string(),
            },
            _ => return "(unset)".to_string(),
        }
    }
    match cur {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => "(unset)".to_string(),
        other => other.to_string(),
    }
}

fn render_completion_json(
    server_name: &str,
    tool_name: &str,
    call: &argo_mcp::CallToolResult,
) -> Value {
    let content: Vec<Value> = call
        .content
        .iter()
        .map(|c| match c {
            McpContent::Text { text } => serde_json::json!({"type": "text", "text": text}),
            McpContent::Image { data, mime_type } => serde_json::json!({
                "type": "image",
                "mime_type": mime_type,
                "data": data,
            }),
            McpContent::Unknown => serde_json::json!({"type": "unknown"}),
        })
        .collect();
    serde_json::json!({
        "outcome": "completed",
        "executor": "mcp",
        "server": server_name,
        "tool": tool_name,
        "is_error": call.is_error,
        "content": content,
    })
}

fn render_error_text(content: &[McpContent]) -> String {
    let mut out = String::new();
    for c in content {
        if let McpContent::Text { text } = c {
            if !out.is_empty() {
                out.push('\n');
            }
            out.push_str(text);
        }
    }
    if out.is_empty() {
        out.push_str("(no error body)");
    }
    out
}

#[cfg(test)]
#[path = "worker_actions_executor_mcp_tests.rs"]
mod tests;
