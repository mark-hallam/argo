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

use crate::overage::{overage_units, report_overage_usage};
use crate::stripe_client::{StripeClient, StripeError};

/// One Pro tenant's period-to-date meter reading.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TenantOverageMeter {
    /// Tenant whose overage is being reported.
    pub tenant_id: String,
    /// Period-to-date actions used (the live `tenant_usage` total).
    pub used_actions: u64,
    /// Actions bundled into the flat tier price (`included_actions`).
    pub included_actions: u64,
}

/// A planned per-tenant usage record: how many overage units to `set` for
/// this tenant in the current period. Only tenants over their allowance
/// appear (units `> 0`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedOverageReport {
    /// Tenant the record is for.
    pub tenant_id: String,
    /// Cumulative period-to-date overage units (`used − included`).
    pub units: u64,
}

/// Plan the per-tenant usage records from a meter snapshot. Pure: tenants
/// at or under their allowance are dropped (nothing to bill), the rest
/// report exactly `used − included` units. Deterministic, so re-planning
/// the same snapshot yields the identical set (idempotent by
/// construction).
#[must_use]
pub fn plan_overage_reports(meters: &[TenantOverageMeter]) -> Vec<PlannedOverageReport> {
    meters
        .iter()
        .filter_map(|m| {
            let units = overage_units(m.used_actions, m.included_actions);
            (units > 0).then(|| PlannedOverageReport {
                tenant_id: m.tenant_id.clone(),
                units,
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
            report.units,
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

    fn meter(tenant: &str, used: u64, included: u64) -> TenantOverageMeter {
        TenantOverageMeter {
            tenant_id: tenant.into(),
            used_actions: used,
            included_actions: included,
        }
    }

    #[test]
    fn plans_exact_used_minus_allowance_for_over_tenants() {
        // Acceptance 1: a Pro tenant over allowance reports (used −
        // allowance) units, exactly.
        let plan = plan_overage_reports(&[meter("t-over", 2_000, 1_500)]);
        assert_eq!(
            plan,
            vec![PlannedOverageReport {
                tenant_id: "t-over".into(),
                units: 500,
            }]
        );
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
        // Acceptance 1 (idempotency): the same snapshot plans the
        // identical set every time — a re-run pushes the same cumulative
        // totals, so Stripe's `action=set` collapses it to one record.
        let meters = [
            meter("t-a", 2_000, 1_500),
            meter("t-b", 1_400, 1_500),
            meter("t-c", 3_100, 1_500),
        ];
        let first = plan_overage_reports(&meters);
        let again = plan_overage_reports(&meters);
        assert_eq!(first, again);
        assert_eq!(
            first,
            vec![
                PlannedOverageReport {
                    tenant_id: "t-a".into(),
                    units: 500
                },
                PlannedOverageReport {
                    tenant_id: "t-c".into(),
                    units: 1_600
                },
            ]
        );
    }
}
