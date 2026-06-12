//! `argo billing report-overage` — the production caller for DEC-053
//! metered overage (`pricing-wire-overage-report-and-cap-enforcement`,
//! part a).
//!
//! Reads every Pro tenant's live `tenant_usage` action meter, computes
//! period-to-date overage units against the Pro tier's `included_actions`
//! allowance, and pushes each over-allowance tenant's cumulative total to
//! the Stripe metered price named by `STRIPE_PRICE_PRO_OVERAGE`. The push
//! is idempotent (`action=set` + a stable `(tenant_id, period_start)`
//! key), so this command is safe to run on a schedule (cron / launchd) —
//! a re-run re-asserts the same totals rather than double-billing.
//!
//! `--dry-run` (and the env-absent path) plans the reports without any
//! Stripe call, so an operator can preview exactly what would be billed.

use crate::cli::BillingReportOverageArgs;
use argo_billing::{
    plan_overage_reports, run_overage_reports, OverageRunSummary, PlannedOverageReport,
    StripeClient, TenantOverageMeter, STRIPE_PRICE_PRO_OVERAGE_ENV,
};
use argo_identity::tier::{Tier, TierConfig};
use argo_store::tenant_quota::TenantQuotaStore;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use chrono::Utc;
use serde_json::json;

/// Dispatch `argo billing report-overage`. Admin-only by convention; the
/// CLI runs locally as the workspace operator.
pub async fn run(args: BillingReportOverageArgs) -> ArgoResult<()> {
    let store = open_store().await?;
    let now = Utc::now();
    let included = pro_included_actions();

    let (meters, period_start) = collect_pro_meters(&store, included, now).await?;
    let reports = plan_overage_reports(&meters);

    let Some(subscription_item) = std::env::var(STRIPE_PRICE_PRO_OVERAGE_ENV).ok() else {
        // No metered price configured → plan-only (gated-cohort flat-fee mode).
        return print_plan(&reports, true, args.dry_run, args.json);
    };
    if args.dry_run {
        return print_plan(&reports, false, args.dry_run, args.json);
    }

    let client = StripeClient::from_env()
        .map_err(|e| ArgoError::Provider(format!("stripe client: {e}")))?;
    let summary = run_overage_reports(
        &client,
        &subscription_item,
        &reports,
        period_start,
        now.timestamp(),
    )
    .await
    .map_err(|e| ArgoError::Provider(format!("overage report run: {e}")))?;
    print_summary(&reports, &summary, args.json);
    Ok(())
}

/// Pro tier `included_actions` allowance from the bundled tier config.
fn pro_included_actions() -> u64 {
    TierConfig::default_config()
        .limits(Tier::Pro)
        .map_or(0, |limits| limits.included_actions)
}

/// Read every Pro tenant's period-to-date action meter. Returns the
/// per-tenant meters plus the (shared, monthly) period_start used as the
/// idempotency anchor. `period_start` defaults to `0` when there are no
/// Pro tenants (no report is pushed in that case anyway).
async fn collect_pro_meters(
    store: &SessionStore,
    included: u64,
    now: chrono::DateTime<Utc>,
) -> ArgoResult<(Vec<TenantOverageMeter>, i64)> {
    let quotas = TenantQuotaStore::new(store.pool());
    let tenants = argo_store::tenant_query::list_tenants_on_tier(store, "pro").await?;
    let mut meters = Vec::with_capacity(tenants.len());
    let mut period_start = 0_i64;
    for tenant in tenants {
        let tenant_id = tenant.to_string();
        let summary = quotas.summary(&tenant_id, now).await?;
        period_start = summary.usage.period_start;
        meters.push(TenantOverageMeter {
            tenant_id,
            used_actions: summary.usage.used_actions,
            included_actions: included,
        });
    }
    Ok((meters, period_start))
}

fn print_plan(
    reports: &[PlannedOverageReport],
    env_absent: bool,
    dry_run: bool,
    json_out: bool,
) -> ArgoResult<()> {
    if json_out {
        println!(
            "{}",
            json!({
                "mode": "plan",
                "stripe_env_set": !env_absent,
                "dry_run": dry_run,
                "reports": reports
                    .iter()
                    .map(|r| json!({ "tenant_id": r.tenant_id, "units": r.units }))
                    .collect::<Vec<_>>(),
            })
        );
        return Ok(());
    }
    if env_absent && !dry_run {
        println!("(STRIPE_PRICE_PRO_OVERAGE unset — planning only, no Stripe push)");
    }
    if reports.is_empty() {
        println!("no Pro tenant is over its included-actions allowance");
    } else {
        for r in reports {
            println!("{}: {} overage units (planned)", r.tenant_id, r.units);
        }
        println!("{} tenant(s) would be billed", reports.len());
    }
    Ok(())
}

fn print_summary(reports: &[PlannedOverageReport], summary: &OverageRunSummary, json_out: bool) {
    if json_out {
        println!(
            "{}",
            json!({
                "mode": "report",
                "planned": reports.len(),
                "reported": summary.reported,
                "failed": summary.failed,
            })
        );
        return;
    }
    println!(
        "reported {} / {} Pro tenant(s) to Stripe ({} failed)",
        summary.reported,
        reports.len(),
        summary.failed,
    );
}

async fn open_store() -> ArgoResult<SessionStore> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let db = cwd.join(".skycot").join("sessions.db");
    let store = SessionStore::open(db.to_string_lossy().as_ref()).await?;
    store.migrate().await?;
    Ok(store)
}
