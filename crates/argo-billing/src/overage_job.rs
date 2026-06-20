//! DEC-053 overage-report **job** — the production caller for
//! [`crate::overage::report_overage_usage`]
//! (`pricing-wire-overage-report-and-cap-enforcement`, part a).
//!
//! [`crate::overage`] owns the pure money-math + the single-tenant Stripe
//! push. This module owns the *batch*: turn a snapshot of every Pro
//! tenant's live action meter into the set of usage records to push, then
//! push them. Splitting the planning out of the I/O keeps the
//! "(used − allowance), exactly once" invariant unit-testable with no
//! Stripe round-trip (the CLI handler in `argo-cli` supplies the meter
//! readings + the subscription item).
//!
//! # Idempotency
//!
//! Each push uses [`crate::overage::report_overage_usage`]'s cumulative
//! `action=set` + stable `(tenant_id, period_start)` `Idempotency-Key`, so
//! re-running the whole job re-asserts the same per-tenant totals — a
//! no-op, never a double-bill. [`plan_overage_reports`] is pure, so the
//! same meter snapshot always plans the identical set.

use crate::overage::{est_overage_cost_micros, overage_units, report_overage_usage};
use crate::stripe_client::{StripeClient, StripeError};

/// One Pro tenant's period-to-date meter reading.
#[derive(Debug, Clone, PartialEq)]
pub struct TenantOverageMeter {
    /// Tenant whose overage is being reported.
    pub tenant_id: String,
    /// Period-to-date actions used (the live `tenant_usage` total).
    pub used_actions: u64,
    /// Actions bundled into the flat tier price (`included_actions`).
    pub included_actions: u64,
    /// Period-to-date **real provider dollar-spend** in USD micros (the
    /// DEC-038 `worker_budget` ledger, surfaced on `TenantUsage`). This is
    /// what makes the bill reflect actual token cost, not a flat per-action
    /// count — a 50-token and a 50k-token action no longer bill identically.
    pub used_usd_micros: u64,
    /// The Pro tier's `overage_multiplier` (e.g. `1.10` = cost + 10% markup).
    pub multiplier: f64,
}

/// A planned per-tenant usage record: the cumulative period-to-date overage
/// **in US cents** to `set` on the Stripe metered price for this tenant. Only
/// tenants whose real overage rounds to ≥ 1 cent appear.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedOverageReport {
    /// Tenant the record is for.
    pub tenant_id: String,
    /// Cumulative period-to-date overage in **US cents** — real provider
    /// dollar-spend beyond the bundled allowance × the tier markup. The
    /// Stripe metered price MUST be configured at **$0.01 per unit** so the
    /// reported quantity (cents) bills as dollars. (Was: a flat action count.)
    pub overage_cents: u64,
}

/// Plan the per-tenant usage records from a meter snapshot. Pure: tenants
/// at or under their allowance (or whose overage rounds below 1 cent) are
/// dropped — nothing to bill. The billed quantity is **real dollar-spend
/// overage × markup, in cents** (DEC-053 + the 10% managed-key markup), not a
/// flat action count, so heavy-token and light-token actions bill honestly.
/// Deterministic, so re-planning the same snapshot yields the identical set
/// (idempotent by construction; Stripe `action=set` collapses re-runs).
#[must_use]
pub fn plan_overage_reports(meters: &[TenantOverageMeter]) -> Vec<PlannedOverageReport> {
    meters
        .iter()
        .filter_map(|m| {
            let units = overage_units(m.used_actions, m.included_actions);
            let micros =
                est_overage_cost_micros(units, m.used_actions, m.used_usd_micros, m.multiplier);
            // 1 cent = 10_000 USD micros; round to the nearest cent.
            let cents = (micros + 5_000) / 10_000;
            (cents > 0).then(|| PlannedOverageReport {
                tenant_id: m.tenant_id.clone(),
                overage_cents: cents,
            })
        })
        .collect()
}

/// Outcome of running an overage-report batch.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OverageRunSummary {
    /// Tenants with overage that were pushed to Stripe.
    pub reported: usize,
    /// Tenants planned but skipped on a transport error (best-effort:
    /// one tenant's failure must not abort the rest of the batch).
    pub failed: usize,
}

