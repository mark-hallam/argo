//! `argo skill self-improve` — one on-demand self-improvement tick.
//!
//! Thin wrapper over [`argo_skill::self_improve_tick_with`]: builds the
//! tick config from CLI flags, runs ONE tick, and prints the report. No
//! background loop, no auto-graduation — a single tick callable on
//! demand (the brief's drivability contract).

use crate::handlers::provider_distillation_caller::{
    ProviderDistillationCaller, ENV_DISTILL_LIVE,
};
use argo_skill::{self_improve_tick_with, DeterministicDistillationCaller, SelfImproveConfig,
    SkillStore};
use argo_types::{ArgoError, ArgoResult, TenantId};
use uuid::Uuid;

/// True when the live provider-backed distillation caller is enabled via
/// [`ENV_DISTILL_LIVE`]. Default OFF → the deterministic caller runs, so
/// CI and the demo stay reproducible (no model, no network). The eval
/// gate (`run_ab_eval` → `decide_promotion`) is authoritative either way;
/// the live caller only changes how the draft body is synthesised.
fn live_distillation_enabled() -> bool {
    std::env::var(ENV_DISTILL_LIVE)
        .map(|v| matches!(v.trim(), "1" | "true" | "TRUE" | "yes" | "on"))
        .unwrap_or(false)
}

/// Run one tick. `tenant` scopes clustering to a single tenant when set;
/// `since_micros` is the last-tick cursor (defaults to the start of the
/// log).
pub async fn run(
    skills: &SkillStore,
    tenant: Option<&str>,
    since_micros: Option<i64>,
    json: bool,
) -> ArgoResult<()> {
    let tenant_id = match tenant {
        Some(raw) => Some(TenantId(Uuid::parse_str(raw).map_err(|e| {
            ArgoError::Provider(format!("self-improve: --tenant '{raw}' is not a uuid: {e}"))
        })?)),
        None => None,
    };
    let config = SelfImproveConfig {
        since_micros: since_micros.unwrap_or(i64::MIN),
        tenant_id,
        ..SelfImproveConfig::default()
    };
    // Default OFF → deterministic, reproducible tick. Opt in to the real
    // provider-backed caller with ARGO_SKILL_DISTILL_LIVE=1. A distilled
    // skill is still draft-only; promotion is decided ONLY by the measured
    // A/B eval gate inside the tick (DEC-042 / NO-THEATER).
    let report = if live_distillation_enabled() {
        let caller = ProviderDistillationCaller::from_env();
        self_improve_tick_with(skills, config, None, &caller).await?
    } else {
        self_improve_tick_with(skills, config, None, &DeterministicDistillationCaller).await?
    };
    if json {
        println!(
            "{}",
            serde_json::to_string(&report)
                .map_err(|e| ArgoError::Schema(format!("self-improve report json: {e}")))?
        );
    } else {
        println!(
            "traces_read={} candidate_clusters={} promoted={} decisions={}",
            report.traces_read,
            report.candidate_clusters,
            report.promoted,
            report.outcomes.len(),
        );
        for outcome in &report.outcomes {
            println!(
                "  {} [{}] -> {:?}",
                outcome.skill_name, outcome.tenant_id, outcome.decision
            );
        }
    }
    Ok(())
}
