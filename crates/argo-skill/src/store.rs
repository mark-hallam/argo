//! `SkillStore` — async wrapper over the `skill` + `skill_lock` tables in
//! argo-store.
use crate::parse::parse_skill_md;
use crate::skill::{PrincipalOverride, Skill, SkillFrontmatter};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, Lock, SessionId, SkillId};
use chrono::{TimeZone, Utc};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use uuid::Uuid;

/// Content-addressed skill store with append-only history + locks.
pub struct SkillStore {
    store: Arc<SessionStore>,
}

impl std::fmt::Debug for SkillStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SkillStore").finish()
    }
}

impl SkillStore {
    /// Build over an open session store.
    #[must_use]
    pub fn new(store: Arc<SessionStore>) -> Self {
        Self { store }
    }

    /// Install a skill from a raw `SKILL.md` document.
    ///
    /// If a skill with the same `name` already exists at any version, the
    /// install is rejected — use [`Self::supersede`] to bump versions.
    pub async fn install(&self, raw: &str, learned_in: Option<SessionId>) -> ArgoResult<Skill> {
        let (frontmatter, body) = parse_skill_md(raw)?;
        let existing = self.find_active_by_name(&frontmatter.name).await?;
        if existing.is_some() {
            return Err(ArgoError::Schema(format!(
                "skill '{}' already installed; use supersede() to bump",
                frontmatter.name
            )));
        }
        self.insert_row(frontmatter, body, learned_in).await
    }

