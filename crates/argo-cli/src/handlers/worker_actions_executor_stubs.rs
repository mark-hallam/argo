//! Placeholder `Executor` impls for the executor families that
//! land in later streams. Stream A (#A-executor-trait) keeps
//! `worker_actions_executor.rs` under the per-file 500-LOC
//! budget by holding only the trait + `HumanExecutor` there;
//! the three not-yet-implemented stubs live here.
//!
//! Each stub returns a `not yet implemented` error pointing at
//! the brief that wires the real impl. The operator sees the
//! gap immediately if a manifest declares one of these executor
//! kinds; nothing dispatches silently.
//!
//! - `McpExecutor` → wired by Stream `#C-mcp-wire` (calls a
//!   registered MCP server's tool).
//! - `ComputerUseExecutor` → wired by Stream
//!   `#B-computer-use-executor` (Anthropic CU loop in an E2B
//!   Firecracker sandbox).
//! - `ApiExecutor` → wired by the connector flavours used by
//!   Stream H's auto-approval + orchestrator integration.

use super::worker_actions::ActionRecord;
use super::worker_actions_executor::{ActionOutcome, Executor, ExecutorKind};
use argo_types::{ArgoError, ArgoResult};
use std::path::Path;

/// Placeholder for the MCP executor that Stream `#C-mcp-wire`
/// wires into the agent dispatch loop.
pub(crate) struct McpExecutor;

impl McpExecutor {
    pub(crate) fn new() -> Self {
        Self
    }
}

impl Executor for McpExecutor {
    fn kind(&self) -> ExecutorKind {
        ExecutorKind::Mcp
    }
    fn dispatch(&self, _action: &ActionRecord, _cwd: &Path) -> ArgoResult<ActionOutcome> {
        Err(ArgoError::Provider(
            "executor `mcp` is not yet implemented; track \
             docs/build-plan/streams/C-mcp-wire.md (Stream C \
             wires MCP into agent dispatch; once it merges, this \
             stub is swapped for the real impl)"
                .into(),
        ))
    }
}

/// Placeholder for the computer-use executor wired by Stream
/// `#B-computer-use-executor`. Drives a Firecracker microVM via
/// the Anthropic computer-use loop.
pub(crate) struct ComputerUseExecutor;

impl ComputerUseExecutor {
    pub(crate) fn new() -> Self {
        Self
    }
}

impl Executor for ComputerUseExecutor {
    fn kind(&self) -> ExecutorKind {
        ExecutorKind::ComputerUse
    }
    fn dispatch(&self, _action: &ActionRecord, _cwd: &Path) -> ArgoResult<ActionOutcome> {
        Err(ArgoError::Provider(
            "executor `computer_use` is not yet implemented; \
             track docs/build-plan/streams/B-computer-use-executor.md \
             (Stream B ships the E2B sandbox + Anthropic CU loop; \
             this stub swaps out when that lands)"
                .into(),
        ))
    }
}

/// Placeholder for the REST API executor wired by Stream H
/// connector flavours.
pub(crate) struct ApiExecutor;

impl ApiExecutor {
    pub(crate) fn new() -> Self {
        Self
    }
}

impl Executor for ApiExecutor {
    fn kind(&self) -> ExecutorKind {
        ExecutorKind::Api
    }
    fn dispatch(&self, _action: &ActionRecord, _cwd: &Path) -> ArgoResult<ActionOutcome> {
        Err(ArgoError::Provider(
            "executor `api` is not yet implemented; ApiExecutor \
             lands with the connector flavours that depend on it. \
             Until then, declare `executor = \"human\"` to fall \
             back to the artifact-bundle path."
                .into(),
        ))
    }
}