/// Push every planned report to the Stripe metered price's
/// `subscription_item_id`. Best-effort per tenant — a single transport
/// failure is counted and logged, never fatal to the batch, because the
/// stable idempotency key makes the next run safely re-push the same
/// total. `period_start` keys the idempotency, `timestamp` stamps the
/// usage record.
///
/// # Errors
/// Never returns `Err` for an individual tenant push (those are tallied
/// in [`OverageRunSummary::failed`]); the `Result` is reserved for future
/// fatal setup errors and currently always `Ok`.
pub async fn run_overage_reports(
    client: &StripeClient,
    subscription_item_id: &str,
    reports: &[PlannedOverageReport],
    period_start: i64,
    timestamp: i64,
) -> Result<OverageRunSummary, StripeError> {
    let mut summary = OverageRunSummary::default();
    for report in reports {
        match report_overage_usage(
            client,
            subscription_item_id,
            &report.tenant_id,
            period_start,
            report.overage_cents,
            timestamp,
        )
        .await
        {
            Ok(_) => summary.reported += 1,
            Err(e) => {
                summary.failed += 1;
                tracing::warn!(
                    target: "billing::overage_job",
                    tenant_id = %report.tenant_id,
                    error = %e,
                    "overage usage-record push failed; will retry idempotently next run",
                );
            }
        }
    }
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a meter with a fixed average action cost of `$0.001` (1_000
    /// micros/action) and the 10% Pro markup, so the expected cents are
    /// `overage_units × 1_000 × 1.10 / 10_000` = `overage_units × 0.11`.
    fn meter(tenant: &str, used: u64, included: u64) -> TenantOverageMeter {
        TenantOverageMeter {
            tenant_id: tenant.into(),
            used_actions: used,
            included_actions: included,
            used_usd_micros: used.saturating_mul(1_000), // $0.001/action avg
            multiplier: 1.10,
        }
    }

    #[test]
    fn bills_real_dollar_overage_in_cents_not_a_flat_action_count() {
        // 2000 used, 1500 included → 500 overage actions; avg $0.001/action;
        // ×1.10 → 500 × 1_000 × 1.10 = 550_000 micros = $0.55 = 55 cents.
        let plan = plan_overage_reports(&[meter("t-over", 2_000, 1_500)]);
        assert_eq!(
            plan,
            vec![PlannedOverageReport {
                tenant_id: "t-over".into(),
                overage_cents: 55,
            }]
        );
    }

    #[test]
    fn token_heavy_actions_bill_more_than_token_light_ones() {
        // Same 500 overage actions, but one tenant's actions cost 10× more in
        // real provider spend — it must bill ~10× more, NOT identically (the
        // exact bug the flat action-count model had).
        let light = TenantOverageMeter {
            tenant_id: "t-light".into(),
            used_actions: 2_000,
            included_actions: 1_500,
            used_usd_micros: 2_000 * 1_000, // $0.001/action
            multiplier: 1.10,
        };
        let heavy = TenantOverageMeter {
            tenant_id: "t-heavy".into(),
            used_actions: 2_000,
            included_actions: 1_500,
            used_usd_micros: 2_000 * 10_000, // $0.01/action (10× pricier)
            multiplier: 1.10,
        };
        let plan = plan_overage_reports(&[light, heavy]);
        assert_eq!(plan[0].overage_cents, 55); // 500 × 1_000 × 1.10 → 55¢
        assert_eq!(plan[1].overage_cents, 550); // 500 × 10_000 × 1.10 → $5.50
    }

    #[test]
    fn drops_tenants_at_or_under_allowance() {
        let plan = plan_overage_reports(&[
            meter("t-under", 1_000, 1_500),
            meter("t-exact", 1_500, 1_500),
        ]);
        assert!(plan.is_empty(), "no overage → nothing to push");
    }

    #[test]
    fn plan_is_deterministic_so_reruns_are_idempotent() {
        // The same snapshot plans the identical set every time — a re-run
        // pushes the same cumulative cents, so Stripe's `action=set` collapses
        // it to one record.
        let meters = [
            meter("t-a", 2_000, 1_500), // 500 over → 55¢
            meter("t-b", 1_400, 1_500), // under → dropped
            meter("t-c", 3_100, 1_500), // 1_600 over → 1_600×1_000×1.10 = $1.76
        ];
        let first = plan_overage_reports(&meters);
        let again = plan_overage_reports(&meters);
        assert_eq!(first, again);
        assert_eq!(
            first,
            vec![
                PlannedOverageReport {
                    tenant_id: "t-a".into(),
                    overage_cents: 55
                },
                PlannedOverageReport {
                    tenant_id: "t-c".into(),
                    overage_cents: 176
                },
            ]
        );
    }
}
