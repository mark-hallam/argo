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

mod parse;
mod skill;
mod store;
mod store_vertical;

#[cfg(test)]
mod tests;

pub use parse::{parse_skill_md, FRONTMATTER_DELIM};
pub use skill::{PrincipalOverride, Provenance, Skill, SkillFrontmatter};
pub use store::SkillStore;
