//! Self-improvement cron driver — wires the closed learning library
//! into a single, on-demand tenant-scoped tick.
//!
//! The stages beneath this module were each real but disconnected; this
//! driver is the missing orchestration layer that runs them end to end
//! over live session audit rows:
//!
//! ```text
//! audit rows ─since cursor, tenant-scoped─▶ TraceFingerprint
//!   ─cluster_traces─▶ TraceCluster ─distillation_candidates─▶ candidate
//!   ─cluster_to_cohort─▶ Cohort
//!   ─distill_and_propose_judged (deterministic caller)─▶ proposed skill
//!   ─run_ab_eval─▶ SkillAbEvalReport ─decide_promotion─▶ PromotionDecision
//!   ─[on Promote]─▶ promote_with_regression_gate
//!   ─▶ ONE hash-chained per-skill decision audit row (tenant-stamped)
//! ```
//!
//! Mirrors [`crate::per_worker_learning_cron`]'s tick + hash-chained
//! audit + `Option<TenantId>` filter shape — it does NOT fork a parallel
//! cron type, and it calls the EXISTING stage fns rather than
//! re-implementing any of them.
//!
//! The distillation step runs through a **deterministic**
//! [`DeterministicDistillationCaller`] — no live model, no network — so
//! the whole tick is reproducible in CI and the demo, mirroring how
//! [`crate::ab_eval::BodyContainsExecutor`] keeps the A/B lane
//! deterministic by default.
//!
//! ## Drivability + autonomy
//! This is a **single tick callable on demand** (the
//! `argo skill self-improve` CLI subcommand is its consumer). It does
//! NOT spawn a background loop and does NOT auto-graduate anything — the
//! autofire / graduation flags stay OFF. A promotion only happens when a
//! measured A/B-eval clears [`crate::self_improve_loop::PromotionPolicy`].
//!
//! ## Sensible deviation — the trace ↔ A/B family link
//! A [`crate::dreams_distillation::distill_and_propose_judged`] call
//! proposes a NEW standalone skill (its own generated name), while
//! [`crate::ab_eval::run_ab_eval`] compares two versions of ONE existing
//! skill family. The library pieces compose that way by construction, so
//! each trace row names the family + the two versions to A/B
//! (`skill_name` / `baseline_version` / `candidate_version`); the
//! distillation proposal is the cohort-mining side effect, and the
//! promotion stage measures + gates that named family. One decision
//! audit row is emitted per cluster outcome regardless.

use crate::ab_eval::run_ab_eval;
use crate::ab_test::{create_run, ensure_ab_schema, promote_with_regression_gate};
use crate::dreams_distillation::{distill_and_propose_judged, Cohort, DistillationResult,
    SkillDistillationCaller};
use crate::self_improve_loop::{
    cluster_to_cohort, decide_promotion, PromotionDecision, PromotionPolicy, TraceMember,
};
use crate::store::SkillStore;
use crate::trace_cluster::{
    cluster_traces, distillation_candidates, TraceFingerprint, DEFAULT_MIN_CLUSTER_SIZE,
    DEFAULT_MIN_MEAN_SUCCESS, DEFAULT_MIN_SIMILARITY,
};
use argo_judge::Judge;
use argo_types::{ArgoError, ArgoResult, SessionId, TenantId};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

#[path = "self_improve_cron_helpers.rs"]
mod helpers;

/// Audit `kind` written for each self-improvement promotion decision.
pub const DECISION_AUDIT_KIND: &str = "skill.self_improve.decision";
/// Audit `action_type` column value for a decision row.
const DECISION_ACTION: &str = "SELF_IMPROVE_DECISION";
/// Audit `kind` the tick reads session traces from. Producers stamp the
/// trace payload (see [`SelfImproveTrace`]) onto a row of this kind.
pub const TRACE_AUDIT_KIND: &str = "skill.self_improve.trace";

