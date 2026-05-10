//! `Tool` trait + supporting types.
//!
//! Spec: `docs/plan/argo-build-plan.md` §9. The trait is the single seam
//! between the agent loop's tool-dispatch path and the eight in-core tool
//! implementations.
use crate::process_registry::ProcessRegistry;
use argo_types::{ArgoResult, CapabilitySet, ContentBlock, Principal, ToolCallResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Per-line streaming event emitted by tools that opt into progress
/// reporting. The `terminal` tool is the first subscriber; other tools
/// (`code_execute`, `delegate_task`, …) can opt in incrementally.
///
/// Each `Stdout` / `Stderr` payload is a single line (sans trailing
/// newline). Lines longer than [`STREAM_CHUNK_MAX_BYTES`] are split so
/// the channel can't memory-explode on a runaway stream. A final
/// `Exited` event marks the end of the run; receivers can use it as a
/// completion sentinel.
///
/// ```
/// use argo_tool::ToolProgressEvent;
/// let ev = ToolProgressEvent::Stdout("hello".into());
/// assert!(matches!(ev, ToolProgressEvent::Stdout(_)));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolProgressEvent {
    /// One stdout line (no trailing newline). May be split if longer
    /// than [`STREAM_CHUNK_MAX_BYTES`].
    Stdout(String),
    /// One stderr line (no trailing newline). May be split if longer
    /// than [`STREAM_CHUNK_MAX_BYTES`].
    Stderr(String),
    /// Child process exited; emitted exactly once after both streams
    /// drain. `exit_code` is `-1` if the process was killed by a signal.
    Exited {
        /// Process exit code (`-1` if killed by signal).
        exit_code: i32,
    },
}

/// Maximum byte size of a single `Stdout`/`Stderr` event. A producer
/// that reads a line longer than this splits it into multiple events
/// so a single pathological line can't flood the channel.
pub const STREAM_CHUNK_MAX_BYTES: usize = 4 * 1024;

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
    /// Optional per-line progress channel. When `Some`, tools that
    /// support streaming (currently `terminal`) emit
    /// [`ToolProgressEvent`]s as output is produced. When `None`, tools
    /// fall through to their non-streaming code path and behavior is
    /// byte-for-byte identical to before this field existed.
    ///
    /// ```
    /// use argo_tool::{ToolContext, ToolProgressEvent};
    /// use argo_types::Principal;
    /// use tokio::sync::mpsc;
    ///
    /// let (tx, _rx) = mpsc::channel::<ToolProgressEvent>(64);
    /// let ctx = ToolContext::new(
    ///     Principal::Local { os_user: "demo".into() },
    ///     std::path::PathBuf::from("/tmp"),
    /// )
    /// .with_progress_tx(tx);
    /// assert!(ctx.progress_tx.is_some());
    /// ```
    pub progress_tx: Option<mpsc::Sender<ToolProgressEvent>>,
    /// Optional shared registry of background processes. When
    /// `None`, `terminal { background: true }` and the `process`
    /// tool both fail with a clear "background unsupported in this
    /// context" error. Wired by the agent runner once per session.
    pub background_processes: Option<Arc<ProcessRegistry>>,
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
            progress_tx: None,
            background_processes: None,
        }
    }

    /// Builder helper attaching a [`ToolProgressEvent`] sender.
    /// Defaults to `None`; existing call sites compile unchanged.
    #[must_use]
    pub fn with_progress_tx(mut self, tx: mpsc::Sender<ToolProgressEvent>) -> Self {
        self.progress_tx = Some(tx);
        self
    }

    /// Builder-style attach for the background-process registry.
    #[must_use]
    pub fn with_background_processes(mut self, reg: Arc<ProcessRegistry>) -> Self {
        self.background_processes = Some(reg);
        self
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
