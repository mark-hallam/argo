use crate::store_sql::{
    ensure_role_columns, ensure_worker_charter_columns, ensure_worker_level_columns,
    ensure_worker_skill_table, ensure_worker_trust_grade_columns, insert_duty, kpis_to_json,
    parse_worker_id, to_json, worker_from_row, IDENTITY_SCHEMA, WORKER_SELECT,
};
use crate::{circuit, Duty, Worker, WorkerId, WorkerLevel, WorkerUpdate};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use chrono::Utc;
use sqlx::Row;
use uuid::Uuid;

/// SQLite-backed persistence for worker identities.
///
/// ```
/// # async fn demo() -> argo_types::ArgoResult<()> {
/// use argo_identity::{IdentityStore, Persona, Role, Worker};
/// use argo_store::SessionStore;
/// use argo_types::TenantId;
///
/// let store = SessionStore::open(":memory:").await?;
/// store.migrate().await?;
/// let identities = IdentityStore::new(&store);
/// identities.ensure_schema().await?;
/// let worker = Worker::draft(TenantId::new(), "gtm", Persona::named("Maya", "op"), Role::new("Sell."));
/// identities.create_worker(&worker).await?;
/// # Ok(()) }
/// ```
pub struct IdentityStore<'a> {
    pub(crate) store: &'a SessionStore,
}

impl<'a> IdentityStore<'a> {
    /// Wrap an existing session store.
    ///
    /// ```
    /// # async fn demo() -> argo_types::ArgoResult<()> {
    /// use argo_identity::IdentityStore;
    /// use argo_store::SessionStore;
    /// let store = SessionStore::open(":memory:").await?;
    /// let _identities = IdentityStore::new(&store);
    /// # Ok(()) }
    /// ```
    #[must_use]
    pub fn new(store: &'a SessionStore) -> Self {
        Self { store }
    }

    /// Create identity tables if they are missing.
    ///
    /// ```
    /// # async fn demo() -> argo_types::ArgoResult<()> {
    /// use argo_identity::IdentityStore;
    /// use argo_store::SessionStore;
    /// let store = SessionStore::open(":memory:").await?;
    /// store.migrate().await?;
    /// IdentityStore::new(&store).ensure_schema().await?;
    /// # Ok(()) }
    /// ```
    pub async fn ensure_schema(&self) -> ArgoResult<()> {
        for stmt in IDENTITY_SCHEMA.split_terminator(';') {
            let sql = stmt.trim();
            if sql.is_empty() {
                continue;
            }
            sqlx::query(sql)
                .execute(self.store.pool())
                .await
                .map_err(|e| ArgoError::Provider(format!("identity schema: {e}")))?;
        }
        ensure_role_columns(self.store.pool()).await?;
        ensure_worker_charter_columns(self.store.pool()).await?;
        ensure_worker_level_columns(self.store.pool()).await?;
        ensure_worker_trust_grade_columns(self.store.pool()).await?;
        ensure_worker_skill_table(self.store.pool()).await?;
        Ok(())
    }

    /// Persist a worker, its persona, role, and duties.
    ///
    /// ```
    /// # async fn demo() -> argo_types::ArgoResult<()> {
    /// use argo_identity::{IdentityStore, Persona, Role, Worker};
    /// use argo_store::SessionStore;
    /// use argo_types::TenantId;
    /// let store = SessionStore::open(":memory:").await?;
    /// store.migrate().await?;
    /// let identities = IdentityStore::new(&store);
    /// let worker = Worker::draft(TenantId::new(), "gtm", Persona::named("Maya", "op"), Role::new("Sell."));
    /// identities.create_worker(&worker).await?;
    /// # Ok(()) }
    /// ```
    pub async fn create_worker(&self, worker: &Worker) -> ArgoResult<()> {
        self.ensure_schema().await?;
        let persona_id = Uuid::now_v7().to_string();
        let role_name = format!("worker:{}", worker.id);
        let now = Utc::now().timestamp_micros();
        let mut tx = self
            .store
            .pool()
            .begin()
            .await
            .map_err(|e| ArgoError::Provider(format!("identity tx: {e}")))?;

        sqlx::query(
            "INSERT INTO persona (
                id, worker_id, tenant_id, name, voice_preamble, avatar, bio,
                builder_attribution, provenance_chain_json
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&persona_id)
        .bind(worker.id.to_string())
        .bind(worker.tenant_id.to_string())
        .bind(&worker.persona.name)
        .bind(&worker.persona.voice_profile.preamble)
        .bind(&worker.persona.avatar)
        .bind(&worker.persona.bio)
        .bind(&worker.persona.builder_attribution)
        .bind(to_json(&worker.persona.provenance_chain, "persona provenance")?)
        .execute(&mut *tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("insert persona: {e}")))?;

