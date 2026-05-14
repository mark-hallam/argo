//! Worker-skill mutators and lookups on [`IdentityStore`] live in their
//! own file so the parent [`store`](crate::store) module stays under
//! the 500-LOC ceiling. The four methods underpin
//! `mission-routing-skillmatch-rolematch` (M4) `TargetSelector::SkillMatch`
//! resolution.

use crate::store::IdentityStore;
use crate::store_sql::parse_worker_id;
use crate::{Worker, WorkerId};
use argo_types::{ArgoError, ArgoResult, TenantId};
use sqlx::Row;

impl IdentityStore<'_> {
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
            .map_err(|e| ArgoError::Io(format!("remove worker_skill: {e}")))?;
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
