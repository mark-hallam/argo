//! The eight in-core tool implementations (§9).
//!
//! Phase 3 ships four end-to-end (`file_read`, `file_write`,
//! `file_patch`, `terminal`) and four schema-only stubs that return a
//! clear "configure / not yet wired" error so the model still sees them
//! in its tool list:
//!
//! - `web_search`, `web_extract` — backend lives in an MCP server (§12).
//! - `memory` — backend is `argo-memory` (Agent H).
//! - `delegate_task` — backend is `argo-delegate` (Phase 5 / §16).
mod delegate_task;
mod file_patch;
mod file_read;
mod file_write;
mod memory;
mod terminal;
mod web;

pub use delegate_task::DelegateTask;
pub use file_patch::FilePatch;
pub use file_read::FileRead;
pub use file_write::FileWrite;
pub use memory::Memory;
pub use terminal::Terminal;
pub use web::{WebExtract, WebSearch};

use crate::tool::Tool;
use std::path::PathBuf;
use std::sync::Arc;

/// Construct the eight in-core tools as `Arc<dyn Tool>` ready for
/// [`crate::ToolRegistry::register`]. The `_workspace_root` argument
/// is reserved for future per-workspace defaults; the tools themselves
/// take the workspace from `ToolContext` at dispatch time so a single
/// registry serves multiple workspaces.
#[must_use]
pub fn builtin_tools(_workspace_root: PathBuf) -> Vec<Arc<dyn Tool>> {
    vec![
        Arc::new(FileRead),
        Arc::new(FileWrite),
        Arc::new(FilePatch),
        Arc::new(Terminal),
        Arc::new(WebSearch),
        Arc::new(WebExtract),
        Arc::new(Memory),
        Arc::new(DelegateTask),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_tools_returns_eight_unique_names() {
        let tools = builtin_tools(PathBuf::from("/tmp"));
        assert_eq!(tools.len(), 8);
        let mut names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
        names.sort_unstable();
        let mut deduped = names.clone();
        deduped.dedup();
        assert_eq!(names, deduped);
        assert_eq!(
            names,
            vec![
                "delegate_task",
                "file_patch",
                "file_read",
                "file_write",
                "memory",
                "terminal",
                "web_extract",
                "web_search",
            ]
        );
    }
}
