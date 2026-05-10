//! `argo-tool` — tool registry + the eight in-core tools (§9).
//!
//! - [`Tool`] is the trait every tool implements.
//! - [`ToolRegistry::dispatch`] is the single seam where every call is
//!   gated against the principal's capabilities (Invariant 3, §10).
//! - [`builtin_tools`] returns the eight in-core tools ready to register.
//!
//! Phase 3 ships four tools end-to-end (`file_read`, `file_write`,
//! `file_patch`, `terminal`) and four schema-only stubs whose backends
//! land later (`web_search`, `web_extract` → MCP §12; `memory` → Agent H;
//! `delegate_task` → Agent K / §16). Hermes-parity tools `vision_analyze`,
//! `code_execute`, `send_message`, `todo`, and `clarify` ship alongside.
//!
//! ```
//! # use std::path::PathBuf;
//! use argo_tool::{builtin_tools, ToolRegistry};
//! let mut reg = ToolRegistry::new();
//! for t in builtin_tools(PathBuf::from("/tmp")) {
//!     reg.register(t);
//! }
//! assert!(reg.names().contains(&"code_execute"));
//! assert!(reg.names().contains(&"vision_analyze"));
//! assert!(reg.names().contains(&"todo"));
//! assert!(reg.names().contains(&"clarify"));
//! ```
#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

mod path_guard;
mod registry;
mod tool;
mod tools;

pub use path_guard::confine_path;
pub use registry::ToolRegistry;
pub use tool::{
    error_text, success_text, Concurrency, Tool, ToolContext, ToolProgressEvent, ToolSchema,
    STREAM_CHUNK_MAX_BYTES,
};
pub use tools::{
    builtin_tools, builtin_tools_with_memory, risk_tier, Clarify, ClarifyAnswer, ClarifyDispatcher,
    CodeExecute, DelegateTask, FilePatch, FileRead, FileWrite, Memory, MessageDispatcher,
    ResolvedImage, SendMessage, SendOutcome, Terminal, Todo, TodoItem, Vision, VisionAnswer,
    VisionDispatcher, WebExtract, WebSearch,
};
