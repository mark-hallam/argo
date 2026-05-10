//! Real `FilesystemExecutor` impl + dispatch helpers, sibling to
//! `worker_actions_executor_stubs.rs`. Holds the executor's own
//! sandbox / fs_root / audit-emit machinery so the parent file
//! stays under the per-file 500-LOC budget.
//!
//! Wired by `claude-filesystem-executor` (V1.2). Dispatches the
//! action's payload as one of the six FS ops (`read_file` /
//! `write_file` / `list_dir` / `mkdir` / `remove` / `stat`) inside
//! the per-worker sandbox at `worker.fs_root` (default
//! `/tmp/argo-fs/<worker_id>/`).
//!
//! The fs_root is resolved from the workers table on dispatch; when
//! the column is NULL or the worker row is missing, we fall back to
//! the deterministic default. That keeps the executor usable from
//! test rigs that bypass `argo worker create`.

use super::worker_actions::ActionRecord;
use super::worker_actions_executor::{ActionOutcome, Executor, ExecutorKind};
use crate::home::{db_path, resolve_home};
use argo_executor_filesystem::{ops as fs_ops, FsOpKind, Sandbox as FsSandbox};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use std::path::{Path, PathBuf};

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
            .unwrap_or("(missing)");
        let path_str = payload.get("path").and_then(|v| v.as_str()).unwrap_or(".");
        let result = run_fs_dispatch(action, cwd, op_str, path_str);
        match &result {
            Ok(_) => emit_fs_audit(&action.worker_id, op_str, path_str, "allowed", None),
            Err(e) => emit_fs_audit(
                &action.worker_id,
                op_str,
                path_str,
                "denied",
                Some(&e.to_string()),
            ),
        }
        result
    }
}

fn run_fs_dispatch(
    action: &ActionRecord,
    cwd: &Path,
    op_str: &str,
    path_str: &str,
) -> ArgoResult<ActionOutcome> {
    if op_str == "(missing)" {
        return Err(ArgoError::Provider(
            "filesystem executor requires action.payload.op \
             ∈ {read_file, write_file, list_dir, mkdir, remove, stat}"
                .into(),
        ));
    }
    let payload = &action.payload;
    let op = parse_fs_op(op_str)?;
    let fs_root = resolve_fs_root(cwd, &action.worker_id.to_string())?;
    let sb = FsSandbox::open(&fs_root)
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

/// Emit one structured audit event per FS-executor dispatch under
/// target `argo::fs::audit`. Tenant is hard-coded to `default-tenant`
/// pending Stream E identity plumbing on `ActionRecord`; a follow-up
/// audit-store brief will ingest these events.
fn emit_fs_audit(
    worker_id: &argo_identity::WorkerId,
    op: &str,
    path: &str,
    decision: &str,
    reason: Option<&str>,
) {
    tracing::info!(
        target: "argo::fs::audit",
        tenant = "default-tenant",
        worker = %worker_id,
        op = op,
        path = path,
        decision = decision,
        reason = reason.unwrap_or(""),
        "fs executor audit"
    );
}

#[cfg(test)]
#[path = "worker_actions_executor_stubs_audit_tests.rs"]
mod audit_tests;

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
    sb: &FsSandbox,
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
