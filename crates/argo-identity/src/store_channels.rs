//! `IdentityStore::update_channels` lives in its own file so the parent
//! [`store`](crate::store) module stays under the 500-LOC ceiling.
//! Mirrors `update_charter` from #964 in shape (read in the caller,
//! mutate the slice, then write the whole vector back).

use crate::{store::IdentityStore, NotifyChannelRef};
use argo_types::{ArgoError, ArgoResult, TenantId};

use crate::WorkerId;

impl IdentityStore<'_> {
    /// Replace the worker's notification channel list (`Stream
    /// claude-worker-channels-cli`). Returns `ArgoError::Provider` when
    /// the (`tenant_id`, `worker_id`) pair does not match a stored row.
    pub async fn update_channels(
        &self,
        tenant_id: TenantId,
        worker_id: WorkerId,
        channels: &[NotifyChannelRef],
    ) -> ArgoResult<()> {
        self.ensure_schema().await?;
        let json = serde_json::to_string(channels)
            .map_err(|e| ArgoError::Provider(format!("serialize channels: {e}")))?;
        let result =
            sqlx::query("UPDATE worker SET channels_json = ? WHERE id = ? AND tenant_id = ?")
                .bind(json)
                .bind(worker_id.to_string())
                .bind(tenant_id.to_string())
                .execute(self.store.pool())
                .await
                .map_err(|e| ArgoError::Provider(format!("update channels: {e}")))?;
        if result.rows_affected() == 0 {
            return Err(ArgoError::Provider(format!(
                "worker {worker_id} not found in tenant {tenant_id}"
            )));
        }
        Ok(())
    }
}
