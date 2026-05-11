//! `argo-agent-runner` — reusable multi-turn agent loop.
//!
//! Drives the same `run_turn` pipeline that `argo chat` and the
//! gateway's `POST /sessions/<id>/runs` endpoint share, factored
//! out of `argo-cli` so non-binary callers (the TUI's live agent
//! mode, eval harnesses, future ACP/MCP daemons) can spawn live
//! agent turns directly without going through the binary.
//!
//! Wire shape:
//!   - [`run_turn`] / [`run_turn_with_events`] — orchestrate one
//!     turn against an open `SessionStore` + workspace `cwd`.
//!     Returns a [`TurnResult`] (assistant message id + final
//!     text + token counts).
//!   - [`RunEventSink`] — optional `mpsc::UnboundedSender` so the
//!     gateway streaming endpoint pushes events to clients in
//!     real time. CLI / non-streaming callers pass `None`.
//!   - [`MAX_TOOL_ROUNDS`] — default cap on tool-dispatch rounds
//!     within a single turn (8). Callers may override.
//!
//! Provider transport, tool dispatch, audit emission, and spend
//! recording all live in this crate; CLI and gateway just call
//! `run_turn[_with_events]` and surface the result.
//!
//! Manifest discovery: providers are loaded from
//! `<cwd>/manifests/providers/<provider-id>.toml`.
//!
//! Examples
//! ```ignore
//! use argo_agent_runner::{run_turn, MAX_TOOL_ROUNDS};
//! # async fn doit(store: std::sync::Arc<argo_store::SessionStore>,
//! #               cwd: &std::path::Path,
//! #               sid: argo_types::SessionId) -> argo_types::ArgoResult<()> {
//! let r = run_turn(&store, cwd, sid, Some("hi"), false, MAX_TOOL_ROUNDS).await?;
//! println!("assistant said {} bytes", r.text.len());
//! # Ok(()) }
//! ```
#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

mod dispatch;
#[cfg(test)]
mod dispatch_tests;
mod helpers;
mod transport;
mod turn;
mod turn_vertical;
mod turn_vertical_grants;

pub use helpers::{
    build_assistant_content, emit_event, lift_tool_blocks, message_from_row, price_for,
    record_turn_spend, stream_one_round, RoundResp, RunEventSink, SpendCtx,
};
pub use transport::{build_transport, load_manifest};
pub use turn::turn_planner::{maybe_decompose, DecomposeOutcome, PlannerHook};
pub use turn::{
    run_turn, run_turn_with_events, run_turn_with_planner_hook, TurnResult, MAX_TOOL_ROUNDS,
};