        sqlx::query(
            "INSERT INTO role (
                tenant_id, name, inherits_from, capabilities, worker_id,
                charter, kpis_json, escalation
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(worker.tenant_id.0.as_bytes().as_slice())
        .bind(&role_name)
        .bind(Option::<String>::None)
        .bind("[]")
        .bind(worker.id.to_string())
        .bind(&worker.role.charter)
        .bind(to_json(&worker.role.kpis, "role kpis")?)
        .bind(&worker.role.escalation)
        .execute(&mut *tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("insert role: {e}")))?;

        let kpis_json = kpis_to_json(worker.kpis.as_ref())?;
        let last_used_iso = worker.last_used_at.map(|ts| ts.to_rfc3339());
        sqlx::query(
            "INSERT INTO worker (
                id, tenant_id, vertical_id, persona_id, role_id, memory_scope,
                auto_approval_json, channels_json, created_at,
                charter_md, kpi_json, trust_grade, last_used_at
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(worker.id.to_string())
        .bind(worker.tenant_id.to_string())
        .bind(&worker.vertical_id)
        .bind(&persona_id)
        .bind(&role_name)
        .bind(worker.memory_namespace.scope.as_str())
        .bind(to_json(&worker.auto_approval, "auto approval")?)
        .bind(to_json(&worker.channels, "channels")?)
        .bind(now)
        .bind(worker.charter.as_deref())
        .bind(kpis_json)
        .bind(i64::from(worker.trust_grade))
        .bind(last_used_iso)
        .execute(&mut *tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("insert worker: {e}")))?;

