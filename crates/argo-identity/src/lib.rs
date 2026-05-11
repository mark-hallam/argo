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
pub mod circuit;
pub mod classification;
pub mod org;
pub mod org_ingest;
pub mod policy;
mod prompt;
pub mod roster;
mod roster_store;
mod store;
mod store_archive_methods;
mod store_channels;
mod store_skill_tests;
mod store_skills;
mod store_sql;
#[cfg(test)]
mod voice_profile_tests;
mod store_trust_grade;
mod types;
pub mod worker_budget;
mod worker_update;

pub use circuit::CircuitState;
pub use classification::{ClassRule, ClassificationRules, DataClass};
pub use org::{IngestSummary, MemberKind, OrgMember, OrgStore, OrgUnit};
pub use org_ingest::{OrgChartYaml, OrgMemberYaml, OrgUnitYaml};
pub use prompt::compose_system_prompt;
pub use roster::{Roster, RosterId, RosterStore};
pub use store::IdentityStore;
pub use types::{
    AutoApprovalPolicy, Duty, Formality, MemoryNs, MemoryScope, NotifyChannelRef, Persona, Role,
    Tone, VoiceProfile, Worker, WorkerId, WorkerLevel, MAX_CHARTER_BYTES, MAX_KPIS_BYTES,
};
pub use worker_update::WorkerUpdate;

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