/// Tunables for one self-improvement tick.
#[derive(Debug, Clone)]
pub struct SelfImproveConfig {
    /// Earliest `audit_record.created_at` (micros) to consider — the
    /// last-tick cursor. `i64::MIN` reads from the beginning.
    pub since_micros: i64,
    /// Optional tenant filter. `Some(t)` clusters ONLY tenant `t`'s
    /// traces (tenant isolation); `None` reads every tenant.
    pub tenant_id: Option<TenantId>,
    /// Cosine-similarity floor for two traces to cluster.
    pub min_similarity: f64,
    /// Minimum members before a cluster is a distillation candidate.
    pub min_cluster_size: usize,
    /// Mean-success floor for a candidate cluster.
    pub min_mean_success: f64,
    /// Promotion-on-performance thresholds.
    pub promotion_policy: PromotionPolicy,
    /// `target_runs` stamped on the A/B run created for a promotion.
    pub target_runs: i64,
}

impl Default for SelfImproveConfig {
    fn default() -> Self {
        Self {
            since_micros: i64::MIN,
            tenant_id: None,
            min_similarity: DEFAULT_MIN_SIMILARITY,
            min_cluster_size: DEFAULT_MIN_CLUSTER_SIZE,
            min_mean_success: DEFAULT_MIN_MEAN_SUCCESS,
            promotion_policy: PromotionPolicy::default(),
            target_runs: 100,
        }
    }
}

/// One production trace, parsed from a [`TRACE_AUDIT_KIND`] audit row.
///
/// Carries everything the chain needs: the clustering signal
/// (`action_sequence` + `success_metric`), the cohort rebuild context
/// (`vertical_id` / `action_type` / `final_text`), AND the A/B family
/// link (`skill_name` / `baseline_version` / `candidate_version`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelfImproveTrace {
    /// Stable trace id (the audit row id).
    pub id: String,
    /// Owning tenant (the audit row's `tenant_id` column).
    pub tenant_id: String,
    /// Vertical the trace ran under.
    pub vertical_id: String,
    /// Action kind the trace performed.
    pub action_type: String,
    /// Ordered action/step names (clustering signal + cohort body).
    pub trajectory: Vec<String>,
    /// Final captured text (URL / confirmation / …), if any.
    pub final_text: Option<String>,
    /// Success metric of the trace, 0.0–1.0.
    pub success_metric: f64,
    /// Skill family this trace's pattern improves.
    pub skill_name: String,
    /// Baseline (before) version to A/B.
    pub baseline_version: String,
    /// Candidate (after) version to A/B.
    pub candidate_version: String,
}

impl SelfImproveTrace {
    fn fingerprint(&self) -> TraceFingerprint {
        TraceFingerprint::from_parts(
            self.id.clone(),
            self.trajectory.clone(),
            self.success_metric,
        )
    }

    fn member(&self) -> TraceMember {
        TraceMember {
            id: self.id.clone(),
            vertical_id: self.vertical_id.clone(),
            action_type: self.action_type.clone(),
            trajectory: self.trajectory.clone(),
            final_text: self.final_text.clone(),
            success_metric: self.success_metric,
        }
    }
}

/// One cluster's outcome inside a tick report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelfImproveOutcome {
    /// Skill family the decision applies to.
    pub skill_name: String,
    /// Tenant the decision was scoped + audited under.
    pub tenant_id: String,
    /// The pure promotion decision over the measured A/B-eval.
    pub decision: PromotionDecision,
    /// Skill id of the cohort-distilled proposal (review-queue side
    /// effect), if the distillation succeeded.
    pub proposed_skill_id: Option<String>,
}

/// Summary returned by one self-improvement tick.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SelfImproveReport {
    /// Traces read after the tenant/cursor filter.
    pub traces_read: usize,
    /// Clusters that cleared the distillation-candidate filter.
    pub candidate_clusters: usize,
    /// Per-cluster decision outcomes (one audit row each).
    pub outcomes: Vec<SelfImproveOutcome>,
    /// Count of `Promote` decisions applied via the regression gate.
    pub promoted: usize,
}

/// Deterministic [`SkillDistillationCaller`] — builds a cohort body
/// mechanically from the cohort's anchor trajectory. No model, no
/// network, so every tick is reproducible.
#[derive(Debug, Default, Clone, Copy)]
pub struct DeterministicDistillationCaller;