        for duty in &worker.role.duties {
            insert_duty(&mut tx, worker, &role_name, duty, now).await?;
        }
        tx.commit()
            .await
            .map_err(|e| ArgoError::Provider(format!("identity commit: {e}")))?;
        Ok(())
    }

    /// Load one worker for a tenant.
    ///
    /// ```
    /// # async fn demo() -> argo_types::ArgoResult<()> {
    /// use argo_identity::{IdentityStore, Persona, Role, Worker};
    /// use argo_store::SessionStore;
    /// use argo_types::TenantId;
    /// let tenant = TenantId::new();
    /// let store = SessionStore::open(":memory:").await?;
    /// store.migrate().await?;
    /// let identities = IdentityStore::new(&store);
    /// let worker = Worker::draft(tenant, "gtm", Persona::named("Maya", "op"), Role::new("Sell."));
    /// identities.create_worker(&worker).await?;
    /// assert!(identities.get_worker(tenant, worker.id).await?.is_some());
    /// # Ok(()) }
    /// ```
    pub async fn get_worker(
        &self,
        tenant_id: TenantId,
        worker_id: WorkerId,
    ) -> ArgoResult<Option<Worker>> {
        self.ensure_schema().await?;
        let row = sqlx::query(WORKER_SELECT)
            .bind(worker_id.to_string())
            .bind(tenant_id.to_string())
            .fetch_optional(self.store.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("get worker: {e}")))?;
        match row {
            Some(row) => {
                let duties = self.load_duties(worker_id).await?;
                let mut worker = worker_from_row(row, duties)?;
                worker.circuit_state = circuit::state_for(
                    self.store.pool(),
                    &worker.id.to_string(),
                )
                .await?;
                Ok(Some(worker))
            }
            None => Ok(None),
        }
    }

    /// List workers for one tenant, newest first.
    ///
    /// Pass `include_archived = false` to filter out soft-deleted
    /// workers (`worker.archived_at IS NOT NULL`); `true` returns the
    /// full set including retired rows. Default operator listings use
    /// `false`; audit/export paths pass `true`.
    ///
    /// ```
    /// # async fn demo() -> argo_types::ArgoResult<()> {
    /// use argo_identity::{IdentityStore, Persona, Role, Worker};
    /// use argo_store::SessionStore;
    /// use argo_types::TenantId;
    /// let tenant = TenantId::new();
    /// let store = SessionStore::open(":memory:").await?;
    /// store.migrate().await?;
    /// let identities = IdentityStore::new(&store);
    /// identities.create_worker(&Worker::draft(tenant, "gtm", Persona::named("Maya", "op"), Role::new("Sell."))).await?;
    /// assert_eq!(identities.list_workers(tenant, false).await?.len(), 1);
    /// # Ok(()) }
    /// ```
    pub async fn list_workers(
        &self,
        tenant_id: TenantId,
        include_archived: bool,
    ) -> ArgoResult<Vec<Worker>> {
        self.ensure_schema().await?;
        let sql = if include_archived {
            "SELECT id FROM worker WHERE tenant_id = ? ORDER BY created_at DESC, id ASC"
        } else {
            "SELECT id FROM worker \
             WHERE tenant_id = ? AND archived_at IS NULL \
             ORDER BY created_at DESC, id ASC"
        };
        let ids: Vec<String> = sqlx::query_scalar(sql)
            .bind(tenant_id.to_string())
            .fetch_all(self.store.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("list workers: {e}")))?;

        let mut workers = Vec::with_capacity(ids.len());
        for id in ids {
            let worker_id = parse_worker_id(&id)?;
            if let Some(worker) = self.get_worker(tenant_id, worker_id).await? {
                workers.push(worker);
            }
        }
        Ok(workers)
    }

    async fn load_duties(&self, worker_id: WorkerId) -> ArgoResult<Vec<Duty>> {
        let rows = sqlx::query(
            "SELECT cron, entry_action, description
             FROM duty WHERE worker_id = ? ORDER BY created_at ASC, id ASC",
        )
        .bind(worker_id.to_string())
        .fetch_all(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("load duties: {e}")))?;
        Ok(rows
            .into_iter()
            .map(|row| {
                Duty::new(
                    row.get::<String, _>("cron"),
                    row.get::<String, _>("entry_action"),
                    row.get::<String, _>("description"),
                )
            })
            .collect())
    }

    /// Set the tenure `level` for a worker (Stream
    /// `claude-worker-promotion-workflow`). The promotion handler
    /// gates eligibility before calling in; this is the persistence
    /// primitive only and trusts the caller. Returns
    /// `ArgoError::Provider` when the `(tenant_id, worker_id)` pair
    /// does not match a stored row.
    pub async fn update_level(
        &self,
        tenant_id: TenantId,
        worker_id: WorkerId,
        level: WorkerLevel,
    ) -> ArgoResult<()> {
        self.ensure_schema().await?;
        let result = sqlx::query(
            "UPDATE worker SET level = ? WHERE id = ? AND tenant_id = ?",
        )
        .bind(level.as_str())
        .bind(worker_id.to_string())
        .bind(tenant_id.to_string())
        .execute(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("update level: {e}")))?;
        if result.rows_affected() == 0 {
            return Err(ArgoError::Provider(format!(
                "worker {worker_id} not found in tenant {tenant_id}"
            )));
        }
        Ok(())
    }

    /// Update the per-instance charter and/or KPIs for a worker
    /// (`Stream claude-worker-charter-storage`). Either argument may be
    /// `None` to null the column. Returns `ArgoError::Provider` when the
    /// (`tenant_id`, `worker_id`) pair does not match a stored row.
    pub async fn update_charter(
        &self,
        tenant_id: TenantId,
        worker_id: WorkerId,
        charter: Option<&str>,
        kpis: Option<&serde_json::Value>,
    ) -> ArgoResult<()> {
        self.ensure_schema().await?;
        let kpis_json = kpis_to_json(kpis)?;
        let result = sqlx::query(
            "UPDATE worker SET charter_md = ?, kpi_json = ? WHERE id = ? AND tenant_id = ?",
        )
        .bind(charter)
        .bind(kpis_json)
        .bind(worker_id.to_string())
        .bind(tenant_id.to_string())
        .execute(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("update charter: {e}")))?;
        if result.rows_affected() == 0 {
            return Err(ArgoError::Provider(format!(
                "worker {worker_id} not found in tenant {tenant_id}"
            )));
        }
        Ok(())
    }

    /// Soft-delete a worker by setting `worker.archived_at` to the
    /// current ISO-8601 timestamp (Stream `mvp-identity-archive-worker`,
    /// MISSION_BACKLOG #40 archive half). Audit and handoff history
    /// reference the textual `worker_id` and are deliberately left
    /// intact.
    ///
    /// Idempotent: a second call on an already-archived worker is a
    /// no-op (returns `Ok(())` without rewriting the timestamp).
    /// Returns `ArgoError::Provider` when the
    /// (`tenant_id`, `worker_id`) pair does not match any row.
    ///
    /// Distinct from `argo_store::worker_archive::archive_worker`,
    /// which is the heavier freeze/wake snapshot mechanism (deletes
    /// the live row after copying it into a sibling `worker_archive`
    /// table). This method only flips the soft-delete flag.
    pub async fn archive_worker(
        &self,
        tenant_id: TenantId,
        worker_id: WorkerId,
    ) -> ArgoResult<()> {
        self.ensure_schema().await?;
        let updated = sqlx::query(
            "UPDATE worker SET archived_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') \
             WHERE id = ? AND tenant_id = ? AND archived_at IS NULL",
        )
        .bind(worker_id.to_string())
        .bind(tenant_id.to_string())
        .execute(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("archive worker: {e}")))?;
        if updated.rows_affected() > 0 {
            return Ok(());
        }
        // 0 rows touched: either the worker doesn't exist (NotFound),
        // or it was already archived (idempotent no-op).
        let existing: Option<String> = sqlx::query_scalar(
            "SELECT id FROM worker WHERE id = ? AND tenant_id = ?",
        )
        .bind(worker_id.to_string())
        .bind(tenant_id.to_string())
        .fetch_optional(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("archive worker lookup: {e}")))?;
        if existing.is_none() {
            return Err(ArgoError::Provider(format!(
                "worker {worker_id} not found in tenant {tenant_id}"
            )));
        }
        Ok(())
    }

    /// Apply a partial update to a worker row (MISSION_BACKLOG #39,
    /// Stream `mission-identity-update-worker`). Unset fields on
    /// [`WorkerUpdate`] preserve their current values via a single
    /// COALESCE-based `UPDATE`. Returns the post-update [`Worker`].
    ///
    /// Errors: [`ArgoError::NotFound`] when the (`tenant_id`,
    /// `worker_id`) pair does not match a stored row.
    /// [`ArgoError::Schema`] when a supplied value is structurally
    /// invalid — today only an empty `vertical_id` triggers this; the
    /// `argo-types` audit (see `argo-types/src/error.rs`) makes
    /// `Schema` the canonical "caller-supplied data malformed"
    /// variant.
    ///
    /// No-op semantics: if every field on `updates` is `None`, the
    /// method short-circuits, skips the UPDATE, and returns the
    /// current row (still surfacing [`ArgoError::NotFound`] if the
    /// worker does not exist).
    pub async fn update_worker(
        &self,
        tenant_id: TenantId,
        worker_id: WorkerId,
        updates: WorkerUpdate,
    ) -> ArgoResult<Worker> {
        self.ensure_schema().await?;
        if let Some(vertical) = updates.vertical_id.as_deref() {
            if vertical.is_empty() {
                return Err(ArgoError::Schema(
                    "worker.vertical_id must not be empty".into(),
                ));
            }
        }
        if !updates.is_empty() {
            let result = sqlx::query(
                "UPDATE worker SET \
                    vertical_id = COALESCE(?, vertical_id), \
                    level = COALESCE(?, level), \
                    trust_grade = COALESCE(?, trust_grade), \
                    last_used_at = COALESCE(?, last_used_at) \
                 WHERE id = ? AND tenant_id = ?",
            )
            .bind(updates.vertical_id.as_deref())
            .bind(updates.level.map(|l| l.as_str()))
            .bind(updates.trust_grade.map(i64::from))
            .bind(updates.last_used_at.map(|ts| ts.to_rfc3339()))
            .bind(worker_id.to_string())
            .bind(tenant_id.to_string())
            .execute(self.store.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("update worker: {e}")))?;
            if result.rows_affected() == 0 {
                return Err(ArgoError::NotFound(format!(
                    "worker {worker_id} in tenant {tenant_id}"
                )));
            }
        }
        self.get_worker(tenant_id, worker_id).await?.ok_or_else(|| {
            ArgoError::NotFound(format!("worker {worker_id} in tenant {tenant_id}"))
        })
    }

    /// Associate `skill_id` with `worker_id` at the given `proficiency`
    /// (Stream `mvp-worker-skills-table`). Idempotent: a second call for
    /// the same `(worker_id, skill_id)` pair upserts the proficiency on
    /// the composite primary key. Tenant scoping is enforced at lookup
    /// time via [`Self::workers_with_skill`] (which JOINs against
    /// `worker`); this primitive trusts its caller.
    pub async fn add_worker_skill(
        &self,
        worker_id: WorkerId,
        skill_id: &str,
        proficiency: i64,
    ) -> ArgoResult<()> {
        self.ensure_schema().await?;
        sqlx::query(
            "INSERT INTO worker_skill (worker_id, skill_id, proficiency) VALUES (?, ?, ?)
             ON CONFLICT(worker_id, skill_id) DO UPDATE SET proficiency = excluded.proficiency",
        )
        .bind(worker_id.to_string())
        .bind(skill_id)
        .bind(proficiency)
        .execute(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("add worker_skill: {e}")))?;
        Ok(())
    }

    /// Remove the `(worker_id, skill_id)` association if present
    /// (Stream `mvp-worker-skills-table`). No-op when the row is absent.
    pub async fn remove_worker_skill(
        &self,
        worker_id: WorkerId,
        skill_id: &str,
    ) -> ArgoResult<()> {
        self.ensure_schema().await?;
        sqlx::query("DELETE FROM worker_skill WHERE worker_id = ? AND skill_id = ?")
            .bind(worker_id.to_string())
            .bind(skill_id)
            .execute(self.store.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("remove worker_skill: {e}")))?;
        Ok(())
    }

    /// Workers in `tenant_id` associated with `skill_id`, newest first
    /// (Stream `mvp-worker-skills-table`). Used by
    /// `mission-routing-skillmatch-rolematch` (M4) to resolve
    /// `TargetSelector::SkillMatch(String)`.
    pub async fn workers_with_skill(
        &self,
        tenant_id: TenantId,
        skill_id: &str,
    ) -> ArgoResult<Vec<Worker>> {
        self.ensure_schema().await?;
        let ids: Vec<String> = sqlx::query_scalar(
            "SELECT w.id FROM worker w
             JOIN worker_skill ws ON ws.worker_id = w.id
             WHERE w.tenant_id = ? AND ws.skill_id = ?
             ORDER BY w.created_at DESC, w.id ASC",
        )
        .bind(tenant_id.to_string())
        .bind(skill_id)
        .fetch_all(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("workers_with_skill: {e}")))?;

        let mut workers = Vec::with_capacity(ids.len());
        for id in ids {
            let worker_id = parse_worker_id(&id)?;
            if let Some(worker) = self.get_worker(tenant_id, worker_id).await? {
                workers.push(worker);
            }
        }
        Ok(workers)
    }

    /// All `(skill_id, proficiency)` rows for `worker_id`, ordered by
    /// `skill_id` for determinism (Stream `mvp-worker-skills-table`).
    pub async fn skills_for_worker(
        &self,
        worker_id: WorkerId,
    ) -> ArgoResult<Vec<(String, i64)>> {
        self.ensure_schema().await?;
        let rows = sqlx::query(
            "SELECT skill_id, proficiency FROM worker_skill
             WHERE worker_id = ? ORDER BY skill_id ASC",
        )
        .bind(worker_id.to_string())
        .fetch_all(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("skills_for_worker: {e}")))?;
        Ok(rows
            .into_iter()
            .map(|row| {
                (
                    row.get::<String, _>("skill_id"),
                    row.get::<i64, _>("proficiency"),
                )
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Persona, Role};

    #[tokio::test]
    async fn create_get_list_roundtrip() {
        let store = SessionStore::open(":memory:").await.unwrap();
        store.migrate().await.unwrap();
        let tenant = TenantId::new();
        let mut role = Role::new("Own outbound follow-up.");
        role.duties
            .push(Duty::new("0 9 * * 1-5", "follow_up", "Send approved follow-up."));
        let worker = Worker::draft(tenant, "gtm-engineer", Persona::named("Maya", "op"), role);
        let identities = IdentityStore::new(&store);

        identities.create_worker(&worker).await.unwrap();
        let loaded = identities
            .get_worker(tenant, worker.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(loaded.persona.name, "Maya");
        assert_eq!(loaded.role.duties[0].entry_action, "follow_up");
        assert_eq!(identities.list_workers(tenant, false).await.unwrap().len(), 1);
    }
}

#[cfg(test)]
#[path = "store_archive_tests.rs"]
mod archive_tests;

#[cfg(test)]
#[path = "worker_update_tests.rs"]
mod worker_update_tests;
