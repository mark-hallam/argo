//! `GET /api/v1/billing/meter` + `POST /api/v1/billing/cap`
//! (Stream `pricing-stripe-metered-overage-and-caps`, DEC-053).
//!
//! The honest Pro meter and the customer-visible hard spend cap:
//!
//! - **meter** (read) — `included` (the bundled allowance) vs `used`
//!   (the live action meter, the SAME `tenant_usage` source as
//!   `GET /api/v1/tenant/usage` — NO parallel counter) vs `overage`
//!   (`used − included`, the exact units billed to the Stripe metered
//!   price). The dollar figure is a labelled *estimate* derived from
//!   the per-tenant spend (DEC-038) × the tier's overage multiplier.
//! - **cap** (read on the meter, set via POST) — the per-tenant
//!   `tenant_quota.max_usd` hard cap. SpendGate (sibling brief) stops
//!   usage at the cap, so billing never exceeds it (the DEC-053
//!   "stop, don't surprise-bill" invariant). When no explicit cap is
//!   set the tier's `default_monthly_cap_usd` is surfaced as the
//!   effective default.
//!
//! Both reads/writes resolve the active tenant via the canonical
//! `resolve_tenant::resolve_tenant_for_request` (JWT session first), so
//! a signed-in user only ever sees / sets THEIR tenant's cap. Setting
//! the cap is **owner-only** (`Action::BillingManage`).

use super::role_gate::{enforce_role, Action};
use argo_billing::overage::{est_overage_cost_micros, overage_units};
use argo_identity::tier::{Tier, TierConfig};
use argo_store::tenant_quota::{TenantQuotaStore, TenantQuotaUpdate};
use argo_store::tenant_query::{tenant_tier, tenant_usage_for_period};
use argo_store::SessionStore;
use chrono::Utc;
use serde::Deserialize;
use serde_json::json;
use std::str::FromStr;
use std::sync::Arc;

/// `GET /api/v1/billing/meter` — the honest included/used/overage meter
/// plus the effective hard cap for the resolved tenant.
pub(super) async fn meter_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    if let Some(deny) = enforce_role(req, Action::BillingView) {
        return deny;
    }
    let store = match store {
        Some(s) => s,
        None => return super::store_unavailable(),
    };
    let tenant = match super::resolve_tenant::resolve_tenant_for_request(req, store).await {
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

    // Action meter — the same source `GET /api/v1/tenant/usage` reads.
    let (used_actions, _period_start) = match tenant_usage_for_period(store, tenant, now).await {
        Ok(v) => v,
        Err(e) => return server_error(&format!("usage read: {e}")),
    };
    // Spend + configured cap from the tenant_quota store (DEC-038).
    let tenant_key = tenant.to_string();
    let quotas = TenantQuotaStore::new(store.pool());
    let summary = match quotas.summary(&tenant_key, now).await {
        Ok(s) => s,
        Err(e) => return server_error(&format!("quota read: {e}")),
    };

    let included = limits.included_actions;
    let multiplier = limits.overage_multiplier;
    let units = overage_units(used_actions, included);
    let est_cost_micros = est_overage_cost_micros(
        units,
        used_actions,
        summary.usage.used_usd_micros,
        multiplier,
    );

    // Effective cap: an explicit per-tenant cap wins; otherwise the
    // tier's default_monthly_cap_usd is the surfaced default.
    let explicit_cap_usd = summary
        .quota
        .as_ref()
        .and_then(|q| q.max_usd_micros)
        .map(micros_to_usd);
    let default_cap_usd = if limits.default_monthly_cap_usd > 0 {
        Some(f64::from(limits.default_monthly_cap_usd))
    } else {
        None
    };
    let cap_is_default = explicit_cap_usd.is_none();
    let effective_cap_usd = explicit_cap_usd.or(default_cap_usd);

    let body = json!({
        "tier": tier.as_str(),
        "display_name": limits.display_name,
        "included_actions": included,
        "used_actions": used_actions,
        "overage_units": units,
        "overage_multiplier": multiplier,
        "estimated_overage_usd": micros_to_usd(est_cost_micros),
        "cap_usd": effective_cap_usd,
        "cap_is_default": cap_is_default,
    });
    ("200 OK", body.to_string())
}

/// `POST /api/v1/billing/cap` — set the customer's self-chosen hard
/// monthly spend cap (owner-only). Body: `{ "cap_usd": <number ≥ 0> }`.
pub(super) async fn set_cap_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    if let Some(deny) = enforce_role(req, Action::BillingManage) {
        return deny;
    }
    let store = match store {
        Some(s) => s,
        None => return super::store_unavailable(),
    };
    let body = super::body_from_request(req);
    let payload: CapPayload = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => return bad_request(&format!("cap body decode: {e}")),
    };
    if !(payload.cap_usd.is_finite() && payload.cap_usd >= 0.0) {
        return bad_request("cap_usd must be a finite number ≥ 0");
    }
    let tenant = match super::resolve_tenant::resolve_tenant_for_request(req, store).await {
        Ok(t) => t,
        Err(e) => return server_error(&format!("resolve tenant: {e}")),
    };
    let tenant_key = tenant.to_string();
    let max_usd_micros = (payload.cap_usd * 1_000_000.0).round() as u64;
    let quotas = TenantQuotaStore::new(store.pool());
    let update = TenantQuotaUpdate {
        max_usd_micros: Some(max_usd_micros),
        ..Default::default()
    };
    match quotas.set_quota(&tenant_key, update, Utc::now()).await {
        Ok(summary) => {
            let cap = summary
                .quota
                .as_ref()
                .and_then(|q| q.max_usd_micros)
                .map(micros_to_usd);
            ("200 OK", json!({ "cap_usd": cap }).to_string())
        }
        Err(e) => server_error(&format!("cap set: {e}")),
    }
}

#[derive(Debug, Deserialize)]
struct CapPayload {
    cap_usd: f64,
}

fn micros_to_usd(micros: u64) -> f64 {
    micros as f64 / 1_000_000.0
}

fn bad_request(message: &str) -> (&'static str, String) {
    (
        "400 Bad Request",
        json!({ "error": { "code": "bad_request", "message": message } }).to_string(),
    )
}

fn server_error(message: &str) -> (&'static str, String) {
    (
        "500 Internal Server Error",
        json!({ "error": { "code": "internal_error", "message": message } }).to_string(),
    )
}

#[cfg(test)]
#[path = "billing_meter_tests.rs"]
mod tests;
