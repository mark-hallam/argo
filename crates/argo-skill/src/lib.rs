//! `argo-skill` — content-addressed skill store with locks (§14.2).
//!
//! A skill is a `SKILL.md` file with a YAML frontmatter block followed
//! by a free-form Markdown body. Frontmatter declares `name`, `version`,
//! `description`, the `capabilities` the skill needs, `provenance`, and a
//! `lock` setting (Amendment 3).
//!
//! Locks gate writes via [`SkillStore::install`] and
//! [`SkillStore::supersede`]:
//!
//! - `Lock::None` — agent may freely edit.
//! - `Lock::UserProtected` — agent edits rejected with
//!   [`argo_types::ArgoError::UserProtected`]; only [`PrincipalOverride::Force`]
//!   may write through.
//! - `Lock::ContentLocked` — agent edits allowed but the new skill MUST
//!   reference the previous skill via `supersede` so the lineage hash
//!   chain stays intact.
//!
//! Spec: `docs/plan/argo-build-plan.md` §14.2.
#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

pub mod conflicts;
pub mod learn;
pub mod library;
mod parse;
mod skill;
mod store;
mod store_vertical;
pub mod success_metric;
pub mod trust;

#[cfg(test)]
mod tests;

pub use learn::{should_propose, SkillCandidate, DEFAULT_SUCCESS_THRESHOLD};
pub use success_metric::{
    should_propose_metric, ProposalThreshold, SuccessMetric,
};
pub use conflicts::{detect_conflicts, parse_keep_flag, render_diff, SkillConflict};
pub use library::{LayeredLookup, LayeredSkill, SkillLayer};
pub use parse::{parse_skill_md, FRONTMATTER_DELIM};
pub use skill::{LifecycleState, PrincipalOverride, Provenance, Skill, SkillFrontmatter};
pub use store::SkillStore;
pub use trust::{is_skill_eligible, min_trust_for_risk, TrustGrade};
