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

mod escalation;
mod handoff;
pub mod persist;
mod session;

pub use escalation::{EscalationChain, EscalationStep};
pub use handoff::{Handoff, HandoffId, HandoffStatus};
pub use persist::OrchestratorStore;
pub use session::{Session, SessionId, SessionStatus};
