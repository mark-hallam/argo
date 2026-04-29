//! `argo-routing` — cost-tier routing (§24, Amendment 1).
//!
//! Provides three concrete [`Router`] implementations behind a common
//! trait:
//!
//! - [`FixedRouter`] — always returns the default model. Zero overhead.
//! - [`TieredRouter`] — runs the configured classifier model with the
//!   §24.2 fixed prompt, picks one of `simple/medium/complex`. Falls
//!   through to the default on slow / error / garbage classifier output
//!   so the loop never halts (§24.4).
//! - [`PolicyPluginRouter`] — placeholder; lights up with the Phase 4
//!   plugin host.
//!
//! [`PersistingRouter`] wraps any router and writes its non-Fixed
//! decisions to the `routing_decision` table (§24.4).
//!
//! ```
//! # use argo_routing::FixedRouter;
//! let _router = FixedRouter::new();
//! ```
#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

mod cache;
mod fixed;
mod persist;
mod policy_plugin;
mod router;
mod tiered;

pub use cache::{TierCache, DEFAULT_TTL};
pub use fixed::FixedRouter;
pub use persist::PersistingRouter;
pub use policy_plugin::PolicyPluginRouter;
pub use router::{Router, RoutingContext};
pub use tiered::TieredRouter;
