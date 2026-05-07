//! Stub + real `Executor` impls for the executor families that
//! aren't `Human`. Stream A (#A-executor-trait) keeps
//! `worker_actions_executor.rs` under the per-file 500-LOC
//! budget by holding only the trait + `HumanExecutor` there;
//! sibling impls live here.
//!
//! - `McpExecutor` → real impl as of
//!   `mission-wire-mcp-action-executor`; delegates to the per-server
//!   pool in `worker_actions_executor_mcp`.
//! - `ComputerUseExecutor` → real impl, delegates to
//!   `argo-executor-computer-use` (Stream B). Falls back to a
//!   clear error when `E2B_API_KEY` / `ANTHROPIC_API_KEY`
//!   aren't set so unconfigured workspaces surface the gap
//!   rather than crash.
//! - `ApiExecutor` → still a stub; wired by
//!   `mission-wire-api-action-executor`.

use super::worker_actions::ActionRecord;
use super::worker_actions_executor::{ActionOutcome, Executor, ExecutorKind};
use super::worker_actions_executor_api;
use super::worker_actions_executor_mcp;
use crate::home::{db_path, resolve_home};
use argo_executor_computer_use::{
    ComputerUseExecutor as CuExecutorImpl, CuConfig, Outcome, ProfileRegistry, RunRequest,
};
use argo_executor_filesystem::{ops as fs_ops, FsOpKind, Sandbox};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use argo_vertical::{ActionDeclaration, McpServerDeclaration};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Real action-level MCP executor wired by
/// `mission-wire-mcp-action-executor`. Carries the action's manifest
/// declaration plus the manifest-level `[mcp_servers]` map so dispatch
/// can resolve `executor.server` → transport without re-loading the
/// vertical at call time.
pub(crate) struct McpExecutor {
    declaration: Option<ActionDeclaration>,
    servers: HashMap<String, McpServerDeclaration>,
}

impl McpExecutor {
    /// Production constructor used by `resolve_executor`. Carries the
    /// per-action declaration + the manifest's `[mcp_servers]` table
    /// so the dispatch path is self-contained.
    pub(crate) fn with_manifest(
        decl: ActionDeclaration,
        servers: HashMap<String, McpServerDeclaration>,
    ) -> Self {
        Self {
            declaration: Some(decl),
            servers,
        }
    }
}

impl Executor for McpExecutor {
    fn kind(&self) -> ExecutorKind {
        ExecutorKind::Mcp
    }
    fn dispatch(&self, action: &ActionRecord, cwd: &Path) -> ArgoResult<ActionOutcome> {
        let decl = self.declaration.as_ref().ok_or_else(|| {
            ArgoError::Provider(
                "McpExecutor needs an ActionDeclaration; resolve_executor \
                 must build it via `with_manifest`"
                    .into(),
            )
        })?;
        worker_actions_executor_mcp::dispatch_blocking(action, cwd, decl, &self.servers)
    }
}

/// Real computer-use executor wired by Stream B
/// `#B-computer-use-executor`. Carries the action's manifest
/// declaration so dispatch can read `profile`, `budget_usd`, and
/// the per-action description into the system prompt.
pub(crate) struct ComputerUseExecutor {
    declaration: Option<ActionDeclaration>,
}

impl ComputerUseExecutor {
    /// Constructor used by `resolve_executor` in Stream A —
    /// passes through the action's manifest declaration so we
    /// can read profile + budget + description.
    pub(crate) fn with_declaration(decl: ActionDeclaration) -> Self {
        Self {
            declaration: Some(decl),
        }
    }
}

impl Executor for ComputerUseExecutor {
    fn kind(&self) -> ExecutorKind {
        ExecutorKind::ComputerUse
    }