    /// Replace the active version of `name` with a new `SKILL.md`. The new
    /// frontmatter MUST carry the same `name`.
    ///
    /// Lock semantics:
    /// - `UserProtected` → reject unless `principal == Force`.
    /// - `ContentLocked` → permitted, but the new skill's `learned_in`
    ///   chain is preserved by linking `superseded_by`.
    pub async fn supersede(
        &self,
        new_raw: &str,
        learned_in: Option<SessionId>,
        principal: PrincipalOverride,
    ) -> ArgoResult<Skill> {
        let (frontmatter, body) = parse_skill_md(new_raw)?;
        let prev = self
            .find_active_by_name(&frontmatter.name)
            .await?
            .ok_or_else(|| {
                ArgoError::Schema(format!(
                    "no installed skill named '{}'; use install() first",
                    frontmatter.name
                ))
            })?;
        if matches!(prev.frontmatter.lock, Lock::UserProtected)
            && principal != PrincipalOverride::Force
        {
            return Err(ArgoError::UserProtected {
                entity: format!("skill:{}", frontmatter.name),
            });
        }
        let new_skill = self.insert_row(frontmatter, body, learned_in).await?;
        sqlx::query("UPDATE skill SET superseded_by = ? WHERE id = ?")
            .bind(new_skill.id.0.as_bytes().as_slice())
            .bind(prev.id.0.as_bytes().as_slice())
            .execute(self.store.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("skill supersede: {e}")))?;
        Ok(new_skill)
    }

    /// Lock or unlock a skill.
    pub async fn set_lock(
        &self,
        skill_id: SkillId,
        lock: Lock,
        principal: PrincipalOverride,
    ) -> ArgoResult<()> {
        if principal != PrincipalOverride::Force {
            return Err(ArgoError::UserProtected {
                entity: format!("skill_lock:{} (only user can set/clear)", skill_id.0),
            });
        }
        match lock {
            Lock::None => {
                sqlx::query("DELETE FROM skill_lock WHERE skill_id = ?")
                    .bind(skill_id.0.as_bytes().as_slice())
                    .execute(self.store.pool())
                    .await
                    .map_err(|e| ArgoError::Provider(format!("skill unlock: {e}")))?;
            }
            other => {
                let label = match other {
                    Lock::UserProtected => "user-protected",
                    Lock::ContentLocked => "content-locked",
                    Lock::None => unreachable!(),
                };
                sqlx::query(
                    "INSERT INTO skill_lock (skill_id, lock, set_at, set_by) \
                     VALUES (?, ?, ?, 'user') \
                     ON CONFLICT(skill_id) DO UPDATE SET lock = excluded.lock, \
                     set_at = excluded.set_at, set_by = 'user'",
                )
                .bind(skill_id.0.as_bytes().as_slice())
                .bind(label)
                .bind(Utc::now().timestamp_micros())
                .execute(self.store.pool())
                .await
                .map_err(|e| ArgoError::Provider(format!("skill lock: {e}")))?;
            }
        }
        Ok(())
    }

    /// Active (non-superseded) version for `name`, if any.
    pub async fn find_active_by_name(&self, name: &str) -> ArgoResult<Option<Skill>> {
        let row: Option<RawSkill> = sqlx::query_as::<_, RawSkill>(
            "SELECT s.id, s.name, s.version, s.body, s.frontmatter, s.source, s.hash, \
             s.learned_in, s.superseded_by, s.created_at, l.lock \
             FROM skill s LEFT JOIN skill_lock l ON l.skill_id = s.id \
             WHERE s.name = ? AND s.superseded_by IS NULL",
        )
        .bind(name)
        .fetch_optional(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("skill lookup: {e}")))?;
        row.map(Skill::try_from).transpose()
    }

    /// All active skills, ordered by name.
    pub async fn list_active(&self) -> ArgoResult<Vec<Skill>> {
        let rows = sqlx::query_as::<_, RawSkill>(
            "SELECT s.id, s.name, s.version, s.body, s.frontmatter, s.source, s.hash, \
             s.learned_in, s.superseded_by, s.created_at, l.lock \
             FROM skill s LEFT JOIN skill_lock l ON l.skill_id = s.id \
             WHERE s.superseded_by IS NULL ORDER BY s.name ASC",
        )
        .fetch_all(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("skill list: {e}")))?;
        rows.into_iter().map(Skill::try_from).collect()
    }

    async fn insert_row(
        &self,
        mut frontmatter: SkillFrontmatter,
        body: String,
        learned_in: Option<SessionId>,
    ) -> ArgoResult<Skill> {
        let id = SkillId::new();
        let created_at = Utc::now();
        let hash = sha256(&body);
        // The frontmatter's `lock` field is informational; the live state
        // is the `skill_lock` table. Clear it so the returned struct
        // matches what `find_active_by_name` would yield.
        frontmatter.lock = Lock::None;
        let frontmatter_json = serde_json::to_string(&frontmatter)
            .map_err(|e| ArgoError::Schema(format!("frontmatter serialize: {e}")))?;
        sqlx::query(
            "INSERT INTO skill (id, name, version, body, frontmatter, source, hash, \
             learned_in, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id.0.as_bytes().as_slice())
        .bind(&frontmatter.name)
        .bind(&frontmatter.version)
        .bind(&body)
        .bind(&frontmatter_json)
        .bind(&frontmatter.provenance.source)
        .bind(hash.as_slice())
        .bind(learned_in.map(|s| s.0.as_bytes().to_vec()))
        .bind(created_at.timestamp_micros())
        .execute(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("skill install: {e}")))?;
        Ok(Skill {
            id,
            frontmatter,
            body,
            hash,
            learned_in,
            superseded_by: None,
            created_at,
        })
    }
}

fn sha256(body: &str) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(body.as_bytes());
    let out = h.finalize();
    let mut buf = [0u8; 32];
    buf.copy_from_slice(&out);
    buf
}

#[derive(sqlx::FromRow)]
struct RawSkill {
    id: Vec<u8>,
    name: String,
    #[allow(dead_code)]
    version: String,
    body: String,
    frontmatter: String,
    #[allow(dead_code)]
    source: String,
    hash: Vec<u8>,
    learned_in: Option<Vec<u8>>,
    superseded_by: Option<Vec<u8>>,
    created_at: i64,
    lock: Option<String>,
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
                "skill.name diverged from frontmatter.name",
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
        Ok(Skill {
            id,
            frontmatter,
            body: r.body,
            hash,
            learned_in,
            superseded_by,
            created_at,
        })
    }
}

fn uuid_from_blob(b: &[u8], field: &'static str) -> ArgoResult<Uuid> {
    Uuid::from_slice(b).map_err(|e| ArgoError::Schema(format!("{field}: {e}")))
}
