//! `argo tenant quota show`.

use argo_store::tenant_quota::TenantQuotaStore;
use argo_types::ArgoResult;
use chrono::Utc;
use serde_json::json;

pub async fn run(tenant: &str, json_out: bool) -> ArgoResult<()> {
    super::tenant_quota_set::validate_tenant(tenant)?;
    let store = super::tenant_quota_set::open_store().await?;
    let summary = TenantQuotaStore::new(store.pool())
        .summary(tenant, Utc::now())
        .await?;
    if json_out {
        super::tenant_quota_set::print_json(&json!({"ok": true, "tenant_quota": summary}))
    } else {
        super::tenant_quota_set::print_summary("show", &summary);
        Ok(())
    }
}