    fn dispatch(&self, action: &ActionRecord, cwd: &Path) -> ArgoResult<ActionOutcome> {
        // Build a CuConfig + ComputerUseExecutor from env. If
        // the keys aren't set, the operator sees a clear error
        // pointing at the env vars rather than a generic
        // network failure later.
        let config = CuConfig::from_env().map_err(|e| {
            ArgoError::Provider(format!(
                "computer-use executor not configured: {e}; set E2B_API_KEY \
                 and ANTHROPIC_API_KEY in your environment to enable this kind"
            ))
        })?;
        // Profile registry: prefer the workspace's
        // `manifests/executors/computer_use.toml`; fall back to
        // the built-in `marketplace_browser` profile.
        let mut registry = ProfileRegistry::load_from_path(&config.profiles_path)
            .map_err(|e| ArgoError::Provider(e.to_string()))?;
        if registry.is_empty() {
            registry = ProfileRegistry::with_builtin_marketplace_browser();
        }
        let cu = CuExecutorImpl::new(config, registry);

        let decl = self.declaration.as_ref().ok_or_else(|| {
            ArgoError::Provider(
                "ComputerUseExecutor needs an ActionDeclaration; \
                 dispatch was called from a path that didn't carry one"
                    .into(),
            )
        })?;
        let profile_name = decl
            .executor
            .profile()
            .unwrap_or("marketplace_browser")
            .to_string();
        let budget_usd = decl.executor.budget_usd();
        let bundle_dir = cwd
            .join(".skycot")
            .join("outputs")
            .join(&action.job_id)
            .join("actions")
            .join(&action.action_id);

        let req = RunRequest {
            action_id: action.action_id.clone(),
            job_id: action.job_id.clone(),
            // TODO(stream-E-thread-through): tenant + persona
            // come from the worker context once Stream E's
            // identity primitives are threaded through
            // ActionRecord. Until then we use stable sentinels
            // so the pool key is deterministic per action_type.
            tenant_id: "default-tenant".into(),
            persona_id: "default-persona".into(),
            profile_name,
            system_prompt: build_system_prompt(action, decl),
            budget_usd,
            bundle_dir: Some(bundle_dir),
            // session_id plumbed through by the worker daemon
            // once Stream H-orchestrator-persist threads identity
            // through dispatch; today's cli surface doesn't carry
            // a session id, so the skill-learning loop is a no-op
            // here. The cu executor still runs cleanly; only the
            // optional propose call is skipped.
            session_id: None,
        };

        let outcome = run_blocking(cu, req)?;
        Ok(map_outcome(outcome))
    }
}

/// Bridge sync `Executor::dispatch` to the async `cu.run`. We're
/// always inside a tokio runtime here (the cli is async); we use
/// `block_in_place` to allow the loop to run without panicking.
fn run_blocking(cu: CuExecutorImpl, req: RunRequest) -> ArgoResult<Outcome> {
    let handle = tokio::runtime::Handle::try_current().map_err(|e| {
        ArgoError::Provider(format!(
            "computer-use executor needs a tokio runtime to run; got: {e}"
        ))
    })?;
    let outcome = tokio::task::block_in_place(|| handle.block_on(cu.run(req)));
    outcome.map_err(Into::into)
}

fn map_outcome(o: Outcome) -> ActionOutcome {
    match o {
        Outcome::Completed {
            result,
            bundle_dir,
            budget,
        } => ActionOutcome::Completed {
            bundle_path: bundle_dir,
            result_json: serde_json::json!({
                "outcome": "completed",
                "final_text": result.final_text,
                "actions_taken": result.actions_taken,
                "tokens_used": result.tokens_used,
                "budget": budget,
            }),
        },
        Outcome::HumanHandoff {
            reason,
            bundle_dir,
            budget,
        } => {
            // Print the handoff reason so the operator sees why
            // the loop bailed without having to crack open the
            // bundle's browser.jsonl.
            eprintln!(
                "argo worker: WARN — computer-use loop handed off to human: {reason} \
                 (budget: {} steps, {} tokens, {}s)",
                budget.steps_used, budget.tokens_used, budget.wallclock_seconds_used
            );
            ActionOutcome::HumanHandoff {
                bundle_path: bundle_dir,
            }
        }
    }
}

