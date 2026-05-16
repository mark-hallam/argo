//! `GET /api/v1/tenant/usage` — return the active pricing tier, the
//! period-to-date action count, the monthly cap, and the next reset
//! timestamp (Stream `claude-pricing-plans-config`).
//!
//! The endpoint reads from `workspace.tier` + the `tenant_usage`
//! table (both bootstrapped lazily by
//! `argo_store::tenant_query::ensure_pricing_schema`). Bearer-auth
//! is gated upstream by `routes::dispatch_route`.

use argo_identity::tier::{Tier, TierConfig};
use argo_store::tenant_query::{tenant_tier, tenant_usage_for_period};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use chrono::{DateTime, Datelike, NaiveDate, TimeZone, Utc};
use serde_json::json;
use std::str::FromStr;
use std::sync::Arc;

/// `GET /api/v1/tenant/usage` entry point.
pub(super) async fn show_response(
    _req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let store = match store {
        Some(s) => s,
        None => return super::store_unavailable(),
    };
    let tenant = match resolve_workspace_tenant(store).await {
        Ok(t) => t,
        Err(e) => return server_error(&format!("resolve tenant: {e}")),
    };
    let now = Utc::now();
    let tier_label = match tenant_tier(store, tenant).await {
        Ok(s) => s,
        Err(e) => return server_error(&format!("tier read: {e}")),
    };
    let tier = Tier::from_str(&tier_label).unwrap_or(Tier::Free);
    let config = TierConfig::default_config();
    let Some(limits) = config.limits(tier) else {
        return server_error(&format!("tier '{tier_label}' missing from v1-tiers.toml"));
    };
    let (used, _period_start) = match tenant_usage_for_period(store, tenant, now).await {
        Ok(v) => v,
        Err(e) => return server_error(&format!("usage read: {e}")),
    };
    let limit = limits.max_actions_per_month;
    let remaining = if limit == 0 {
        None
    } else {
        Some(limit.saturating_sub(used))
    };
    let resets_at = next_month_start_rfc3339(now);
    let body = json!({
        "tier": tier.as_str(),
        "display_name": limits.display_name,
        "used": used,
        "limit": limit,
        "remaining": remaining,
        "resets_at": resets_at,
    });
    ("200 OK", body.to_string())
}

async fn resolve_workspace_tenant(store: &SessionStore) -> ArgoResult<TenantId> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let name = cwd.file_name().and_then(|s| s.to_str()).unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (tenant, _ws) = store.ensure_workspace(name, &root).await?;
    Ok(tenant)
}

fn next_month_start_rfc3339(now: DateTime<Utc>) -> String {
    let (year, month) = if now.month() == 12 {
        (now.year() + 1, 1)
    } else {
        (now.year(), now.month() + 1)
    };
    NaiveDate::from_ymd_opt(year, month, 1)
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .and_then(|naive| Utc.from_local_datetime(&naive).single())
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_else(|| now.to_rfc3339())
}

fn server_error(message: &str) -> (&'static str, String) {
    (
        "500 Internal Server Error",
        json!({ "error": { "code": "internal_error", "message": message } }).to_string(),
    )
}

#[cfg(test)]
#[path = "usage_endpoint_tests.rs"]
mod tests;
