//! `Tool` trait + supporting types.
//!
//! Spec: `docs/plan/argo-build-plan.md` §9. The trait is the single seam
//! between the agent loop's tool-dispatch path and the eight in-core tool
//! implementations.
use argo_types::{ArgoResult, CapabilitySet, ContentBlock, Principal, ToolCallResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Per-call context handed to a tool's `dispatch` method.
#[derive(Debug, Clone)]
pub struct ToolContext {
    /// Acting principal (used for capability checks + audit).
    pub principal: Principal,
    /// The workspace root the tool operates against; relative paths in
    /// args resolve here.
    pub workspace_root: PathBuf,
    /// Stable correlation id for the call.
    pub call_id: argo_types::ToolCallId,
    /// Wall-clock deadline for the call.
    pub deadline: Option<std::time::Instant>,
}

impl ToolContext {
    /// Convenience constructor used in tests + the agent loop.
    #[must_use]
    pub fn new(principal: Principal, workspace_root: PathBuf) -> Self {
        Self {
            principal,
            workspace_root,
            call_id: argo_types::ToolCallId::new(),
            deadline: None,
        }
    }
}

/// JSON-Schema description of a tool's argument shape, ready to ship to a
/// provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSchema {
    /// Tool name (matches `Tool::name`).
    pub name: String,
    /// One-line description visible to the model.
    pub description: String,
    /// JSON Schema for the `arguments` object.
    pub parameters: serde_json::Value,
}

/// Concurrency hint a tool exposes so the agent loop can decide whether
/// to dispatch in parallel with siblings (Phase 2 of §9: "concurrency
/// declarations").
///
/// Phase 1 walking skeleton uses a single dispatch path and ignores this;
/// Phase 2's parallel dispatch will honour it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Concurrency {
    /// Always safe to run in parallel with other Parallel tools (e.g.
    /// pure reads, network calls without side effects).
    Parallel,
    /// Holds an exclusive lock keyed by `lock_key` (e.g. file path for
    /// writes).
    ExclusiveBy(&'static str),
    /// Must run alone — no other tool dispatches until this one returns
    /// (e.g. terminal exec).
    Serial,
}

/// The in-core tool seam.
///
/// ```
/// # use argo_tool::Tool;
/// fn assert_object_safe(_t: &dyn Tool) {}
/// ```
#[async_trait]
pub trait Tool: Send + Sync + std::fmt::Debug {
    /// Stable name (e.g. `"file_read"`).
    fn name(&self) -> &str;

    /// One-line description for the model.
    fn description(&self) -> &str;

    /// JSON Schema describing the `arguments` object.
    fn parameters(&self) -> serde_json::Value;

    /// Concurrency declaration (§9 — parallel dispatch in Phase 2).
    fn concurrency(&self) -> Concurrency {
        Concurrency::Parallel
    }

    /// Capabilities required to run this tool with the given arguments.
    /// Returning a *narrow* set per-call (e.g. `FsRead { path_glob: "/etc/hosts" }`)
    /// lets the engine deny calls that exceed the principal's grant even
    /// when the broad declaration would pass.
    fn required_capabilities(&self, args: &serde_json::Value) -> ArgoResult<CapabilitySet>;

    /// Build a [`ToolSchema`] for shipping to providers.
    #[must_use]
    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: self.parameters(),
        }
    }

    /// Execute the call. Side-effects (FS / process / net) happen here.
    async fn dispatch(
        &self,
        args: serde_json::Value,
        ctx: &ToolContext,
    ) -> ArgoResult<ToolCallResult>;
}

/// Tiny helper used by every tool to produce a successful text result.
#[must_use]
pub fn success_text(
    call_id: argo_types::ToolCallId,
    text: String,
    started_ms: u64,
) -> ToolCallResult {
    ToolCallResult {
        id: call_id,
        is_error: false,
        content: vec![ContentBlock::Text { text }],
        duration_ms: started_ms,
        tokens_in: None,
        tokens_out: None,
    }
}

/// Tiny helper used by every tool to produce an error text result.
#[must_use]
pub fn error_text(
    call_id: argo_types::ToolCallId,
    text: String,
    started_ms: u64,
) -> ToolCallResult {
    ToolCallResult {
        id: call_id,
        is_error: true,
        content: vec![ContentBlock::Text { text }],
        duration_ms: started_ms,
        tokens_in: None,
        tokens_out: None,
    }
}
