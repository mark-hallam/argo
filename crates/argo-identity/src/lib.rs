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

pub mod auto_approval;
mod prompt;
mod store;
mod types;
pub mod worker_budget;

pub use prompt::compose_system_prompt;
pub use store::IdentityStore;
pub use types::{
    AutoApprovalPolicy, Duty, MemoryNs, MemoryScope, NotifyChannelRef, Persona, Role, VoiceProfile,
    Worker, WorkerId,
};

impl WorkerId {
    /// All-zero `WorkerId` sentinel — used as the v1 platform-default
    /// worker (the platform-shipped Worker every workspace gets at
    /// first init) and as a placeholder in tests / pre-#810 action
    /// rows.
    ///
    /// Defined here in `lib.rs` rather than `types.rs` because the
    /// brief restricting changes to `types.rs` carved this seam to
    /// keep the type definition fixed.
    ///
    /// ```
    /// use argo_identity::WorkerId;
    /// assert_eq!(WorkerId::nil().to_string(), "00000000-0000-0000-0000-000000000000");
    /// ```
    #[must_use]
    pub fn nil() -> Self {
        Self(uuid::Uuid::nil())
    }
}
