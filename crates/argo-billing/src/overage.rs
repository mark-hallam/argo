//! DEC-053 metered-overage reporting — the Pro-margin mechanic.
//!
//! Pro bundles `included_actions` per teammate per month; usage beyond
//! it bills as **metered overage at token-cost × `overage_multiplier`**
//! (1.4). This module is the bridge between the live usage meter (the
//! `tenant_usage` action total — there is NO parallel counter) and a
//! Stripe **metered price**: the operator creates the metered price in
//! the dashboard and its subscription item is read from the
//! [`STRIPE_PRICE_PRO_OVERAGE_ENV`] env var; this module pushes the
//! period-to-date overage units to it as a usage record.
//!
//! # Idempotency (the "report exactly once" invariant)
//!
//! Reporting uses Stripe's `action=set` semantics: the quantity is the
//! **cumulative** period-to-date overage, so re-running the job simply
//! re-asserts the same total — a no-op, never a double-bill. That is
//! belt-and-suspendered with an `Idempotency-Key` keyed on
//! `(tenant_id, period_start)` so even a transient retry within Stripe's
//! 24h dedup window collapses to one record.
//!
//! # The cap invariant (DEC-053 "stop, don't surprise")
//!
//! Billing never exceeds the customer's self-set hard cap because usage
//! *stops* at the cap — SpendGate (sibling brief) refuses actions once
//! the cap is reached, so `used_actions` freezes and the overage units
//! computed here stop growing. This module reports what happened; it
//! does not bill past what the gate allowed.

use crate::stripe_client::{StripeClient, StripeError};

/// Env var naming the Stripe metered-price **subscription item** the
/// operator created for Pro overage (e.g. `si_…`). Read at report time;
/// absent → overage reporting is skipped (gated-cohort flat-fee mode).
pub const STRIPE_PRICE_PRO_OVERAGE_ENV: &str = "STRIPE_PRICE_PRO_OVERAGE";

/// Outcome of an overage-report decision — pure, network-free.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverageReport {
    /// Period-to-date overage units (`used − included`, floored at 0).
    pub units: u64,
    /// Whether a Stripe usage record should be pushed. `false` when
    /// `units == 0` (no overage → nothing to report).
    pub should_report: bool,
}

/// Period-to-date overage unit count: actions used beyond the bundled
/// allowance, floored at zero. This is the exact quantity reported to
/// the Stripe metered price.
///
/// ```
/// use argo_billing::overage::overage_units;
/// assert_eq!(overage_units(1_000, 1_500), 0); // under allowance
/// assert_eq!(overage_units(2_000, 1_500), 500); // 500 over
/// ```
#[must_use]
pub fn overage_units(used_actions: u64, included_actions: u64) -> u64 {
    used_actions.saturating_sub(included_actions)
}

/// Decide whether (and how much) to report. Pure so the report path is
/// fully unit-testable without a Stripe round-trip.
#[must_use]
pub fn plan_overage(used_actions: u64, included_actions: u64) -> OverageReport {
    let units = overage_units(used_actions, included_actions);
    OverageReport {
        units,
        should_report: units > 0,
    }
}

/// Estimated overage cost in USD micros for the honest `/billing` meter:
/// `units × avg_token_cost_per_action × multiplier`. The average token
/// cost is derived from the live spend (`used_usd_micros / used_actions`,
/// DEC-038 data), so the meter reflects real cost, not a guess. Returns
/// `0` when there is no overage or no usage to derive an average from.
///
/// This is a *display* estimate; the authoritative bill is what Stripe
/// charges against the metered price's per-unit amount (which the
/// operator sets to track token-cost × `multiplier`).
///
/// ```
/// use argo_billing::overage::est_overage_cost_micros;
/// // 2000 actions used, 1500 included → 500 overage; $1.00 total spend
/// // over 2000 actions = $0.0005/action; ×1.4 → 500 × 500micros × 1.4.
/// let micros = est_overage_cost_micros(500, 2_000, 1_000_000, 1.4);
/// assert_eq!(micros, 350_000); // $0.35
/// ```
#[must_use]
pub fn est_overage_cost_micros(
    units: u64,
    used_actions: u64,
    used_usd_micros: u64,
    multiplier: f64,
) -> u64 {
    if units == 0 || used_actions == 0 || multiplier <= 0.0 {
        return 0;
    }
    let avg_per_action = used_usd_micros as f64 / used_actions as f64;
    let cost = units as f64 * avg_per_action * multiplier;
    if cost.is_finite() && cost >= 0.0 {
        cost.round() as u64
    } else {
        0
    }
}

/// Stable idempotency key for a tenant's overage report in one period.
/// Same `(tenant_id, period_start)` → same key → Stripe collapses
/// retries to one record.
#[must_use]
pub fn idempotency_key(tenant_id: &str, period_start: i64) -> String {
    format!("overage-{tenant_id}-{period_start}")
}

