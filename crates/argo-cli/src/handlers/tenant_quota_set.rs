//! `argo tenant quota set`.

use crate::cli::cli_tenant_cmds::cli_tenant_quota_cmds::{TenantQuotaCmd, TenantQuotaPeriodArg};
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_identity::IdentityStore;
use argo_store::tenant_quota::{
    QuotaPeriod, TenantQuotaStore, TenantQuotaSummary, TenantQuotaUpdate,
};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use chrono::Utc;
use serde::Serialize;
use serde_json::json;

pub async fn run_cmd(cmd: TenantQuotaCmd) -> ArgoResult<()> {
    match cmd {
        TenantQuotaCmd::Set {
            tenant,
            max_usd,
            max_actions,
            max_workers,
            period,
            json,
        } => run(&tenant, max_usd, max_actions, max_workers, period, json).await,
        TenantQuotaCmd::Show { tenant, json } => super::tenant_quota_show::run(&tenant, json).await,
        TenantQuotaCmd::List { json } => super::tenant_quota_list::run(json).await,
        TenantQuotaCmd::Clear {
            tenant,
            field,
            json,
        } => super::tenant_quota_clear::run(&tenant, field, json).await,
    }
}

pub async fn run(
    tenant: &str,
    max_usd: Option<f64>,
    max_actions: Option<u64>,
    max_workers: Option<u64>,
    period: Option<TenantQuotaPeriodArg>,
    json_out: bool,
) -> ArgoResult<()> {
    validate_tenant(tenant)?;
    let update = TenantQuotaUpdate {
        period: period.map(period_to_store),
        max_usd_micros: max_usd.map(usd_to_micros).transpose()?,
        max_actions,
        max_workers,
    };
    if update.period.is_none()
        && update.max_usd_micros.is_none()
        && update.max_actions.is_none()
        && update.max_workers.is_none()
    {
        return Err(ArgoError::Provider(
            "tenant quota set requires --period or at least one cap".into(),
        ));
    }
    let store = open_store().await?;
    let summary = TenantQuotaStore::new(store.pool())
        .set_quota(tenant, update, Utc::now())
        .await?;
    if json_out {
        print_json(&json!({"ok": true, "tenant_quota": summary}))
    } else {
        print_summary("set", &summary);
        Ok(())
    }
}

pub(crate) async fn open_store() -> ArgoResult<SessionStore> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(db_path(&home)).await?;
    store.migrate().await?;
    IdentityStore::new(&store).ensure_schema().await?;
    Ok(store)
}

pub(crate) fn validate_tenant(tenant: &str) -> ArgoResult<()> {
    if tenant.trim().is_empty() {
        return Err(ArgoError::Provider("tenant quota: empty tenant id".into()));
    }
    Ok(())
}

pub(crate) fn period_to_store(period: TenantQuotaPeriodArg) -> QuotaPeriod {
    match period {
        TenantQuotaPeriodArg::Daily => QuotaPeriod::Daily,
        TenantQuotaPeriodArg::Monthly => QuotaPeriod::Monthly,
    }
}

pub(crate) fn usd_to_micros(value: f64) -> ArgoResult<u64> {
    if !value.is_finite() || value < 0.0 {
        return Err(ArgoError::Provider(format!(
            "--max-usd must be a non-negative finite number, got {value}"
        )));
    }
    let micros = (value * 1_000_000.0).round();
    if micros > u64::MAX as f64 {
        return Err(ArgoError::Provider(format!(
            "--max-usd is too large: {value}"
        )));
    }
    Ok(micros as u64)
}

pub(crate) fn print_json<T: Serialize>(value: &T) -> ArgoResult<()> {
    let body = serde_json::to_string_pretty(value)
        .map_err(|e| ArgoError::Provider(format!("serialize tenant quota JSON: {e}")))?;
    println!("{body}");
    Ok(())
}

pub(crate) fn print_summary(prefix: &str, summary: &TenantQuotaSummary) {
    println!("{prefix} tenant quota {}", summary.tenant_id);
    if let Some(quota) = &summary.quota {
        println!("  period:       {}", quota.period.as_str());
        println!("  max_usd:      {}", fmt_limit_usd(quota.max_usd_micros));
        println!("  max_actions:  {}", fmt_limit(quota.max_actions));
        println!("  max_workers:  {}", fmt_limit(quota.max_workers));
    } else {
        println!("  quota:        none");
    }
    println!("  used_usd:     {:.6}", summary.usage.used_usd);
    println!("  used_actions: {}", summary.usage.used_actions);
    println!("  used_workers: {}", summary.usage.used_workers);
}

fn fmt_limit(value: Option<u64>) -> String {
    value.map_or_else(|| "-".to_string(), |v| v.to_string())
}

fn fmt_limit_usd(value: Option<u64>) -> String {
    value.map_or_else(
        || "-".to_string(),
        |v| format!("{:.6}", v as f64 / 1_000_000.0),
    )
}
