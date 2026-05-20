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
use crate::skill::{PrincipalOverride, Skill};
use crate::trust::TrustGrade;
use argo_store::skill_provenance::{record_provenance, NewSkillProvenance, SkillProvenanceKind};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, Lock, SessionId, SkillId};
use chrono::Utc;
use std::sync::Arc;

#[path = "store_trust_grade.rs"]
mod store_trust_grade;
#[path = "store_write.rs"]
mod store_write;

pub(crate) use crate::store_lookups::RawSkill;
pub(crate) use store_trust_grade::{trust_grade_as_u8, trust_grade_from_u8};

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
        let previous = self.find_active_by_name(name).await?;
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
        sqlx::query("UPDATE skill SET superseded_by = ? WHERE name = ? AND id <> ?")
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
        record_provenance(
            self.store.pool(),
            NewSkillProvenance {
                skill_name: name,
                skill_version: &promoted.frontmatter.version,
                kind: SkillProvenanceKind::ForgePromoted,
                actor: Some("forge"),
                parent_id: previous
                    .as_ref()
                    .map(|skill| skill.frontmatter.version.as_str()),
                change_reason: Some("forge promotion"),
                breaking_change: false,
            },
        )
        .await?;
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

    /// Set or clear the minimum [`TrustGrade`] required for the
    /// active skill named `name` to be surfaced.
    ///
    /// `Some(grade)` writes the enum's numeric discriminant into
    /// `skill.min_trust_grade`. `None` clears the column back to
    /// NULL — "no restriction" — which is the v1 default. Returns
    /// [`ArgoError::Provider`] if no active row matches `name`.
    ///
    /// Enforcement (refusing to surface a skill for a worker whose
    /// `worker.trust_grade` is below the configured floor) lives in
    /// the M6 chain (`mission-skill-store-available-for-worker` +
    /// `mission-skill-session-boot-wire`); this method only
    /// persists the value.
    pub async fn set_min_trust_grade(
        &self,
        name: &str,
        grade: Option<TrustGrade>,
    ) -> ArgoResult<()> {
        let value = grade.map(trust_grade_as_u8).map(i64::from);
        let affected = sqlx::query(
            "UPDATE skill SET min_trust_grade = ? \
             WHERE name = ? AND superseded_by IS NULL",
        )
        .bind(value)
        .bind(name)
        .execute(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("set_min_trust_grade: {e}")))?
        .rows_affected();
        if affected == 0 {
            return Err(ArgoError::Provider(format!(
                "no active skill named '{name}'"
            )));
        }
        Ok(())
    }

    /// Read the current `min_trust_grade` for the active skill
    /// named `name`. Returns `Ok(None)` for "no restriction" (the
    /// default) or `Ok(Some(grade))` when a value has been set.
    /// Returns [`ArgoError::Provider`] when no active row matches
    /// `name`.
    pub async fn get_min_trust_grade(&self, name: &str) -> ArgoResult<Option<TrustGrade>> {
        let row: Option<(Option<i64>,)> = sqlx::query_as(
            "SELECT min_trust_grade FROM skill \
             WHERE name = ? AND superseded_by IS NULL",
        )
        .bind(name)
        .fetch_optional(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("get_min_trust_grade: {e}")))?;
        let (raw,) =
            row.ok_or_else(|| ArgoError::Provider(format!("no active skill named '{name}'")))?;
        match raw {
            None => Ok(None),
            Some(v) => {
                let byte = u8::try_from(v).map_err(|_| {
                    ArgoError::Schema(format!(
                        "skill.min_trust_grade out of range for '{name}': {v}"
                    ))
                })?;
                let grade = trust_grade_from_u8(byte).ok_or_else(|| {
                    ArgoError::Schema(format!(
                        "skill.min_trust_grade unknown discriminant for '{name}': {byte}"
                    ))
                })?;
                Ok(Some(grade))
            }
        }
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
        let res = sqlx::query("DELETE FROM session_skill WHERE session_id = ? AND skill_id = ?")
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
}
