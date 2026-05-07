//! Argo orchestrator — multi-worker session primitives (H-3).
//!
//! A `Session` is a thread of work that may involve N workers and
//! N humans. Each worker that touches the session emits actions
//! against the shared session memory namespace; handoffs let a
//! worker pass control + payload to the next worker (or human)
//! in the escalation chain.
//!
//! H-3 ships the data types + escalation chain walker. The
//! gateway / dispatch wiring (read/write Sessions + Handoffs in
//! the store) is a follow-up — H-3 keeps the surface tight enough
//! to land in one PR.
//!
//! ```
//! use argo_orchestrator::{EscalationChain, Session};
//! use argo_identity::WorkerId;
//! use argo_types::TenantId;
//!
//! let session = Session::new(TenantId::new(), "skate-followup");
//! assert_eq!(session.title, "skate-followup");
//!
//! let chain = EscalationChain::from_strs(&[
//!     "worker:support_agent",
//!     "worker:sales_agent",
//!     "human:mark@acmeco.com",
//! ]);
//! assert_eq!(chain.len(), 3);
//! ```
#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

pub mod auction;
/// Worker duty scheduler — autonomous cron-driven actions
/// (Stream `claude-worker-duty-scheduler`, V1.3).
pub mod duty_scheduler;
mod escalation;
mod handoff;
pub mod persist;
pub mod routing;
mod session;
mod walker;

pub use auction::{
    fit_score, Auction, AuctionBid, AuctionStatus, AuctionStore, AuctionTask, AuctionWinner,
};
pub use duty_scheduler::{
    spawn_duty_scheduler, ClosedGate, DutyGate, DutyRecord, DutyScheduler, DutyStore,
    FireDecision, OpenGate, TickReport,
};
pub use escalation::{EscalationChain, EscalationStep};
pub use handoff::{Handoff, HandoffId, HandoffStatus};
pub use persist::OrchestratorStore;
pub use routing::{
    HandoffRoutingRule, RoutingContext, RoutingEngine, RoutingRuleStore, RuleId, TargetSelector,
    WhenClause,
};
pub use session::{Session, SessionId, SessionStatus};
pub use walker::{HandoffNotifier, HandoffWalker, SweepReport};
