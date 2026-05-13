//! Action-level Browser executor (mission-wire-browser-action-executor).
//!
//! Adapts the existing 16-tool stateful browser MCP server (the one
//! shipped under `mcp-servers/browser/`) into the per-action `Executor`
//! trait. Without this adapter every browser-driven action falls back
//! to [`super::worker_actions_executor::HumanExecutor`] and the mission
//! MVP is not autonomous.
//!
//! Dispatch is intentionally a thin wrapper. We:
//!   1. Parse `executor.binding` (an inline JSON object carrying
//!      `target_url`, `mode`, optional `instruction_template`,
//!      optional `steps`, and `session_scope`) — see
//!      [`super::worker_actions_executor_browser_binding`].
//!   2. Acquire (or open) a sticky MCP-stdio session keyed by
//!      `(tenant, worker)` so cookies persist across sequential
//!      actions for the same worker.
//!   3. Drive the session: `navigate` → either a scripted
//!      `click`/`fill`/`submit` sequence or a `cu_loop` tool call.
//!   4. Capture a final `screenshot` into the action's evidence
//!      bundle directory.
//!   5. Return [`ActionOutcome::Completed`] with the screenshot list.
//!
//! The Chrome/CDP machinery lives behind the browser MCP's tools;
//! this brief is an *adapter*, not a refactor.

use super::worker_actions::ActionRecord;
use super::worker_actions_executor::ActionOutcome;
use super::worker_actions_executor_browser_binding::{
    parse_binding, render_string, BrowserBinding, BrowserMode, BrowserStep, SessionScope,
};
use argo_mcp::{CallToolResult, McpClient, McpClientPool, McpContent, PoolConfig};
use argo_types::{ArgoError, ArgoResult, TenantId};
use argo_vertical::ActionDeclaration;
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tokio::sync::Mutex;
use uuid::Uuid;

/// Default per-action wallclock budget. Overridable via
/// `ARGO_BROWSER_ACTION_TIMEOUT_MS`.
const DEFAULT_BROWSER_ACTION_TIMEOUT_MS: u64 = 30_000;

const BROWSER_TIMEOUT_ENV: &str = "ARGO_BROWSER_ACTION_TIMEOUT_MS";

/// Path to the browser MCP server binary (or a stdio-MCP test
/// fixture). Required at dispatch time.
const BROWSER_MCP_BIN_ENV: &str = "ARGO_BROWSER_MCP_BIN";

/// Per-(tenant, worker) browser session pool key.
pub(crate) type SessionKey = (String, String);

/// Process-wide pool of live browser-MCP clients.
pub(crate) type BrowserPool = Mutex<HashMap<SessionKey, Arc<McpClient>>>;

/// Process-wide default pool. Tests construct local pools so
/// browser-MCP children don't leak between integration cases.
pub(crate) fn default_pool() -> &'static BrowserPool {
    static POOL: OnceLock<BrowserPool> = OnceLock::new();
    POOL.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Process-wide warm-pool of [`McpClient`] subprocesses keyed by
/// `(tenant_id, manifest_id)`. Drives the cold-spawn replacement at
/// [`acquire_client`] — `(tenant, manifest)` collision means a shared
/// subprocess, so the [`SessionKey`] is folded into the manifest id to
/// preserve the existing per-`(tenant, worker)` browser-session
/// isolation invariant (cookies/state never leak across workers).
fn shared_client_pool() -> &'static Arc<McpClientPool> {
    static POOL: OnceLock<Arc<McpClientPool>> = OnceLock::new();
    POOL.get_or_init(|| McpClientPool::new(PoolConfig::default()))
}

/// Default wallclock budget honouring `ARGO_BROWSER_ACTION_TIMEOUT_MS`.
pub(crate) fn default_timeout() -> Duration {
    std::env::var(BROWSER_TIMEOUT_ENV)
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|ms| *ms > 0)
        .map(Duration::from_millis)
        .unwrap_or(Duration::from_millis(DEFAULT_BROWSER_ACTION_TIMEOUT_MS))
}

