//! Skill data types.
use argo_types::{CapabilitySet, Lock, SessionId, SkillId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Where a skill came from.
///
/// `source` is one of `builtin`, `bundle:<bundle-name>`, `user`, `forge`
/// (and others as the marketplace evolves). `hash` is `sha256-…` of the
/// canonical body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provenance {
    /// Where the skill came from.
    pub source: String,
    /// `sha256-<hex>` of the canonical body.
    pub hash: String,
}

/// Parsed YAML frontmatter from a `SKILL.md` file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillFrontmatter {
    /// Skill name (must be unique per workspace).
    pub name: String,
    /// SemVer-shaped version string.
    pub version: String,
    /// One-line description visible to the model.
    pub description: String,
    /// Capabilities the skill needs at dispatch time.
    #[serde(default)]
    pub capabilities: CapabilitySet,
    /// Where the skill came from.
    pub provenance: Provenance,
    /// Edit-protection lock (Amendment 3).
    #[serde(default)]
    pub lock: Lock,
}

/// One row in `skill` plus its lock + decoded body.
#[derive(Debug, Clone)]
pub struct Skill {
    /// Stable id.
    pub id: SkillId,
    /// Parsed frontmatter.
    pub frontmatter: SkillFrontmatter,
    /// Markdown body (everything after the frontmatter block).
    pub body: String,
    /// SHA-256 of the canonical body string.
    pub hash: [u8; 32],
    /// Session that learned this skill, if any.
    pub learned_in: Option<SessionId>,
    /// Predecessor in the supersession chain.
    pub superseded_by: Option<SkillId>,
    /// When the skill was first installed.
    pub created_at: DateTime<Utc>,
    /// Vertical that bundled this skill, if any (#678). NULL
    /// for legacy skills installed before migration v9 + for
    /// skills installed via the bare `install()` (no vertical
    /// context).
    pub vertical_id: Option<String>,
    /// Lifecycle state (#681). New rows from `install` /
    /// `install_for_vertical` default to `Active` for backward
    /// compat. Agent-proposed learned skills go through
    /// `install_for_review` which starts them in
    /// `PendingReview`; only `transition_lifecycle` flips
    /// them to `Active` (or `Rejected` / `Archived`).
    pub lifecycle_state: LifecycleState,
}

/// Learned-skill lifecycle state machine (#681 — work order #3).
///
/// State transitions enforced by [`crate::SkillStore::transition_lifecycle`]:
///
/// - `Draft` → `PendingReview` (agent submits for human review)
/// - `PendingReview` → `Active` (human approves)
/// - `PendingReview` → `Rejected` (human refuses)
/// - `Active` → `Archived` (rollback / supersede)
///
/// Lookup methods that gate the agent's runtime view of
/// available skills (e.g.
/// `SkillStore::list_skills_for_vertical`) filter to
/// `lifecycle_state = 'active'` so non-active rows never
/// influence agent decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleState {
    /// Agent is still drafting; not visible to anyone yet.
    Draft,
    /// Submitted for human review; not visible to the agent.
    PendingReview,
    /// Approved by a human; visible + usable.
    Active,
    /// Refused by a human; never loaded again.
    Rejected,
    /// Was active but rolled back / superseded; preserved for
    /// audit but not loaded.
    Archived,
}

impl LifecycleState {
    /// Stable wire label (matches the `lifecycle_state` column
    /// values).
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::PendingReview => "pending_review",
            Self::Active => "active",
            Self::Rejected => "rejected",
            Self::Archived => "archived",
        }
    }

    /// Parse a wire label back into a state. Returns `None` for
    /// unknown values so the caller can fail loud.
    #[must_use]
    pub fn from_label(s: &str) -> Option<Self> {
        match s {
            "draft" => Some(Self::Draft),
            "pending_review" => Some(Self::PendingReview),
            "active" => Some(Self::Active),
            "rejected" => Some(Self::Rejected),
            "archived" => Some(Self::Archived),
            _ => None,
        }
    }
}

/// Caller-supplied override flag for agent-driven writes against a
/// `Lock::UserProtected` skill (mirrors the memory store's flag).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrincipalOverride {
    /// Standard agent dispatch — locks honoured.
    Agent,
    /// Human-in-the-loop override — `UserProtected` is permitted to
    /// write through.
    Force,
}
