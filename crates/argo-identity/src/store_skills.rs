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
use std::collections::HashSet;

/// Outcome of a [`IdentityStore::bulk_assign_skill_to_workers`] call.
/// `assigned` counts rows the bulk insert created; `already_had`
/// counts rows whose `(worker_id, skill_id)` pair was already
/// present in the table at call time. Idempotent re-application is
/// not an error — the bulk action UI surfaces `already_had` as a
/// non-blocking detail line.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BulkAssignSkillOutcome {
    /// Number of workers that did NOT previously have the skill;
    /// equals the row-count of the upsert minus `already_had`.
    pub assigned: usize,
    /// Number of workers that already had the `(worker, skill)`
    /// pair — the bulk action treated them as a no-op.
    pub already_had: usize,
}

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
    pub async fn remove_worker_skill(&self, worker_id: WorkerId, skill_id: &str) -> ArgoResult<()> {
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
    pub async fn skills_for_worker(&self, worker_id: WorkerId) -> ArgoResult<Vec<(String, i64)>> {
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

    /// `(skill_id, learned_at_iso)` pairs for `worker_id`. `learned_at`
    /// is `worker_skill.created_at` — the table doesn't carry a
    /// separate `acquired_at` column (mission-skill-discovery brief
    /// accepts `created_at` as the learning-loop timestamp).
    pub async fn skills_learned_for_worker(
        &self,
        worker_id: WorkerId,
    ) -> ArgoResult<Vec<(String, String)>> {
        self.ensure_schema().await?;
        let rows = sqlx::query(
            "SELECT skill_id, created_at FROM worker_skill
             WHERE worker_id = ? ORDER BY skill_id ASC",
        )
        .bind(worker_id.to_string())
        .fetch_all(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("skills_learned_for_worker: {e}")))?;
        Ok(rows
            .into_iter()
            .map(|row| {
                (
                    row.get::<String, _>("skill_id"),
                    row.get::<String, _>("created_at"),
                )
            })
            .collect())
    }

    /// Workers in `tenant_id` associated with `skill_id` paired with
    /// their `worker_skill.created_at` ("learned at") timestamps. Used
    /// by the skill-discovery marketplace surface to render the
    /// "Workers using this skill" section with "Learned N weeks ago"
    /// chips.
    pub async fn workers_with_skill_learned_at(
        &self,
        tenant_id: TenantId,
        skill_id: &str,
    ) -> ArgoResult<Vec<(Worker, String)>> {
        self.ensure_schema().await?;
        let rows: Vec<(String, String)> = sqlx::query_as(
            "SELECT w.id, ws.created_at FROM worker w
             JOIN worker_skill ws ON ws.worker_id = w.id
             WHERE w.tenant_id = ? AND ws.skill_id = ?
             ORDER BY ws.created_at DESC, w.id ASC",
        )
        .bind(tenant_id.to_string())
        .bind(skill_id)
        .fetch_all(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("workers_with_skill_learned_at: {e}")))?;

        let mut out = Vec::with_capacity(rows.len());
        for (id, created_at) in rows {
            let worker_id = parse_worker_id(&id)?;
            if let Some(worker) = self.get_worker(tenant_id, worker_id).await? {
                out.push((worker, created_at));
            }
        }
        Ok(out)
    }

    /// Total count of workers (across **all** tenants) associated with
    /// `skill_id`. The brief deliberately surfaces only an aggregate so
    /// no cross-tenant identity leaks; the integer alone is the
    /// "marketplace adoption" signal.
    pub async fn count_workers_with_skill_global(&self, skill_id: &str) -> ArgoResult<i64> {
        self.ensure_schema().await?;
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(DISTINCT worker_id) FROM worker_skill WHERE skill_id = ?",
        )
        .bind(skill_id)
        .fetch_one(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("count_workers_with_skill_global: {e}")))?;
        Ok(count)
    }

    /// Assign one `skill_id` to every worker in `worker_ids` in a
    /// single SQL transaction (Stream
    /// `mission-cohort-detail-bulk-assign-skill`). Idempotent: workers
    /// who already hold the skill are counted in `already_had` rather
    /// than reported as an error. Tenant scoping is the caller's
    /// responsibility — the gateway route validates that every
    /// `worker_id` belongs to the cohort (and therefore the tenant)
    /// before invoking this writer.
    ///
    /// The implementation does a single SELECT to enumerate the
    /// existing `(worker_id, skill_id)` pairs, then issues per-row
    /// `INSERT … ON CONFLICT DO UPDATE` statements inside a single
    /// transaction. The upsert keeps the writer compatible with the
    /// existing single-worker path (`add_worker_skill`) so a
    /// re-application of the same proficiency stays an idempotent
    /// no-op. Empty `worker_ids` is a no-op (returns zero counts).
    pub async fn bulk_assign_skill_to_workers(
        &self,
        worker_ids: &[WorkerId],
        skill_id: &str,
        proficiency: i64,
    ) -> ArgoResult<BulkAssignSkillOutcome> {
        if worker_ids.is_empty() {
            return Ok(BulkAssignSkillOutcome::default());
        }
        self.ensure_schema().await?;

        let id_strings: Vec<String> = worker_ids.iter().map(WorkerId::to_string).collect();
        let placeholders = id_strings.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let select_sql = format!(
            "SELECT worker_id FROM worker_skill \
             WHERE skill_id = ? AND worker_id IN ({placeholders})"
        );
        let mut select = sqlx::query_scalar::<_, String>(&select_sql).bind(skill_id);
        for id in &id_strings {
            select = select.bind(id);
        }
        let existing: HashSet<String> = select
            .fetch_all(self.store.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("bulk_assign_skill: select existing: {e}")))?
            .into_iter()
            .collect();

        let mut tx = self
            .store
            .pool()
            .begin()
            .await
            .map_err(|e| ArgoError::Provider(format!("bulk_assign_skill: begin: {e}")))?;
        for id in &id_strings {
            sqlx::query(
                "INSERT INTO worker_skill (worker_id, skill_id, proficiency) VALUES (?, ?, ?)
                 ON CONFLICT(worker_id, skill_id) DO UPDATE SET proficiency = excluded.proficiency",
            )
            .bind(id)
            .bind(skill_id)
            .bind(proficiency)
            .execute(&mut *tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("bulk_assign_skill: upsert: {e}")))?;
        }
        tx.commit()
            .await
            .map_err(|e| ArgoError::Provider(format!("bulk_assign_skill: commit: {e}")))?;

        let already_had = existing.len();
        let assigned = id_strings.len().saturating_sub(already_had);
        Ok(BulkAssignSkillOutcome {
            assigned,
            already_had,
        })
    }

    /// Skill ids the workers in `tenant_id` have learned since
    /// `since_iso` (RFC3339 UTC). Backs the marketplace "Skills my
    /// workers learned this month" filter — the gateway picks the
    /// cutoff (first-of-month UTC) and this read returns the set.
    pub async fn skill_ids_learned_since(
        &self,
        tenant_id: TenantId,
        since_iso: &str,
    ) -> ArgoResult<Vec<String>> {
        self.ensure_schema().await?;
        let rows: Vec<String> = sqlx::query_scalar(
            "SELECT DISTINCT ws.skill_id FROM worker_skill ws
             JOIN worker w ON w.id = ws.worker_id
             WHERE w.tenant_id = ? AND ws.created_at >= ?
             ORDER BY ws.skill_id ASC",
        )
        .bind(tenant_id.to_string())
        .bind(since_iso)
        .fetch_all(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("skill_ids_learned_since: {e}")))?;
        Ok(rows)
    }
}
