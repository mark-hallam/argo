//! The in-core tool implementations (§9).
//!
//! Phase 3 ships four end-to-end (`file_read`, `file_write`,
//! `file_patch`, `terminal`) and four schema-only stubs that return a
//! clear "configure / not yet wired" error so the model still sees them
//! in its tool list:
//!
//! - `web_search`, `web_extract` — backend lives in an MCP server (§12).
//! - `memory` — backend is `argo-memory` (Agent H).
//! - `delegate_task` — backend is `argo-delegate` (Phase 5 / §16).
//!
//! Hermes-parity tools layered on the §9 base: `vision_analyze`
//! (provider multimodal) and `code_execute` (sandboxed python /
//! node / bash through `argo-sandbox`).
mod clarify;
mod code_execute;
mod delegate_task;
mod file_patch;
mod file_read;
mod file_write;
mod memory;
mod process;
mod send_message;
mod session_search;
mod terminal;
mod terminal_streaming;
mod todo;
mod vision;
mod web;

pub use clarify::{Clarify, ClarifyAnswer, ClarifyDispatcher};
pub use code_execute::CodeExecute;
pub use delegate_task::DelegateTask;
pub use file_patch::FilePatch;
pub use file_read::FileRead;
pub use file_write::FileWrite;
pub use memory::Memory;
pub use process::Process;
pub use send_message::{risk_tier, MessageDispatcher, SendMessage, SendOutcome};
pub use session_search::SessionSearch;
pub use terminal::Terminal;
pub use todo::{Todo, TodoItem};
pub use vision::{ResolvedImage, Vision, VisionAnswer, VisionDispatcher};
pub use web::{WebExtract, WebSearch};

use crate::tool::Tool;
use argo_identity::MemoryNs;
use argo_memory::MemoryStore;
use argo_orchestrator::OrchestratorStore;
use argo_store::SessionStore;
use argo_types::WorkspaceId;
use std::path::PathBuf;
use std::sync::Arc;

/// Construct the in-core tools as `Arc<dyn Tool>` ready for
/// [`crate::ToolRegistry::register`]. The `_workspace_root` argument
/// is reserved for future per-workspace defaults; the tools themselves
/// take the workspace from `ToolContext` at dispatch time so a single
/// registry serves multiple workspaces.
///
/// `Memory` is constructed with `Default::default()` here, which
/// returns the historical "memory store not wired" error at dispatch.
/// Agent-runner callers that want live runtime-backed dispatch use
/// [`builtin_tools_with_runtime`] instead.
#[must_use]
pub fn builtin_tools(_workspace_root: PathBuf) -> Vec<Arc<dyn Tool>> {
    vec![
        Arc::new(FileRead),
        Arc::new(FileWrite),
        Arc::new(FilePatch),
        Arc::new(Terminal::default()),
        Arc::new(WebSearch),
        Arc::new(WebExtract),
        Arc::new(Memory::default()),
        Arc::new(DelegateTask::default()),
        Arc::new(Vision::default()),
        Arc::new(CodeExecute::default()),
        Arc::new(SendMessage::default()),
        Arc::new(SessionSearch::default()),
        Arc::new(Todo),
        Arc::new(Clarify::default()),
        Arc::new(Process),
    ]
}

/// Same shape as [`builtin_tools`] but the `memory` tool is wired to
/// the supplied `MemoryStore` + `WorkspaceId`. `vertical_id`, when
/// `Some`, routes writes through `MemoryStore::append_for_vertical`
/// so vertical-scoped reads (`list_for_vertical`) see them. `None`
/// keeps writes workspace-global. Used by the agent runner once per
/// turn from `build_dispatch_ctx`.
#[must_use]
pub fn builtin_tools_with_memory(
    _workspace_root: PathBuf,
    store: Arc<MemoryStore>,
    workspace_id: WorkspaceId,
    vertical_id: Option<String>,
) -> Vec<Arc<dyn Tool>> {
    vec![
        Arc::new(FileRead),
        Arc::new(FileWrite),
        Arc::new(FilePatch),
        Arc::new(Terminal::default()),
        Arc::new(WebSearch),
        Arc::new(WebExtract),
        Arc::new(Memory::new(store, workspace_id, vertical_id)),
        Arc::new(DelegateTask::default()),
        Arc::new(Vision::default()),
        Arc::new(CodeExecute::default()),
        Arc::new(SendMessage::default()),
        Arc::new(SessionSearch::default()),
        Arc::new(Todo),
        Arc::new(Clarify::default()),
        Arc::new(Process),
    ]
}

/// Same shape as [`builtin_tools`] but wires the runtime-backed tools
/// that need stores supplied by the agent runner.
///
/// `memory`, when `Some`, enables live memory dispatch through
/// [`MemoryStore`]. `memory_namespace`, when `Some`, binds the
/// `memory` tool to a worker [`MemoryNs`] via [`Memory::with_namespace`]
/// so the `query_scored` / `*_team` actions can reach the
/// tenant-scoped namespace store; when `None` the tool falls back to
/// [`Memory::new`] and the namespace-bound actions return a typed
/// error (`Memory::new` is retained for legacy callers that haven't
/// plumbed a worker id yet). `delegate_task` is always wired to the
/// supplied [`OrchestratorStore`], and `session_search` is always
/// wired to the supplied [`SessionStore`]. `send_message` intentionally
/// remains on its default dispatcher until a production channel
/// dispatcher exists.
///
/// ```ignore
/// let tools = builtin_tools_with_runtime(
///     cwd,
///     memory,
///     memory_namespace,
///     orchestrator,
///     session_store,
/// );
/// ```
#[must_use]
pub fn builtin_tools_with_runtime(
    _workspace_root: PathBuf,
    memory: Option<(Arc<MemoryStore>, WorkspaceId, Option<String>)>,
    memory_namespace: Option<MemoryNs>,
    orchestrator: Arc<OrchestratorStore>,
    session_store: Arc<SessionStore>,
) -> Vec<Arc<dyn Tool>> {
    let memory_tool: Arc<dyn Tool> = match memory {
        Some((store, workspace_id, vertical_id)) => match memory_namespace {
            Some(ns) => Arc::new(Memory::with_namespace(store, workspace_id, vertical_id, ns)),
            None => Arc::new(Memory::new(store, workspace_id, vertical_id)),
        },
        None => Arc::new(Memory::default()),
    };
    vec![
        Arc::new(FileRead),
        Arc::new(FileWrite),
        Arc::new(FilePatch),
        Arc::new(Terminal::default()),
        Arc::new(WebSearch),
        Arc::new(WebExtract),
        memory_tool,
        Arc::new(DelegateTask::new(orchestrator)),
        Arc::new(Vision::default()),
        Arc::new(CodeExecute::default()),
        Arc::new(SendMessage::default()),
        Arc::new(SessionSearch::new(session_store)),
        Arc::new(Todo),
        Arc::new(Clarify::default()),
        Arc::new(Process),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_tools_returns_unique_names() {
        let tools = builtin_tools(PathBuf::from("/tmp"));
        assert_eq!(tools.len(), 15);
        let mut names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
        names.sort_unstable();
        let mut deduped = names.clone();
        deduped.dedup();
        assert_eq!(names, deduped);
        assert_eq!(
            names,
            vec![
                "clarify",
                "code_execute",
                "delegate_task",
                "file_patch",
                "file_read",
                "file_write",
                "memory",
                "process",
                "send_message",
                "session_search",
                "terminal",
                "todo",
                "vision_analyze",
                "web_extract",
                "web_search",
            ]
        );
    }
}
