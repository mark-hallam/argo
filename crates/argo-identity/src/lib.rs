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
pub mod roster_policy_override;
mod roster_store;
mod store;
mod store_archive_methods;
mod store_channels;
mod store_hire;
mod store_role;
mod store_skill_tests;
mod store_skills;
mod store_sql;
mod store_trust_grade;
pub mod tenant;
pub mod tenant_provision;
pub mod tenant_store;
pub mod tier;
mod types;
pub mod user;
pub mod user_store;
pub mod user_tenant;
pub mod user_tenant_store;
#[cfg(test)]
mod voice_profile_tests;
pub mod worker_budget;
mod worker_update;

pub use circuit::CircuitState;
pub use classification::{ClassRule, ClassificationRules, DataClass};
pub use org::{IngestSummary, MemberKind, OrgMember, OrgStore, OrgUnit};
pub use org_ingest::{OrgChartYaml, OrgMemberYaml, OrgUnitYaml};
pub use prompt::compose_system_prompt;
pub use roster::{Roster, RosterId, RosterStore};
pub use roster_policy_override::RosterPolicyOverride;
pub use roster_store::RosterAssignment;
pub use store::IdentityStore;
pub use store_hire::{ApprovalPolicyChoice, HireDraft, HireDutyDraft};
pub use tenant::{Tenant, TenantStatus};
pub use tenant_provision::{provision_for_user, ProvisionOutcome, ProvisionRequest};
pub use tier::{AllowDecision, Tier, TierConfig, TierLimits};
pub use user::{UserAccount, UserId, UserStatus};
pub use user_tenant::{UserTenant, UserTenantRole};
pub use types::{
    AutoApprovalPolicy, Duty, Formality, MemoryNs, MemoryScope, NotifyChannelRef, Persona, Role,
    Tone, VoiceProfile, Worker, WorkerId, WorkerLevel, MAX_CHARTER_BYTES, MAX_KPIS_BYTES,
};
pub use worker_update::WorkerUpdate;