/// Synthesise the operator-facing system prompt the CU loop
/// uses. Pulls the action's description, payload, and the
/// declared profile's URL whitelist into a single block.
fn build_system_prompt(action: &ActionRecord, decl: &ActionDeclaration) -> String {
    let payload_pretty =
        serde_json::to_string_pretty(&action.payload).unwrap_or_else(|_| "{}".to_string());
    let platform = action.platform.as_deref().unwrap_or("(unspecified)");
    format!(
        "You are operating a sandboxed browser to carry out one Skycot action.\n\n\
         Action: `{action_type}` on `{platform}` (item `{item_id}`)\n\
         Description: {description}\n\n\
         Payload (JSON):\n{payload_pretty}\n\n\
         Use the `computer` tool to navigate, click, type, and scroll. When the action \
         is complete, emit `stop_success` with the final URL or confirmation text. \
         If you can't complete the action (login wall, captcha, missing field), emit \
         `stop_failure` with a one-line reason.",
        action_type = action.action_type,
        platform = platform,
        item_id = action.item_id.as_deref().unwrap_or("(none)"),
        description = if decl.description.is_empty() {
            "(no description in manifest)"
        } else {
            decl.description.as_str()
        },
    )
}

/// Real filesystem executor wired by `claude-filesystem-executor`
/// (V1.2). Dispatches the action's payload as one of the six FS ops
/// (`read_file` / `write_file` / `list_dir` / `mkdir` / `remove` /
/// `stat`) inside the per-worker sandbox at `worker.fs_root` (default
/// `/tmp/argo-fs/<worker_id>/`).
///
/// The fs_root is resolved from the workers table on dispatch; when
/// the column is NULL or the worker row is missing, we fall back to
/// the deterministic default. That keeps the executor usable from
/// test rigs that bypass `argo worker create`.
pub(crate) struct FilesystemExecutor;

impl FilesystemExecutor {
    pub(crate) fn new() -> Self {
        Self
    }
}

impl Executor for FilesystemExecutor {
    fn kind(&self) -> ExecutorKind {
        ExecutorKind::Filesystem
    }

    fn dispatch(&self, action: &ActionRecord, cwd: &Path) -> ArgoResult<ActionOutcome> {
        let payload = &action.payload;
        let op_str = payload
            .get("op")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                ArgoError::Provider(
                    "filesystem executor requires action.payload.op \
                     ∈ {read_file, write_file, list_dir, mkdir, remove, stat}"
                        .into(),
                )
            })?;
        let path_str = payload.get("path").and_then(|v| v.as_str()).unwrap_or(".");
        let op = parse_fs_op(op_str)?;
        let fs_root = resolve_fs_root(cwd, &action.worker_id.to_string())?;
        let sb = Sandbox::open(&fs_root)
            .map_err(|e| ArgoError::Provider(format!("filesystem sandbox: {e}")))?;
        let result = run_fs_op(&sb, op, path_str, payload)
            .map_err(|e| ArgoError::Provider(format!("filesystem op {op_str}: {e}")))?;
        let bundle_path = cwd
            .join(".skycot")
            .join("outputs")
            .join(&action.job_id)
            .join("actions")
            .join(&action.action_id);
        Ok(ActionOutcome::Completed {
            bundle_path,
            result_json: serde_json::json!({
                "outcome": "completed",
                "executor": "filesystem",
                "fs_root": fs_root.display().to_string(),
                "op": op_str,
                "result": result,
            }),
        })
    }
}

fn parse_fs_op(s: &str) -> ArgoResult<FsOpKind> {
    match s {
        "read_file" => Ok(FsOpKind::ReadFile),
        "write_file" => Ok(FsOpKind::WriteFile),
        "list_dir" => Ok(FsOpKind::ListDir),
        "mkdir" => Ok(FsOpKind::Mkdir),
        "remove" => Ok(FsOpKind::Remove),
        "stat" => Ok(FsOpKind::Stat),
        other => Err(ArgoError::Provider(format!(
            "unknown filesystem op '{other}' — \
             valid: read_file | write_file | list_dir | mkdir | remove | stat"
        ))),
    }
}