/// Dispatch a single browser-executor action through the per-worker
/// MCP-stdio session pool. Returns
/// [`ActionOutcome::Completed`] with the screenshot path list embedded.
pub(crate) async fn dispatch_action(
    action: &ActionRecord,
    cwd: &Path,
    decl: &ActionDeclaration,
    pool: &BrowserPool,
    client_pool: &Arc<McpClientPool>,
    timeout: Duration,
) -> ArgoResult<ActionOutcome> {
    let binding = parse_binding(decl)?;
    let bundle_path = bundle_dir(cwd, action);
    std::fs::create_dir_all(&bundle_path).map_err(|e| {
        ArgoError::Provider(format!(
            "create browser bundle dir {}: {e}",
            bundle_path.display()
        ))
    })?;
    let key = session_key(action, binding.session_scope);
    let client = acquire_client(pool, client_pool, &key).await?;
    let outcome = tokio::time::timeout(
        timeout,
        run_action(&client, &binding, &bundle_path, action),
    )
    .await;
    if matches!(binding.session_scope, SessionScope::PerAction) {
        pool.lock().await.remove(&key);
    }
    let screenshots = match outcome {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => return Err(e),
        Err(_) => {
            return Err(ArgoError::Provider(format!(
                "browser action '{}' on '{}' exceeded wallclock budget {}ms",
                action.action_type,
                binding.target_url,
                timeout.as_millis()
            )));
        }
    };
    Ok(ActionOutcome::Completed {
        bundle_path,
        result_json: json!({
            "outcome": "completed",
            "executor": "browser",
            "mode": match binding.mode {
                BrowserMode::Cu => "cu",
                BrowserMode::Script => "script",
            },
            "target_url": binding.target_url,
            "screenshots": screenshots,
        }),
    })
}

/// Sync wrapper used by the `Executor::dispatch` shim in
/// [`super::worker_actions_executor_stubs`]. Resolves the current
/// tokio handle and blocks on the async dispatch path against the
/// process-wide default pool.
///
/// # Example
/// ```ignore
/// // dispatch_blocking(&action, Path::new("."), &decl)?;
/// ```
pub(crate) fn dispatch_blocking(
    action: &ActionRecord,
    cwd: &Path,
    decl: &ActionDeclaration,
) -> ArgoResult<ActionOutcome> {
    let _ = parse_binding(decl)?;
    let handle = tokio::runtime::Handle::try_current().map_err(|e| {
        ArgoError::Provider(format!(
            "browser executor needs a tokio runtime to dispatch: {e}"
        ))
    })?;
    let timeout = default_timeout();
    tokio::task::block_in_place(|| {
        handle.block_on(dispatch_action(
            action,
            cwd,
            decl,
            default_pool(),
            shared_client_pool(),
            timeout,
        ))
    })
}

fn bundle_dir(cwd: &Path, action: &ActionRecord) -> PathBuf {
    cwd.join(".skycot")
        .join("outputs")
        .join(&action.job_id)
        .join("actions")
        .join(&action.action_id)
}

/// Build a `(tenant, worker)` key for the session pool. Until Stream
/// E threads tenant identity through `ActionRecord` we use a stable
/// `default-tenant` sentinel matching
/// [`super::worker_actions_executor_stubs::ComputerUseExecutor`].
fn session_key(action: &ActionRecord, scope: SessionScope) -> SessionKey {
    let worker = action.worker_id.to_string();
    match scope {
        SessionScope::PerWorker => ("default-tenant".to_string(), worker),
        SessionScope::PerAction => (
            "default-tenant".to_string(),
            format!("{worker}::{}", action.action_id),
        ),
    }
}

async fn acquire_client(
    pool: &BrowserPool,
    client_pool: &Arc<McpClientPool>,
    key: &SessionKey,
) -> ArgoResult<Arc<McpClient>> {
    let mut guard = pool.lock().await;
    if let Some(client) = guard.get(key) {
        return Ok(client.clone());
    }
    let bin = std::env::var(BROWSER_MCP_BIN_ENV).map_err(|_| {
        ArgoError::Provider(format!(
            "browser executor needs the browser MCP server path; \
             set {BROWSER_MCP_BIN_ENV} to `argo-mcp-browser` (or a \
             stdio-MCP fixture) before dispatching"
        ))
    })?;
    let (program, args) = split_program(&bin);
    // mission-mcp-pool-wire-dispatch: route the cold spawn through the
    // injected [`McpClientPool`]. The local `BrowserPool` still tracks
    // per-`SessionKey` cookies, but the underlying subprocess lifecycle
    // (warm reuse across turns, LRU + idle-TTL eviction) is owned by
    // the injected pool. `manifest_id` folds the `SessionKey` in so
    // per-worker isolation is preserved at the global-pool layer.
    let manifest_id = format!("browser-mcp::{}::{}", key.0, key.1);
    let pool_program = program.clone();
    let pool_args = args.clone();
    let arc = client_pool
        .acquire(TenantId(Uuid::nil()), manifest_id, || async move {
            McpClient::connect_stdio(&pool_program, &pool_args).await
        })
        .await
        .map_err(|e| ArgoError::Provider(format!("connect browser MCP '{program}': {e}")))?
        .client_arc();
    guard.insert(key.clone(), arc.clone());
    Ok(arc)
}