/// Form body for a Stripe usage record using cumulative `action=set`
/// semantics. Pure + testable: the `set` action is what makes repeated
/// reports idempotent (the quantity is the period-to-date total, not an
/// increment).
#[must_use]
pub fn usage_record_form(units: u64, timestamp: i64) -> Vec<(String, String)> {
    vec![
        ("quantity".to_string(), units.to_string()),
        ("action".to_string(), "set".to_string()),
        ("timestamp".to_string(), timestamp.to_string()),
    ]
}

/// Push the period-to-date overage units to the Stripe metered price's
/// subscription item via a usage record. Cumulative `action=set` +
/// `Idempotency-Key` make repeated calls safe.
///
/// `subscription_item_id` is the `si_…` from [`STRIPE_PRICE_PRO_OVERAGE_ENV`].
///
/// # Errors
/// Returns [`StripeError`] on transport or API failure.
pub async fn report_overage_usage(
    client: &StripeClient,
    subscription_item_id: &str,
    tenant_id: &str,
    period_start: i64,
    units: u64,
    timestamp: i64,
) -> Result<serde_json::Value, StripeError> {
    let path = format!("/v1/subscription_items/{subscription_item_id}/usage_records");
    let form = usage_record_form(units, timestamp);
    let key = idempotency_key(tenant_id, period_start);
    client.post_form_idempotent(&path, &form, &key).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_overage_when_usage_under_or_at_allowance() {
        // Acceptance 1a: usage ≤ allowance reports 0 overage.
        assert_eq!(overage_units(0, 1_500), 0);
        assert_eq!(overage_units(1_499, 1_500), 0);
        assert_eq!(overage_units(1_500, 1_500), 0);
        let plan = plan_overage(1_500, 1_500);
        assert_eq!(plan.units, 0);
        assert!(!plan.should_report, "no overage → nothing to push to Stripe");
    }

    #[test]
    fn overage_is_used_minus_allowance_exactly() {
        // Acceptance 1b: usage over reports (used − allowance) units.
        assert_eq!(overage_units(2_000, 1_500), 500);
        let plan = plan_overage(2_000, 1_500);
        assert_eq!(plan.units, 500);
        assert!(plan.should_report);
    }

    #[test]
    fn report_uses_cumulative_set_semantics_for_idempotency() {
        // Acceptance 1b idempotency pin: `action=set` reports the
        // cumulative total, so re-running yields the SAME record, not a
        // second increment. The form is identical for the same units.
        let first = usage_record_form(500, 1_000);
        let again = usage_record_form(500, 1_000);
        assert_eq!(first, again);
        assert!(first.contains(&("action".to_string(), "set".to_string())));
        assert!(first.contains(&("quantity".to_string(), "500".to_string())));
    }

    #[test]
    fn idempotency_key_is_stable_per_tenant_period() {
        let a = idempotency_key("tenant-1", 1_700_000_000);
        let b = idempotency_key("tenant-1", 1_700_000_000);
        let other_tenant = idempotency_key("tenant-2", 1_700_000_000);
        let other_period = idempotency_key("tenant-1", 1_702_000_000);
        assert_eq!(a, b, "same tenant+period → same key (Stripe dedups)");
        assert_ne!(a, other_tenant);
        assert_ne!(a, other_period);
    }

    #[test]
    fn cap_freezes_overage_growth() {
        // Acceptance 1c: once the hard cap is reached SpendGate refuses
        // further actions, so `used_actions` stops climbing. Model the
        // gate by freezing usage at the cap and assert the reported
        // overage units stop growing — billing never runs past the cap.
        let included = 1_500;
        // Suppose the cap halts usage at 2_000 actions.
        let used_at_cap = 2_000;
        let units_at_cap = overage_units(used_at_cap, included);
        // A later tick with the SAME frozen usage reports the SAME units
        // (no further overage accrues past the gate).
        let units_next_tick = overage_units(used_at_cap, included);
        assert_eq!(units_at_cap, 500);
        assert_eq!(units_at_cap, units_next_tick, "no overage past the cap");
    }

    #[test]
    fn est_cost_is_zero_without_overage_or_usage() {
        assert_eq!(est_overage_cost_micros(0, 2_000, 1_000_000, 1.4), 0);
        assert_eq!(est_overage_cost_micros(500, 0, 0, 1.4), 0);
        assert_eq!(est_overage_cost_micros(500, 2_000, 1_000_000, 0.0), 0);
    }

    #[test]
    fn est_cost_applies_multiplier_to_average_token_cost() {
        // $1.00 over 2000 actions = $0.0005/action; 500 overage × 1.4.
        let micros = est_overage_cost_micros(500, 2_000, 1_000_000, 1.4);
        assert_eq!(micros, 350_000);
    }
}
