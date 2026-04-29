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
