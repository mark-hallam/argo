//! `argo tenant quota clear`.

use crate::cli::cli_tenant_cmds::cli_tenant_quota_cmds::TenantQuotaFieldArg;
use argo_store::tenant_quota::{TenantQuotaField, TenantQuotaStore};
use argo_types::ArgoResult;
use chrono::Utc;
use serde_json::json;

pub async fn run(
    tenant: &str,
    field: Option<TenantQuotaFieldArg>,
    json_out: bool,
) -> ArgoResult<()> {
    super::tenant_quota_set::validate_tenant(tenant)?;
    let store = super::tenant_quota_set::open_store().await?;
    let field = field.map(field_to_store);
    let summary = TenantQuotaStore::new(store.pool())
        .clear(tenant, field, Utc::now())
        .await?;
    if json_out {
        super::tenant_quota_set::print_json(&json!({
            "ok": true,
            "tenant_quota_clear": {
                "tenant_id": tenant,
                "field": field,
                "summary": summary
            }
        }))
    } else {
        super::tenant_quota_set::print_summary("cleared", &summary);
        Ok(())
    }
}

fn field_to_store(field: TenantQuotaFieldArg) -> TenantQuotaField {
    match field {
        TenantQuotaFieldArg::Usd => TenantQuotaField::MaxUsd,
        TenantQuotaFieldArg::Actions => TenantQuotaField::MaxActions,
        TenantQuotaFieldArg::Workers => TenantQuotaField::MaxWorkers,
    }
}
