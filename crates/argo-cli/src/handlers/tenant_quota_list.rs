//! `argo tenant quota list`.

use argo_store::tenant_quota::TenantQuotaStore;
use argo_types::ArgoResult;
use chrono::Utc;
use serde_json::json;

pub async fn run(json_out: bool) -> ArgoResult<()> {
    let store = super::tenant_quota_set::open_store().await?;
    let rows = TenantQuotaStore::new(store.pool()).list(Utc::now()).await?;
    if json_out {
        super::tenant_quota_set::print_json(&json!({"ok": true, "tenant_quotas": rows}))
    } else {
        if rows.is_empty() {
            println!("(no tenant quotas)");
            return Ok(());
        }
        for row in rows {
            super::tenant_quota_set::print_summary("tenant quota", &row);
        }
        Ok(())
    }
}
