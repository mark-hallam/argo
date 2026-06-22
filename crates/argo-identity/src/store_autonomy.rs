//! `IdentityStore::update_auto_approval` lives in its own file so the
//! parent [`store`](crate::store) module stays under the 500-LOC
//! ceiling. Mirrors `update_channels` in shape: serialize the policy,
//! write the whole `auto_approval_json` blob back for the (tenant,
//! worker) row. Used by the gateway autonomy-level PATCH (DEC-060) to
//! make a chosen level real — the duty gate reads this policy via
//! [`crate::Worker::resolved_policy`].

use crate::{store::IdentityStore, AutoApprovalPolicy, WorkerId};
use argo_types::{ArgoError, ArgoResult, TenantId};

impl IdentityStore<'_> {
    /// Replace the worker's auto-approval policy (the per-worker spend
    /// envelope the duty gate enforces). Returns `ArgoError::Provider`
    /// when the (`tenant_id`, `worker_id`) pair does not match a row.
    ///
    /// ```no_run
    /// # use argo_identity::{IdentityStore, AutoApprovalPolicy, WorkerId};
    /// # use argo_types::TenantId;
    /// # async fn ex(store: &argo_store::SessionStore, t: TenantId, w: WorkerId) {
    /// let ids = IdentityStore::new(store);
    /// ids.update_auto_approval(t, w, &AutoApprovalPolicy::default()).await.ok();
    /// # }
    /// ```
    pub async fn update_auto_approval(
        &self,
        tenant_id: TenantId,
        worker_id: WorkerId,
        policy: &AutoApprovalPolicy,
    ) -> ArgoResult<()> {
        self.ensure_schema().await?;
        let json = serde_json::to_string(policy)
            .map_err(|e| ArgoError::Provider(format!("serialize auto_approval: {e}")))?;
        let result =
            sqlx::query("UPDATE worker SET auto_approval_json = ? WHERE id = ? AND tenant_id = ?")
                .bind(json)
                .bind(worker_id.to_string())
                .bind(tenant_id.to_string())
                .execute(self.store.pool())
                .await
                .map_err(|e| ArgoError::Provider(format!("update auto_approval: {e}")))?;
        if result.rows_affected() == 0 {
            return Err(ArgoError::Provider(format!(
                "worker {worker_id} not found in tenant {tenant_id}"
            )));
        }
        Ok(())
    }
}
