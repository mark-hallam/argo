//! `argo-core` — agent loop walking skeleton (§6, Agent F Phase 1).
//!
//! Phase-1 scope:
//! - `Agent::run_turn` against a single non-streaming provider call.
//! - One tool dispatch path (no parallelism, no compression, no retry,
//!   no streaming).
//! - Capability check before every tool dispatch via
//!   [`argo_tool::ToolRegistry`].
//! - Append-only message history via [`argo_store::SessionStore`].
//! - Hash-chained audit log via [`AuditSink`].
//!
//! Stops short of: streaming, parallel tool execution, compression,
//! retry/repair, subagent delegation, cost-tier routing, judge
//! integration, steer hooks, heartbeat. Those land with §6 Phase 2.
//!
//! ```
//! # use argo_core::{Agent, AgentDeps};
//! fn assert_send_sync<T: Send + Sync>() {}
//! assert_send_sync::<AgentDeps>();
//! ```
#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

mod agent;
mod audit;
mod iter_policy;
mod resume;
mod steer;

#[cfg(test)]
mod tests;

pub use agent::{Agent, AgentDeps, RunTurnOptions, TurnOutcome};
pub use audit::{AuditKind, AuditSink};
pub use iter_policy::{
    decide as decide_iteration_extension, ExtensionDecision, ExtensionLimits, IterationOnMax,
};
pub use resume::{
    apply_restart_policy, mark_session_discarded, scan_interrupted, InterruptedSession,
    RestartAction, RestartPolicy,
};
pub use steer::SteerChannel;