/// Split the env-var spec into `(program, args)`. Honours simple
/// whitespace splitting so dev overrides like
/// `ARGO_BROWSER_MCP_BIN="python3 mock_browser.py"` work without
/// shell-escape gymnastics.
fn split_program(spec: &str) -> (String, Vec<String>) {
    let mut parts = spec.split_whitespace();
    let program = parts.next().unwrap_or("").to_string();
    let args: Vec<String> = parts.map(str::to_string).collect();
    (program, args)
}

/// Drive the browser MCP through one action's worth of work.
/// Captures at least one screenshot and writes each into the bundle.
async fn run_action(
    client: &McpClient,
    binding: &BrowserBinding,
    bundle_path: &Path,
    action: &ActionRecord,
) -> ArgoResult<Vec<String>> {
    let target = render_string(&binding.target_url, action);
    call_tool_or_provider(client, "navigate", json!({"url": target})).await?;
    match binding.mode {
        BrowserMode::Cu => run_cu_mode(client, binding, action).await?,
        BrowserMode::Script => run_script_mode(client, binding, action).await?,
    }
    let screenshots_dir = bundle_path.join("screenshots");
    std::fs::create_dir_all(&screenshots_dir).map_err(|e| {
        ArgoError::Provider(format!(
            "create screenshots dir {}: {e}",
            screenshots_dir.display()
        ))
    })?;
    let final_shot = capture_screenshot(client, &screenshots_dir, "final").await?;
    Ok(vec![final_shot])
}

/// CU mode: forward the rendered goal string to the browser MCP's
/// `cu_loop` tool. If the underlying server doesn't expose `cu_loop`
/// the JSON-RPC error surfaces as a clean Provider error.
async fn run_cu_mode(
    client: &McpClient,
    binding: &BrowserBinding,
    action: &ActionRecord,
) -> ArgoResult<()> {
    let template = binding.instruction_template.as_deref().unwrap_or("");
    let rendered = render_string(template, action);
    call_tool_or_provider(
        client,
        "cu_loop",
        json!({
            "goal": rendered,
            "target_url": render_string(&binding.target_url, action),
        }),
    )
    .await
    .map(|_| ())
}

/// Script mode: walk `binding.steps` and dispatch each to its tool.
/// `navigate` mid-sequence is supported (e.g. follow-up redirect).
async fn run_script_mode(
    client: &McpClient,
    binding: &BrowserBinding,
    action: &ActionRecord,
) -> ArgoResult<()> {
    for step in &binding.steps {
        let args = build_step_args(step, action);
        call_tool_or_provider(client, &step.action, args).await?;
    }
    Ok(())
}

fn build_step_args(step: &BrowserStep, action: &ActionRecord) -> Value {
    let mut map = Map::new();
    if let Some(sel) = step.selector.as_deref() {
        map.insert("selector".into(), Value::String(render_string(sel, action)));
    }
    if let Some(val) = step.value.as_deref() {
        let rendered = render_string(val, action);
        let key = if step.action == "navigate" { "url" } else { "value" };
        map.insert(key.into(), Value::String(rendered));
    }
    Value::Object(map)
}

async fn capture_screenshot(
    client: &McpClient,
    dir: &Path,
    label: &str,
) -> ArgoResult<String> {
    let result = call_tool_or_provider(client, "screenshot", json!({"full_page": false})).await?;
    let body = result
        .content
        .iter()
        .find_map(|c| match c {
            McpContent::Image { data, .. } => data.clone(),
            McpContent::Text { text } => Some(text.clone()),
            _ => None,
        })
        .unwrap_or_default();
    let path = dir.join(format!("{label}.b64"));
    std::fs::write(&path, body.as_bytes()).map_err(|e| {
        ArgoError::Provider(format!("write screenshot {}: {e}", path.display()))
    })?;
    Ok(path.to_string_lossy().into_owned())
}

async fn call_tool_or_provider(
    client: &McpClient,
    tool: &str,
    args: Value,
) -> ArgoResult<CallToolResult> {
    match client.call_tool(tool, args).await {
        Ok(result) if result.is_error => Err(ArgoError::Provider(format!(
            "browser MCP tool '{tool}' returned error: {}",
            error_body(&result.content)
        ))),
        Ok(result) => Ok(result),
        Err(e) => Err(ArgoError::Provider(format!(
            "browser MCP tool '{tool}' failed: {e}"
        ))),
    }
}

fn error_body(content: &[McpContent]) -> String {
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
        "(no error body)".into()
    } else {
        out
    }
}

#[cfg(test)]
#[path = "worker_actions_executor_browser_tests.rs"]
mod tests;
