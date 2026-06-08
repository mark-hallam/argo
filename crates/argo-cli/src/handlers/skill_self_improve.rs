//! `argo skill self-improve` — one on-demand self-improvement tick.
//!
//! Thin wrapper over [`argo_skill::self_improve_tick_with`]: builds the
//! tick config from CLI flags, runs ONE tick, and prints the report. No
//! background loop, no auto-graduation — a single tick callable on
//! demand (the brief's drivability contract).

use argo_skill::{self_improve_tick_with, DeterministicDistillationCaller, SelfImproveConfig,
    SkillStore};
use argo_types::{ArgoError, ArgoResult, TenantId};
use uuid::Uuid;

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
    let report =
        self_improve_tick_with(skills, config, None, &DeterministicDistillationCaller).await?;
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
