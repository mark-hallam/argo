//! Worker `role_name` mutator + lookup for the `IdentityStore`
//! (Stream `mission-routing-skillmatch-rolematch`, Tier M4). Sibling
//! to [`store`](crate::store) so the production file stays under the
//! 500-LOC ceiling — mirrors [`store_skills`](crate::store_skills)
//! and [`store_archive_methods`](crate::store_archive_methods).
//!
//! The two methods underpin
//! `TargetSelector::RoleMatch(String)` resolution in the routing
//! engine: operators stamp workers with a role-name string via the
//! `argo worker onboard --role-name <s>` CLI flag, and the routing
//! engine queries [`IdentityStore::workers_with_role`] to find
//! candidates before ranking them by `trust_grade DESC,
//! last_used_at DESC, worker_id ASC`.

use crate::store::IdentityStore;
use crate::store_sql::parse_worker_id;
use crate::{Worker, WorkerId};
use argo_types::{ArgoError, ArgoResult, TenantId};

impl IdentityStore<'_> {
    /// Stamp `worker_id` with `role_name`. Passing `None` clears the
    /// stamp. Idempotent — repeated calls with the same value are
    /// no-ops at the row level.
    ///
    /// Tenant scoping is enforced by the SQL `WHERE tenant_id = ?`
    /// clause; mismatched `(tenant, worker)` pairs return
    /// [`ArgoError::NotFound`] so callers can distinguish "row absent"
    /// from "row updated".
    pub async fn set_worker_role_name(
        &self,
        tenant_id: TenantId,
        worker_id: WorkerId,
        role_name: Option<&str>,
    ) -> ArgoResult<()> {
        self.ensure_schema().await?;
        let result = sqlx::query(
            "UPDATE worker SET role_name = ? WHERE id = ? AND tenant_id = ?",
        )
        .bind(role_name)
        .bind(worker_id.to_string())
        .bind(tenant_id.to_string())
        .execute(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("update worker.role_name: {e}")))?;
        if result.rows_affected() == 0 {
            return Err(ArgoError::NotFound(format!(
                "worker {worker_id} in tenant {tenant_id}"
            )));
        }
        Ok(())
    }

    /// Workers in `tenant_id` whose `role_name` exactly matches
    /// `role_name`, newest-first by `created_at`. Used by
    /// `mission-routing-skillmatch-rolematch` (M4) to resolve
    /// `TargetSelector::RoleMatch(String)`. Empty result is normal
    /// in v1 because most operators won't have populated the column.
    pub async fn workers_with_role(
        &self,
        tenant_id: TenantId,
        role_name: &str,
    ) -> ArgoResult<Vec<Worker>> {
        self.ensure_schema().await?;
        let ids: Vec<String> = sqlx::query_scalar(
            "SELECT id FROM worker \
             WHERE tenant_id = ? AND role_name = ? AND archived_at IS NULL \
             ORDER BY created_at DESC, id ASC",
        )
        .bind(tenant_id.to_string())
        .bind(role_name)
        .fetch_all(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("workers_with_role: {e}")))?;

        let mut workers = Vec::with_capacity(ids.len());
        for id in ids {
            let worker_id = parse_worker_id(&id)?;
            if let Some(worker) = self.get_worker(tenant_id, worker_id).await? {
                workers.push(worker);
            }
        }
        Ok(workers)
    }
}

#[cfg(test)]
#[path = "store_role_tests.rs"]
mod store_role_tests;
