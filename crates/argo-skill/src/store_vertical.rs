//! Per-vertical extensions to [`SkillStore`] (#678 + #679).
//! Lives in its own file so `store.rs` stays under the
//! 500-LOC budget after the vertical-scoped lookup methods
//! landed.
//!
//! Three additions:
//!
//! - `install_for_vertical(raw, learned_in, vertical_id)` —
//!   tags the new skill row with the bundling vertical so per-
//!   vertical lookups can scope correctly (#678).
//! - `list_skills_bundled_by_vertical(vertical_id)` — returns
//!   only the skill rows tagged for that vertical.
//! - `list_skills_for_vertical(vertical_id)` — UNION of
//!   bundled-by-vertical + learned-in-a-session-of-vertical.
//!   Default skill set the agent runner uses for vertical-bound
//!   sessions (#679 — Rule 6).
use crate::parse::parse_skill_md;
use crate::skill::Skill;
use crate::store::SkillStore;
use argo_types::{ArgoError, ArgoResult, SessionId};

impl SkillStore {
    /// Install a skill, tagging it with the vertical that
    /// bundled it (#678). Used by
    /// `vertical_attach::install_and_attach_bundle` so per-
    /// vertical skill lookups can scope correctly.
    pub async fn install_for_vertical(
        &self,
        raw: &str,
        learned_in: Option<SessionId>,
        vertical_id: Option<&str>,
    ) -> ArgoResult<Skill> {
        let (frontmatter, body) = parse_skill_md(raw)?;
        let existing = self.find_active_by_name(&frontmatter.name).await?;
        if existing.is_some() {
            return Err(ArgoError::Schema(format!(
                "skill '{}' already installed; use supersede() to bump",
                frontmatter.name
            )));
        }
        self.insert_row(frontmatter, body, learned_in, vertical_id).await
    }

    /// List skills bundled by a specific vertical (#678). Returns
    /// only rows where `skill.vertical_id = ?` AND not superseded.
    /// Used by per-vertical skill discovery + by lint to confirm
    /// the bundled skill set.
    pub async fn list_skills_bundled_by_vertical(
        &self,
        vertical_id: &str,
    ) -> ArgoResult<Vec<Skill>> {
        let rows: Vec<crate::store::RawSkill> = sqlx::query_as::<_, crate::store::RawSkill>(
            "SELECT s.id, s.name, s.version, s.body, s.frontmatter, s.source, s.hash, \
                    s.learned_in, s.superseded_by, s.created_at, l.lock, \
                    s.vertical_id \
             FROM skill s LEFT JOIN skill_lock l ON l.skill_id = s.id \
             WHERE s.vertical_id = ? AND s.superseded_by IS NULL \
             ORDER BY s.created_at ASC",
        )
        .bind(vertical_id)
        .fetch_all(self.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("list_skills_bundled_by_vertical: {e}")))?;
        rows.into_iter().map(Skill::try_from).collect()
    }

    /// All active skills visible to a vertical-bound session
    /// (#679 — Rule 6). Returns the union of:
    ///
    /// - bundled skills tagged with `skill.vertical_id = ?`
    /// - learned skills installed via a session whose
    ///   `session.vertical_id = ?`
    ///
    /// Excludes everything else by default — workspace-global
    /// skills (no vertical context), other verticals' bundled
    /// skills, and other verticals' learned skills are NOT
    /// implicitly visible. Cross-vertical sharing requires the
    /// shared-asset surface (Rule 11, future PR).
    ///
    /// This is the default skill set the agent runner uses
    /// when no per-session attachments exist (#653 attachment
    /// path takes precedence when present).
    pub async fn list_skills_for_vertical(
        &self,
        vertical_id: &str,
    ) -> ArgoResult<Vec<Skill>> {
        let rows = sqlx::query_as::<_, crate::store::RawSkill>(
            "SELECT s.id, s.name, s.version, s.body, s.frontmatter, s.source, s.hash, \
                    s.learned_in, s.superseded_by, s.created_at, l.lock, s.vertical_id \
             FROM skill s \
             LEFT JOIN skill_lock l ON l.skill_id = s.id \
             LEFT JOIN session sess ON sess.id = s.learned_in \
             WHERE s.superseded_by IS NULL \
               AND ( \
                 s.vertical_id = ? \
                 OR sess.vertical_id = ? \
               ) \
             ORDER BY s.name ASC",
        )
        .bind(vertical_id)
        .bind(vertical_id)
        .fetch_all(self.pool())
        .await
        .map_err(|e| {
            ArgoError::Provider(format!("list_skills_for_vertical: {e}"))
        })?;
        rows.into_iter().map(Skill::try_from).collect()
    }
}
