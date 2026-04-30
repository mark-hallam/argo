//! `argo-replay` — time-travel session replay (§28.BB).
//!
//! Phase 7 ship: the replay machinery used by
//! `argo session replay <id>`, `argo session diff`, and
//! `argo session bisect`. Three pieces:
//!
//! - **[`Replayer`]** — async trait that walks a recorded session
//!   and re-emits each turn, optionally with model / tool /
//!   workspace overrides. The shipped [`VerbatimReplayer`] is the
//!   reference impl: it copies recorded assistant text through
//!   unchanged. With a verbatim plan it satisfies §28.BB gate 1
//!   ("replay reproduces original verbatim when no overrides").
//!   Real model-override replayers plug into `argo-provider` +
//!   `argo-judge` to compute score deltas (§28.BB gate 2).
//! - **[`diff_sessions`]** — structural diff (turn count, tool
//!   calls, output) between two sessions; the `ReplayDiff` value
//!   renders as a human-readable summary (§28.BB gate 3).
//! - **[`bisect`]** — linear scan that returns the first turn at
//!   which a synchronous hypothesis is violated (§28.BB gate 4).
//!
//! Decoupling: the crate depends on a [`SessionSource`] trait, not
//! on `argo-store` directly. The production binding builds turns
//! from `message` / `tool_call` rows; tests use [`InMemorySource`].
//!
//! ```
//! use argo_replay::{
//!     bisect, diff_sessions, InMemorySource, ReplayPlan, Replayer,
//!     SessionSource, ToolCallRecord, TurnRecord, VerbatimReplayer,
//! };
//! use argo_types::SessionId;
//! # async fn demo() -> argo_types::ArgoResult<()> {
//! let s = SessionId::new();
//! let mut src = InMemorySource::new();
//! src.insert(s, vec![TurnRecord {
//!     turn: 0,
//!     user_text: "hi".into(),
//!     assistant_text: "hello".into(),
//!     tool_calls: vec![],
//! }]);
//! let plan = ReplayPlan::verbatim(s);
//! let result = VerbatimReplayer.execute(&plan, &src).await?;
//! assert_eq!(result.divergence_count(), 0);
//! # Ok(()) }
//! ```
#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

mod diff;
mod replay;
mod source;

#[cfg(test)]
mod tests;

pub use diff::{bisect, diff_sessions, ReplayDiff, TurnDelta};
pub use replay::{ReplayPlan, ReplayResult, Replayer, TurnTrace, VerbatimReplayer};
pub use source::{InMemorySource, SessionSource, ToolCallRecord, TurnRecord};
