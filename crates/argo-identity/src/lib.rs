//! Worker identity primitives for SKYCOT.
//!
//! A worker is the runtime instance that binds a vertical
//! template to persona, role, memory namespace, approval policy,
//! and notification channels.
//!
//! ```
//! use argo_identity::{Persona, Role, Worker};
//! use argo_types::TenantId;
//!
//! let worker = Worker::draft(
//!     TenantId::new(),
//!     "gtm-engineer",
//!     Persona::named("Maya", "Built by tests"),
//!     Role::new("Handle approved marketplace follow-up."),
//! );
//! assert_eq!(worker.persona.name, "Maya");
//! ```
#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

mod prompt;
mod store;
mod types;

pub use prompt::compose_system_prompt;
pub use store::IdentityStore;
pub use types::{
    AutoApprovalPolicy, Duty, MemoryNs, MemoryScope, NotifyChannelRef, Persona, Role, VoiceProfile,
    Worker, WorkerId,
};
