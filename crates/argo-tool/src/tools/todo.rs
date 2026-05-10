//! `todo` — per-workspace task list the agent maintains across a
//! multi-step run (Hermes parity, brief `tool-todo-and-clarify`).
//!
//! Capability gate: declared via existing
//! [`argo_types::Capability::ToolDispatch`] with
//! `tool_name_glob = "todo"` plus narrow `FsRead`/`FsWrite` on the
//! state file — no new capability variant.
//!
//! ## State
//!
//! State persists as JSON at `<workspace_root>/.argo/todo.json`.
//! Items: `{ id: u64, text: string, done: bool, created_at: iso8601 }`.
//!
//! The brief proposed a per-session JSON path, but
//! [`crate::ToolContext`] is part of `argo-tool/src/tool.rs` (outside
//! this brief's may-touch list) and does not carry a session id today,
//! so we scope per-workspace instead. Workspace scoping is also more
//! useful operationally — long-running goals span sessions.
//!
//! ## Example
//!
//! ```
//! use argo_tool::Todo;
//! // The tool's schema is always present; dispatch is pure file IO
//! // against `<workspace_root>/.argo/todo.json`, no wiring required.
//! let _t = Todo;
//! ```
use crate::tool::{error_text, success_text, Tool, ToolContext};
use argo_types::{ArgoError, ArgoResult, Capability, CapabilitySet, ToolCallResult};
use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Instant;

/// `todo` tool — schema-only struct; all state lives on disk under
/// the workspace root.
#[derive(Debug, Default)]
pub struct Todo;

/// One entry in the todo list. Wire-format for both the on-disk JSON
/// file and the tool's success payloads, so the model can parse the
/// same shape it sees back.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TodoItem {
    /// Stable id, monotonic within a workspace.
    pub id: u64,
    /// Task text as supplied to `add`.
    pub text: String,
    /// `true` once `complete` has flipped it.
    pub done: bool,
    /// RFC 3339 / ISO 8601 timestamp of `add`.
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
struct Args {
    action: String,
    #[serde(default)]
    item: Option<String>,
    #[serde(default)]
    id: Option<u64>,
}

#[async_trait]
impl Tool for Todo {
    fn name(&self) -> &str {
        "todo"
    }

    fn description(&self) -> &str {
        "Maintain a per-workspace task list across a multi-step run. \
         `action` is one of `add` (requires `item`), `list`, `complete` \
         (requires `id`), or `remove` (requires `id`). Returns the \
         affected item (or full list for `list`) as JSON."
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "required": ["action"],
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["add", "list", "complete", "remove"],
                },
                "item": {
                    "type": "string",
                    "description": "Task text (required for `add`).",
                },
                "id": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Task id (required for `complete` and `remove`).",
                },
            },
            "additionalProperties": false,
        })
    }

    fn required_capabilities(&self, args: &serde_json::Value) -> ArgoResult<CapabilitySet> {
        let parsed = parse_args(args)?;
        let mut caps = vec![Capability::ToolDispatch {
            tool_name_glob: "todo".into(),
        }];
        match parsed.action.as_str() {
            "list" => caps.push(Capability::FsRead {
                path_glob: ".argo/todo.json".into(),
            }),
            "add" | "complete" | "remove" => caps.push(Capability::FsWrite {
                path_glob: ".argo/todo.json".into(),
            }),
            _ => {}
        }
        Ok(CapabilitySet(caps))
    }

    async fn dispatch(
        &self,
        args: serde_json::Value,
        ctx: &ToolContext,
    ) -> ArgoResult<ToolCallResult> {
        let started = Instant::now();
        let parsed = parse_args(&args)?;
        let path = todo_path(&ctx.workspace_root);
        let outcome = match parsed.action.as_str() {
            "add" => add_action(&path, parsed.item).await,
            "list" => list_action(&path).await,
            "complete" => complete_action(&path, parsed.id).await,
            "remove" => remove_action(&path, parsed.id).await,
            other => Err(format!(
                "invalid action '{other}' (expected add / list / complete / remove)"
            )),
        };
        let elapsed = started.elapsed().as_millis() as u64;
        match outcome {
            Ok(text) => Ok(success_text(ctx.call_id, text, elapsed)),
            Err(msg) => Ok(error_text(ctx.call_id, format!("todo: {msg}"), elapsed)),
        }
    }
}

fn parse_args(args: &serde_json::Value) -> ArgoResult<Args> {
    serde_json::from_value(args.clone()).map_err(|e| ArgoError::Tool {
        tool: "todo".into(),
        msg: format!("invalid arguments: {e}"),
    })
}

pub(crate) fn todo_path(workspace_root: &Path) -> PathBuf {
    workspace_root.join(".argo").join("todo.json")
}

async fn read_items(path: &Path) -> Result<Vec<TodoItem>, String> {
    match tokio::fs::read_to_string(path).await {
        Ok(s) if s.trim().is_empty() => Ok(Vec::new()),
        Ok(s) => serde_json::from_str(&s).map_err(|e| format!("corrupt todo file: {e}")),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(format!("read {}: {e}", path.display())),
    }
}

async fn write_items(path: &Path, items: &[TodoItem]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| format!("create dir {}: {e}", parent.display()))?;
    }
    let body = serde_json::to_string_pretty(items).map_err(|e| format!("serialize: {e}"))?;
    tokio::fs::write(path, body)
        .await
        .map_err(|e| format!("write {}: {e}", path.display()))
}

async fn add_action(path: &Path, item: Option<String>) -> Result<String, String> {
    let text = item.ok_or_else(|| "missing required `item` for add".to_string())?;
    if text.trim().is_empty() {
        return Err("`item` must be non-empty".into());
    }
    let mut items = read_items(path).await?;
    let next_id = items.iter().map(|i| i.id).max().unwrap_or(0) + 1;
    let new = TodoItem {
        id: next_id,
        text,
        done: false,
        created_at: Utc::now().to_rfc3339(),
    };
    items.push(new.clone());
    write_items(path, &items).await?;
    serde_json::to_string(&new).map_err(|e| format!("serialize new: {e}"))
}

async fn list_action(path: &Path) -> Result<String, String> {
    let items = read_items(path).await?;
    serde_json::to_string(&items).map_err(|e| format!("serialize: {e}"))
}

async fn complete_action(path: &Path, id: Option<u64>) -> Result<String, String> {
    let id = id.ok_or_else(|| "missing required `id` for complete".to_string())?;
    let mut items = read_items(path).await?;
    let target = items
        .iter_mut()
        .find(|i| i.id == id)
        .ok_or_else(|| format!("no todo with id {id}"))?;
    target.done = true;
    let snapshot = target.clone();
    write_items(path, &items).await?;
    serde_json::to_string(&snapshot).map_err(|e| format!("serialize snapshot: {e}"))
}

async fn remove_action(path: &Path, id: Option<u64>) -> Result<String, String> {
    let id = id.ok_or_else(|| "missing required `id` for remove".to_string())?;
    let mut items = read_items(path).await?;
    let before = items.len();
    items.retain(|i| i.id != id);
    if items.len() == before {
        return Err(format!("no todo with id {id}"));
    }
    write_items(path, &items).await?;
    Ok(format!("removed todo {id}"))
}

#[cfg(test)]
#[path = "todo_tests.rs"]
mod tests;
