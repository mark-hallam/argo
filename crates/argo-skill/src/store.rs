//! `SkillStore` — async wrapper over the `skill` + `skill_lock` tables in
//! argo-store.
//!
//! Read-side query methods (`find_active_by_name`, `list_active`,
//! `versions_for_name`, `list_all`, `skills_for_session`) and the
//! `RawSkill` row decoder live in the sibling [`store_lookups`] module
//! to keep this file under the 500-LOC HARD RULE 6 ceiling. The
//! `RawSkill` type is re-exported below as `pub(crate)` so other
//! modules in the crate (e.g. `store_vertical`) can keep referencing
//! `crate::store::RawSkill`.
use crate::parse::parse_skill_md;
use crate::skill::{PrincipalOverride, Skill, SkillFrontmatter};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, Lock, SessionId, SkillId};
use chrono::Utc;
use sha2::{Digest, Sha256};
use std::sync::Arc;

pub(crate) use crate::store_lookups::RawSkill;

/// Content-addressed skill store with append-only history + locks.
///
/// ```
/// # async fn doc() -> argo_types::ArgoResult<()> {
/// use std::sync::Arc;
/// use argo_skill::SkillStore;
/// use argo_store::SessionStore;
/// let store = Arc::new(SessionStore::open(":memory:").await?);
/// store.migrate().await?;
/// let skills = SkillStore::new(store);
/// let raw = "---\nname: greet\nversion: \"0.1.0\"\ndescription: \"hi\"\nprovenance:\n  source: builtin\n  hash: sha256-x\n---\nbody";
/// let installed = skills.install(raw, None).await?;
/// assert_eq!(installed.frontmatter.name, "greet");
/// # Ok(()) }
/// ```
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
        self.install_for_vertical(raw, learned_in, None).await
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
        let new_skill = self.insert_row(frontmatter, body, learned_in, None).await?;
        sqlx::query("UPDATE skill SET superseded_by = ? WHERE id = ?")
            .bind(new_skill.id.0.as_bytes().as_slice())
            .bind(prev.id.0.as_bytes().as_slice())
            .execute(self.store.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("skill supersede: {e}")))?;
        Ok(new_skill)
    }

    /// Promote `(name, version)` to be the active row for that
    /// skill name (i.e. the row where `superseded_by IS NULL`).
    /// Any other rows for the same name are linked back to the
    /// promoted row's id.
    ///
    /// Returns the promoted [`Skill`] on success;
    /// [`ArgoError::Schema`] when no row matches the
    /// `(name, version)` pair.
    ///
    /// This is the offline counterpart to `argo forge promote`:
    /// once a forge optimization landed a candidate row, calling
    /// `promote` flips the active pointer without touching the
    /// candidate's bytes.
    pub async fn promote(&self, name: &str, version: &str) -> ArgoResult<Skill> {
        let row: Option<RawSkill> = sqlx::query_as::<_, RawSkill>(
            "SELECT s.id, s.name, s.version, s.body, s.frontmatter, s.source, s.hash, \
             s.learned_in, s.superseded_by, s.created_at, l.lock, s.vertical_id, \
             s.lifecycle_state \
             FROM skill s LEFT JOIN skill_lock l ON l.skill_id = s.id \
             WHERE s.name = ? AND s.version = ?",
        )
        .bind(name)
        .bind(version)
        .fetch_optional(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("skill promote lookup: {e}")))?;
        let row = row.ok_or_else(|| {
            ArgoError::Schema(format!("no skill named '{name}' at version '{version}'"))
        })?;
        let target_id_blob = row.id.clone();
        // Point every other row for this name at the target's id.
        sqlx::query(
            "UPDATE skill SET superseded_by = ? WHERE name = ? AND id <> ?",
        )
        .bind(&target_id_blob)
        .bind(name)
        .bind(&target_id_blob)
        .execute(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("skill promote supersede chain: {e}")))?;
        // Clear the target's own superseded_by so it's the active row.
        sqlx::query("UPDATE skill SET superseded_by = NULL WHERE id = ?")
            .bind(&target_id_blob)
            .execute(self.store.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("skill promote clear: {e}")))?;
        let mut promoted = Skill::try_from(row)?;
        promoted.superseded_by = None;
        Ok(promoted)
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

    /// Attach an active skill to a session. The agent runner's
    /// system-prompt loader checks for attached skills first;
    /// when any are present, only those are injected (instead
    /// of every active row).
    pub async fn attach_to_session(
        &self,
        session_id: SessionId,
        skill_id: SkillId,
    ) -> ArgoResult<()> {
        sqlx::query(
            "INSERT OR IGNORE INTO session_skill (session_id, skill_id, attached_at) \
             VALUES (?, ?, ?)",
        )
        .bind(session_id.0.as_bytes().as_slice())
        .bind(skill_id.0.as_bytes().as_slice())
        .bind(Utc::now().timestamp_micros())
        .execute(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("attach_to_session: {e}")))?;
        Ok(())
    }

    /// Detach a skill from a session. Returns rows affected
    /// (0 when the (session, skill) pair wasn't attached).
    pub async fn detach_from_session(
        &self,
        session_id: SessionId,
        skill_id: SkillId,
    ) -> ArgoResult<u64> {
        let res = sqlx::query(
            "DELETE FROM session_skill WHERE session_id = ? AND skill_id = ?",
        )
        .bind(session_id.0.as_bytes().as_slice())
        .bind(skill_id.0.as_bytes().as_slice())
        .execute(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("detach_from_session: {e}")))?;
        Ok(res.rows_affected())
    }

    /// Pool accessor for sibling-impl modules (e.g.
    /// `store_vertical.rs`). Pub(crate) — not part of the
    /// public API.
    pub(crate) fn pool(&self) -> &sqlx::Pool<sqlx::Sqlite> {
        self.store.pool()
    }

    /// Get a clone of the underlying session-store Arc — used
    /// by callers that need to construct related stores
    /// (e.g. `AuditSink`) over the same database connection.
    /// Public so the CLI's `argo skill review` can emit audit
    /// rows alongside lifecycle transitions (#684).
    #[must_use]
    pub fn store_arc(&self) -> Arc<argo_store::SessionStore> {
        Arc::clone(&self.store)
    }

    pub(crate) async fn insert_row(
        &self,
        frontmatter: SkillFrontmatter,
        body: String,
        learned_in: Option<SessionId>,
        vertical_id: Option<&str>,
    ) -> ArgoResult<Skill> {
        // Bare install path → Active for backward compat.
        // The agent-proposed lifecycle path uses
        // `insert_row_with_state` directly via `propose_for_review`.
        self.insert_row_with_state(
            frontmatter,
            body,
            learned_in,
            vertical_id,
            crate::skill::LifecycleState::Active,
        )
        .await
    }

    pub(crate) async fn insert_row_with_state(
        &self,
        mut frontmatter: SkillFrontmatter,
        body: String,
        learned_in: Option<SessionId>,
        vertical_id: Option<&str>,
        state: crate::skill::LifecycleState,
    ) -> ArgoResult<Skill> {
        let id = SkillId::new();
        let created_at = Utc::now();
        let hash = sha256(&body);
        frontmatter.lock = Lock::None;
        let frontmatter_json = serde_json::to_string(&frontmatter)
            .map_err(|e| ArgoError::Schema(format!("frontmatter serialize: {e}")))?;
        sqlx::query(
            "INSERT INTO skill (id, name, version, body, frontmatter, source, hash, \
             learned_in, created_at, vertical_id, lifecycle_state) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
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
        .bind(vertical_id)
        .bind(state.label())
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
            vertical_id: vertical_id.map(str::to_string),
            lifecycle_state: state,
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
