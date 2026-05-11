//! `IdentityStore::archive_worker` lives in its own file so the parent
//! [`store`](crate::store) module stays under the 500-LOC ceiling.
//! Name disambiguates from the sibling `store_archive_tests.rs` which
//! holds the matching test coverage.

use crate::store::IdentityStore;
use crate::WorkerId;
use argo_types::{ArgoError, ArgoResult, TenantId};

impl IdentityStore<'_> {
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
}
