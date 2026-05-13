//! Read-side query methods for [`SkillStore`] + the `RawSkill` row
//! decoder they all funnel through.
//!
//! Split out of `store.rs` to keep that file under the 500-LOC HARD
//! RULE 6 ceiling. Pure refactor — no behaviour change. Call sites
//! continue to use `SkillStore::find_active_by_name`,
//! `SkillStore::list_active`, etc. unchanged; the inherent-impl block
//! is just hosted in this file. `RawSkill` is re-exported from
//! `store.rs` as `pub(crate) use crate::store_lookups::RawSkill;` so
//! sibling modules (e.g. `store_vertical.rs`) that reach in via
//! `crate::store::RawSkill` keep compiling.
use crate::skill::{Skill, SkillFrontmatter};
use crate::store::SkillStore;
use argo_types::{ArgoError, ArgoResult, Lock, SessionId, SkillId};
use chrono::{TimeZone, Utc};
use uuid::Uuid;

impl SkillStore {
    /// List the skill ids attached to a session in
    /// attachment-order. Returns an empty vec when nothing's
    /// attached — the agent runner falls back to all active
    /// in that case.
    pub async fn skills_for_session(
        &self,
        session_id: SessionId,
    ) -> ArgoResult<Vec<Skill>> {
        let rows: Vec<RawSkill> = sqlx::query_as::<_, RawSkill>(
            "SELECT s.id, s.name, s.version, s.body, s.frontmatter, s.source, s.hash, \
             s.learned_in, s.superseded_by, s.created_at, l.lock, s.vertical_id, \
             s.lifecycle_state \
             FROM session_skill ss \
             JOIN skill s ON s.id = ss.skill_id \
             LEFT JOIN skill_lock l ON l.skill_id = s.id \
             WHERE ss.session_id = ? \
             ORDER BY ss.attached_at ASC",
        )
        .bind(session_id.0.as_bytes().as_slice())
        .fetch_all(self.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("skills_for_session: {e}")))?;
        rows.into_iter().map(Skill::try_from).collect()
    }

    /// Active (non-superseded) version for `name`, if any.
    pub async fn find_active_by_name(&self, name: &str) -> ArgoResult<Option<Skill>> {
        let row: Option<RawSkill> = sqlx::query_as::<_, RawSkill>(
            "SELECT s.id, s.name, s.version, s.body, s.frontmatter, s.source, s.hash, \
             s.learned_in, s.superseded_by, s.created_at, l.lock, s.vertical_id, \
             s.lifecycle_state \
             FROM skill s LEFT JOIN skill_lock l ON l.skill_id = s.id \
             WHERE s.name = ? AND s.superseded_by IS NULL",
        )
        .bind(name)
        .fetch_optional(self.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("skill lookup: {e}")))?;
        row.map(Skill::try_from).transpose()
    }

    /// Every row matching `name`, including superseded — i.e.
    /// the full version history for one skill, sorted oldest
    /// first by `created_at`. Used by `GET /skills/<name>/versions`
    /// to surface the supersede chain to gateway clients.
    pub async fn versions_for_name(&self, name: &str) -> ArgoResult<Vec<Skill>> {
        let rows = sqlx::query_as::<_, RawSkill>(
            "SELECT s.id, s.name, s.version, s.body, s.frontmatter, s.source, s.hash, \
             s.learned_in, s.superseded_by, s.created_at, l.lock, s.vertical_id, \
             s.lifecycle_state \
             FROM skill s LEFT JOIN skill_lock l ON l.skill_id = s.id \
             WHERE s.name = ? ORDER BY s.created_at ASC",
        )
        .bind(name)
        .fetch_all(self.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("versions_for_name: {e}")))?;
        rows.into_iter().map(Skill::try_from).collect()
    }

    /// Every skill row, including superseded — sorted by name
    /// then `created_at`. Used by `GET /skills?include-superseded=true`
    /// to surface the full version history across every name.
    pub async fn list_all(&self) -> ArgoResult<Vec<Skill>> {
        let rows = sqlx::query_as::<_, RawSkill>(
            "SELECT s.id, s.name, s.version, s.body, s.frontmatter, s.source, s.hash, \
             s.learned_in, s.superseded_by, s.created_at, l.lock, s.vertical_id, \
             s.lifecycle_state \
             FROM skill s LEFT JOIN skill_lock l ON l.skill_id = s.id \
             ORDER BY s.name ASC, s.created_at ASC",
        )
        .fetch_all(self.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("skill list_all: {e}")))?;
        rows.into_iter().map(Skill::try_from).collect()
    }

    /// All active skills, ordered by name.
    pub async fn list_active(&self) -> ArgoResult<Vec<Skill>> {
        let rows = sqlx::query_as::<_, RawSkill>(
            "SELECT s.id, s.name, s.version, s.body, s.frontmatter, s.source, s.hash, \
             s.learned_in, s.superseded_by, s.created_at, l.lock, s.vertical_id, \
             s.lifecycle_state \
             FROM skill s LEFT JOIN skill_lock l ON l.skill_id = s.id \
             WHERE s.superseded_by IS NULL ORDER BY s.name ASC",
        )
        .fetch_all(self.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("skill list: {e}")))?;
        rows.into_iter().map(Skill::try_from).collect()
    }
}