fn run_fs_op(
    sb: &Sandbox,
    op: FsOpKind,
    rel: &str,
    payload: &serde_json::Value,
) -> Result<serde_json::Value, argo_executor_filesystem::FsExecutorError> {
    match op {
        FsOpKind::ReadFile => {
            let bytes = fs_ops::read_file(sb, rel)?;
            Ok(serde_json::json!({
                "path": rel,
                "size": bytes.len(),
                "body_utf8": String::from_utf8_lossy(&bytes),
            }))
        }
        FsOpKind::WriteFile => {
            let body = payload
                .get("body_utf8")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let abs = fs_ops::write_file(sb, rel, body.as_bytes())?;
            Ok(serde_json::json!({
                "path": rel,
                "abs_path": abs.display().to_string(),
                "size": body.len(),
            }))
        }
        FsOpKind::ListDir => {
            let entries = fs_ops::list_dir(sb, rel)?;
            Ok(serde_json::json!({"path": rel, "entries": entries}))
        }
        FsOpKind::Mkdir => {
            let abs = fs_ops::mkdir(sb, rel)?;
            Ok(serde_json::json!({"path": rel, "abs_path": abs.display().to_string()}))
        }
        FsOpKind::Remove => {
            fs_ops::remove(sb, rel)?;
            Ok(serde_json::json!({"path": rel}))
        }
        FsOpKind::Stat => {
            let meta = fs_ops::stat(sb, rel)?;
            Ok(serde_json::json!({"path": rel, "meta": meta}))
        }
    }
}

/// Look up `worker.fs_root` from the workspace SessionStore. Falls
/// back to the deterministic `/tmp/argo-fs/<worker_id>/` default when
/// the column is NULL or the worker row isn't found — both legitimate
/// in test rigs that haven't run `argo worker create`.
fn resolve_fs_root(cwd: &Path, worker_id: &str) -> ArgoResult<PathBuf> {
    let default = PathBuf::from("/tmp/argo-fs").join(worker_id);
    let home = match resolve_home(cwd) {
        Ok(h) => h,
        Err(_) => return Ok(default),
    };
    let db = db_path(&home);
    if !db.exists() {
        return Ok(default);
    }
    let handle = tokio::runtime::Handle::try_current().map_err(|e| {
        ArgoError::Provider(format!(
            "filesystem executor needs a tokio runtime to look up fs_root: {e}"
        ))
    })?;
    let lookup = tokio::task::block_in_place(|| {
        handle.block_on(async {
            let store = SessionStore::open(&db).await?;
            let row: Option<Option<String>> = sqlx::query_scalar(
                "SELECT fs_root FROM worker WHERE id = ?",
            )
            .bind(worker_id)
            .fetch_optional(store.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("lookup worker.fs_root: {e}")))?;
            Ok::<Option<String>, ArgoError>(row.flatten())
        })
    });
    match lookup {
        Ok(Some(p)) => Ok(PathBuf::from(p)),
        Ok(None) => Ok(default),
        // Don't blow up dispatch on a transient db error — fall back
        // to the deterministic root so the operator gets a meaningful
        // sandbox-violation error rather than a generic "couldn't
        // open db" message.
        Err(_) => Ok(default),
    }
}

/// Real action-level API executor wired by
/// `mission-wire-api-action-executor`. Carries the action's
/// manifest declaration so dispatch can read `executor.binding`.
pub(crate) struct ApiExecutor {
    declaration: Option<ActionDeclaration>,
}

impl ApiExecutor {
    /// Production constructor used by `resolve_executor`.
    pub(crate) fn with_manifest(decl: ActionDeclaration) -> Self {
        Self {
            declaration: Some(decl),
        }
    }
}

impl Executor for ApiExecutor {
    fn kind(&self) -> ExecutorKind {
        ExecutorKind::Api
    }
    fn dispatch(&self, action: &ActionRecord, cwd: &Path) -> ArgoResult<ActionOutcome> {
        let decl = self.declaration.as_ref().ok_or_else(|| {
            ArgoError::Provider(
                "ApiExecutor needs an ActionDeclaration; resolve_executor \
                 must build it via `with_manifest`"
                    .into(),
            )
        })?;
        worker_actions_executor_api::dispatch_blocking(action, cwd, decl)
    }
}