#[async_trait]
impl SkillDistillationCaller for DeterministicDistillationCaller {
    async fn distill(&self, cohort: &Cohort) -> ArgoResult<DistillationResult> {
        let steps = cohort
            .members
            .first()
            .map(|m| {
                m.trajectory
                    .iter()
                    .map(|s| format!("1. {s}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_default();
        let body = format!(
            "# {action}\n\nDistilled from {n} successful `{action}` runs.\n\n## Steps\n{steps}\n",
            action = cohort.action_type,
            n = cohort.members.len(),
        );
        Ok(DistillationResult {
            body,
            model: "deterministic".to_string(),
            cost_micros: 0,
        })
    }
}

/// Run one self-improvement tick with environment/default tunables.
///
/// # Errors
/// Propagates audit-read / store errors. Per-cluster stage failures
/// (cohort too small, missing A/B versions) are logged and skipped, not
/// fatal — a single bad cluster never aborts the tick.
pub async fn self_improve_tick(skills: &SkillStore) -> ArgoResult<SelfImproveReport> {
    self_improve_tick_with(
        skills,
        SelfImproveConfig::default(),
        None,
        &DeterministicDistillationCaller,
    )
    .await
}

/// Run one self-improvement tick with explicit tunables, judge, and
/// distillation caller.
///
/// # Errors
/// Same as [`self_improve_tick`].
pub async fn self_improve_tick_with(
    skills: &SkillStore,
    config: SelfImproveConfig,
    judge: Option<&dyn Judge>,
    caller: &dyn SkillDistillationCaller,
) -> ArgoResult<SelfImproveReport> {
    let traces = read_self_improve_traces(skills, config.since_micros, config.tenant_id.as_ref())
        .await?;
    let fingerprints: Vec<TraceFingerprint> = traces.iter().map(SelfImproveTrace::fingerprint).collect();
    let clusters = cluster_traces(&fingerprints, config.min_similarity);
    let candidates =
        distillation_candidates(&clusters, config.min_cluster_size, config.min_mean_success);

    let mut report = SelfImproveReport {
        traces_read: traces.len(),
        candidate_clusters: candidates.len(),
        ..SelfImproveReport::default()
    };
    for cluster in &candidates {
        let Some(rep) = traces.iter().find(|t| t.id == cluster.representative_id) else {
            continue;
        };
        let members: Vec<TraceMember> = cluster
            .member_ids
            .iter()
            .filter_map(|id| traces.iter().find(|t| &t.id == id))
            .map(SelfImproveTrace::member)
            .collect();
        let cohort =
            match cluster_to_cohort(cluster, &members, &rep.vertical_id, &rep.action_type) {
                Ok(c) => c,
                Err(err) => {
                    tracing::debug!(target: "argo_skill::self_improve_cron", error = %err, "cluster skipped: cohort bridge");
                    continue;
                }
            };
        let proposed_skill_id =
            match distill_and_propose_judged(&cohort, caller, skills, SessionId::new(), judge).await
            {
                Ok((skill, _)) => Some(skill.id.to_string()),
                Err(err) => {
                    tracing::warn!(target: "argo_skill::self_improve_cron", error = %err, "cluster skipped: distill/propose");
                    continue;
                }
            };
        let ab_report = match run_ab_eval(
            skills,
            &rep.skill_name,
            &rep.baseline_version,
            &rep.candidate_version,
        )
        .await
        {
            Ok(r) => r,
            Err(err) => {
                tracing::warn!(target: "argo_skill::self_improve_cron", skill = %rep.skill_name, error = %err, "cluster skipped: ab-eval");
                continue;
            }
        };
        let decision = decide_promotion(&ab_report, &config.promotion_policy);
        if decision.is_promote() {
            apply_promotion(skills, rep, &config).await?;
            report.promoted += 1;
        }
        helpers::append_decision_audit(skills, rep, &decision, &ab_report).await?;
        report.outcomes.push(SelfImproveOutcome {
            skill_name: rep.skill_name.clone(),
            tenant_id: rep.tenant_id.clone(),
            decision,
            proposed_skill_id,
        });
    }
    Ok(report)
}

/// Apply a promotion through the existing regression gate. Creates the
/// A/B run row (variant_a = baseline, variant_b = candidate) under the
/// trace's tenant, then promotes the candidate as the explicit winner.
async fn apply_promotion(
    skills: &SkillStore,
    rep: &SelfImproveTrace,
    config: &SelfImproveConfig,
) -> ArgoResult<()> {
    let store = skills.store_arc();
    let tenant = TenantId(Uuid::parse_str(&rep.tenant_id).map_err(|e| {
        ArgoError::Provider(format!("self_improve: trace tenant_id '{}' is not a uuid: {e}", rep.tenant_id))
    })?);
    ensure_ab_schema(&store).await?;
    let run_id = create_run(
        &store,
        &tenant,
        &rep.skill_name,
        &rep.baseline_version,
        &rep.candidate_version,
        config.target_runs,
    )
    .await?;
    promote_with_regression_gate(
        &store,
        &tenant,
        &run_id,
        Some(&rep.candidate_version),
        None,
        false,
    )
    .await?;
    Ok(())
}

/// Read [`TRACE_AUDIT_KIND`] audit rows since `since_micros`, optionally
/// tenant-filtered, and parse each payload into a [`SelfImproveTrace`].
///
/// Mirrors [`crate::audit_reader::SessionStoreAuditReader`]'s
/// `created_at >= ?` cursor-scan shape; the `tenant_id = ?` predicate is
/// what enforces tenant isolation in SQL (tenant A's tick never sees
/// tenant B's rows).
async fn read_self_improve_traces(
    skills: &SkillStore,
    since_micros: i64,
    tenant_id: Option<&TenantId>,
) -> ArgoResult<Vec<SelfImproveTrace>> {
    let base = "SELECT id, tenant_id, payload_json FROM audit_record \
                WHERE kind = ? AND created_at >= ?";
    let rows = match tenant_id {
        Some(t) => {
            sqlx::query(&format!("{base} AND tenant_id = ?"))
                .bind(TRACE_AUDIT_KIND)
                .bind(since_micros)
                .bind(t.to_string())
                .fetch_all(skills.pool())
                .await
        }
        None => {
            sqlx::query(base)
                .bind(TRACE_AUDIT_KIND)
                .bind(since_micros)
                .fetch_all(skills.pool())
                .await
        }
    }
    .map_err(|e| ArgoError::Provider(format!("self_improve trace read: {e}")))?;

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let id_bytes: Vec<u8> = row.get("id");
        let row_tenant: Option<String> = row.try_get("tenant_id").ok();
        let payload_json: String = row.get("payload_json");
        let Ok(payload) = serde_json::from_str::<serde_json::Value>(&payload_json) else {
            continue;
        };
        let Some(trace) = parse_trace(&id_bytes, row_tenant, &payload) else {
            continue;
        };
        out.push(trace);
    }
    Ok(out)
}

/// Parse one trace payload. Skips rows missing any required field rather
/// than failing the whole tick on one malformed row.
fn parse_trace(
    id_bytes: &[u8],
    row_tenant: Option<String>,
    payload: &serde_json::Value,
) -> Option<SelfImproveTrace> {
    let str_field = |k: &str| payload.get(k).and_then(|v| v.as_str()).map(str::to_string);
    let trajectory = payload
        .get("trajectory")?
        .as_array()?
        .iter()
        .filter_map(|v| v.as_str().map(str::to_string))
        .collect::<Vec<_>>();
    Some(SelfImproveTrace {
        id: hex(id_bytes),
        tenant_id: row_tenant
            .or_else(|| str_field("tenant_id"))
            .unwrap_or_default(),
        vertical_id: str_field("vertical_id")?,
        action_type: str_field("action_type")?,
        trajectory,
        final_text: str_field("final_text"),
        success_metric: payload.get("success_metric").and_then(|v| v.as_f64())?,
        skill_name: str_field("skill_name")?,
        baseline_version: str_field("baseline_version")?,
        candidate_version: str_field("candidate_version")?,
    })
}

fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

#[cfg(test)]
#[path = "self_improve_cron_tests.rs"]
mod tests;