#[derive(sqlx::FromRow)]
pub(crate) struct RawSkill {
    pub(crate) id: Vec<u8>,
    pub(crate) name: String,
    #[allow(dead_code)]
    pub(crate) version: String,
    pub(crate) body: String,
    pub(crate) frontmatter: String,
    #[allow(dead_code)]
    pub(crate) source: String,
    pub(crate) hash: Vec<u8>,
    pub(crate) learned_in: Option<Vec<u8>>,
    pub(crate) superseded_by: Option<Vec<u8>>,
    pub(crate) created_at: i64,
    pub(crate) lock: Option<String>,
    pub(crate) vertical_id: Option<String>,
    pub(crate) lifecycle_state: Option<String>,
}

impl TryFrom<RawSkill> for Skill {
    type Error = ArgoError;
    fn try_from(r: RawSkill) -> ArgoResult<Self> {
        let id = SkillId(uuid_from_blob(&r.id, "skill.id")?);
        let mut frontmatter: SkillFrontmatter = serde_json::from_str(&r.frontmatter)
            .map_err(|e| ArgoError::Schema(format!("skill.frontmatter JSON: {e}")))?;
        frontmatter.lock = match r.lock.as_deref() {
            None => Lock::None,
            Some("user-protected") => Lock::UserProtected,
            Some("content-locked") => Lock::ContentLocked,
            Some(other) => {
                return Err(ArgoError::Schema(format!(
                    "skill_lock.lock: unknown variant {other}"
                )))
            }
        };
        // Sanity check: stored name must match decoded frontmatter.
        if frontmatter.name != r.name {
            return Err(ArgoError::Invariant(
                "skill.name diverged from frontmatter.name".into(),
            ));
        }
        let learned_in = r
            .learned_in
            .map(|b| uuid_from_blob(&b, "skill.learned_in"))
            .transpose()?
            .map(SessionId);
        let superseded_by = r
            .superseded_by
            .map(|b| uuid_from_blob(&b, "skill.superseded_by"))
            .transpose()?
            .map(SkillId);
        let created_at = Utc
            .timestamp_micros(r.created_at)
            .single()
            .ok_or_else(|| ArgoError::Schema("skill.created_at out of range".into()))?;
        let mut hash = [0u8; 32];
        if r.hash.len() != 32 {
            return Err(ArgoError::Schema("skill.hash must be 32 bytes".into()));
        }
        hash.copy_from_slice(&r.hash);
        let lifecycle_state = r
            .lifecycle_state
            .as_deref()
            .and_then(crate::skill::LifecycleState::from_label)
            .unwrap_or(crate::skill::LifecycleState::Active);
        Ok(Skill {
            id,
            frontmatter,
            body: r.body,
            hash,
            learned_in,
            superseded_by,
            created_at,
            vertical_id: r.vertical_id,
            lifecycle_state,
        })
    }
}

fn uuid_from_blob(b: &[u8], field: &'static str) -> ArgoResult<Uuid> {
    Uuid::from_slice(b).map_err(|e| ArgoError::Schema(format!("{field}: {e}")))
}
