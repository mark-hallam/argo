//! Audit-chain helpers for the self-improvement cron tick. Extracted from
//! `self_improve_cron.rs` (mechanical, no behaviour change) to keep that
//! file under the per-file LOC budget. Owns the ONE hash-chained,
//! tenant-stamped decision audit row each cluster outcome emits.

use super::{SelfImproveTrace, DECISION_ACTION, DECISION_AUDIT_KIND};
use crate::ab_eval::{run_ab_eval, SkillAbEvalReport};
use crate::dreams_distillation::{
    distill_and_propose_judged_for_tenant_gated, Cohort, DistillationResult, SkillDistillationCaller,
};
use crate::self_improve_loop::{decide_promotion, PromotionDecision, PromotionPolicy};
use crate::skill::Skill;
use crate::store::SkillStore;
use argo_audit::AuditKind;
use argo_judge::Judge;
use argo_types::{ArgoError, ArgoResult, SessionId};
use chrono::Utc;
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Append ONE hash-chained, tenant-stamped decision audit row. Mirrors
/// [`crate::per_worker_learning_cron`]'s `append_learning_audit` chain
/// shape (parent = last `session_id IS NULL` row).
pub(super) async fn append_decision_audit(
    skills: &SkillStore,
    rep: &SelfImproveTrace,
    decision: &PromotionDecision,
    ab_report: &SkillAbEvalReport,
) -> ArgoResult<()> {
    let (status, reason) = match decision {
        PromotionDecision::Promote { reason } => ("promote", reason),
        PromotionDecision::Hold { reason } => ("hold", reason),
        PromotionDecision::Reject { reason } => ("reject", reason),
    };
    let skill_id = skills
        .versions_for_name(&rep.skill_name)
        .await
        .ok()
        .and_then(|vs| {
            vs.into_iter()
                .find(|s| s.frontmatter.version == rep.candidate_version)
                .map(|s| s.id.to_string())
        });
    let payload = serde_json::json!({
        "action_type": DECISION_ACTION,
        "outcome_status": status,
        "tenant_id": rep.tenant_id,
        "skill_name": rep.skill_name,
        "skill_id": skill_id,
        "baseline_version": rep.baseline_version,
        "candidate_version": rep.candidate_version,
        "pass_rate_delta": ab_report.pass_rate_delta,
        "baseline_pass_rate": ab_report.baseline.pass_rate,
        "candidate_pass_rate": ab_report.candidate.pass_rate,
        "reason": reason,
    });
    append_chained_audit(
        skills,
        DECISION_AUDIT_KIND,
        Some(DECISION_ACTION),
        Some(status),
        &rep.tenant_id,
        &payload,
    )
    .await
}

/// Append ONE hash-chained, tenant-stamped audit row onto the
/// self-improvement chain. The single extension point for the loop's
/// `session_id IS NULL` chain — [`append_decision_audit`] and the
/// `skill.distilled` / `skill.proposed_for_review` emitters
/// ([`super::emit_distilled_audit`], [`super::emit_proposed_for_review_audit`])
/// ALL route through here, so the chain verifier (`argo audit verify`)
/// walks ONE chain (DEC-042 p1c — never a forked chaining impl).
///
/// `kind` is both the wire `kind` column AND the chain-hash domain
/// separator, so two distinct kinds can never collide on the same parent.
/// `action_type` / `outcome_status` are optional column stamps (the
/// decision row stamps both; the distill/proposal rows leave them NULL and
/// carry their detail in `payload`).
pub(super) async fn append_chained_audit(
    skills: &SkillStore,
    kind: &str,
    action_type: Option<&str>,
    outcome_status: Option<&str>,
    tenant_id: &str,
    payload: &serde_json::Value,
) -> ArgoResult<()> {
    let canonical = serde_json::to_string(payload)
        .map_err(|e| ArgoError::Schema(format!("self_improve audit json: {e}")))?;
    // `BEGIN IMMEDIATE` serializes the read-then-insert so concurrent
    // appends can't fork the chain (fix-audit-chain-append-serialization).
    let mut tx = skills
        .pool()
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(|e| ArgoError::Provider(format!("self_improve audit tx begin: {e}")))?;
    let parent = last_audit_hash(&mut tx).await?;
    let self_hash = chain_hash(&parent, kind, &canonical);
    let audit_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO audit_record (id, session_id, kind, payload_json, \
         parent_hash, self_hash, created_at, principal_id, tenant_id, \
         worker_id, channel, action_type, outcome_status, \
         approver_id, cost_band, data_classification) \
         VALUES (?, NULL, ?, ?, ?, ?, ?, NULL, ?, NULL, NULL, ?, ?, NULL, NULL, NULL)",
    )
    .bind(audit_id.as_bytes().as_slice())
    .bind(kind)
    .bind(&canonical)
    .bind(parent.as_slice())
    .bind(self_hash.as_slice())
    .bind(Utc::now().timestamp_micros())
    .bind(tenant_id)
    .bind(action_type)
    .bind(outcome_status)
    .execute(&mut *tx)
    .await
    .map_err(|e| ArgoError::Provider(format!("self_improve audit insert: {e}")))?;
    tx.commit()
        .await
        .map_err(|e| ArgoError::Provider(format!("self_improve audit tx commit: {e}")))?;
    Ok(())
}

async fn last_audit_hash(tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>) -> ArgoResult<[u8; 32]> {
    let row: Option<Vec<u8>> = sqlx::query_scalar(
        "SELECT self_hash FROM audit_record WHERE session_id IS NULL ORDER BY seq DESC LIMIT 1",
    )
    .fetch_optional(&mut **tx)
    .await
    .map_err(|e| ArgoError::Provider(format!("self_improve audit hash: {e}")))?;
    let mut out = [0u8; 32];
    if let Some(bytes) = row {
        if bytes.len() == 32 {
            out.copy_from_slice(&bytes);
        }
    }
    Ok(out)
}

fn chain_hash(parent: &[u8; 32], kind: &str, payload: &str) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(parent);
    h.update(kind.as_bytes());
    h.update(b"\0");
    h.update(payload.as_bytes());
    let out = h.finalize();
    let mut buf = [0u8; 32];
    buf.copy_from_slice(&out);
    buf
}

/// Emit the two UPSTREAM self-improve audit rows for one distilled
/// candidate (DEC-042 p1c). `skill.distilled` always fires (the provider
/// spend happened, even for a gated-Reject candidate); `skill.proposed_for_review`
/// fires only when the candidate actually entered the review queue
/// (`entered_review_queue` — false on a gated Reject, which goes straight to
/// `Rejected`). Both route through [`append_chained_audit`] so the chain
/// reads distilled → proposed → decision through ONE chain.
pub(super) async fn emit_upstream_audits(
    skills: &SkillStore,
    tenant_id: &str,
    cohort: &Cohort,
    skill: &Skill,
    result: &DistillationResult,
    entered_review_queue: bool,
    learned_in: &str,
) -> ArgoResult<()> {
    emit_distilled_audit(
        skills,
        tenant_id,
        &cohort.vertical_id,
        &cohort.action_type,
        cohort.members.len(),
        &result.model,
        result.cost_micros,
        &skill.frontmatter.name,
        &skill.frontmatter.version,
    )
    .await?;
    if entered_review_queue {
        emit_proposed_for_review_audit(
            skills,
            tenant_id,
            &skill.frontmatter.name,
            &skill.frontmatter.version,
            &cohort.vertical_id,
            "distilled",
            learned_in,
        )
        .await?;
    }
    Ok(())
}

/// Distill the cohort (A/B-gated) and emit the distilled (+ proposed-for-review
/// unless gate-Rejected) audit rows BEFORE the decision row, so the chain reads
/// distilled → proposed → decision. Extracted from `self_improve_tick_with`
/// (mechanical, no behaviour change) to hold that fn under the 80-LOC ceiling.
/// Returns the proposed skill id, or `None` to SKIP the cluster — a
/// distill/propose failure is non-fatal to the tick (logged + skipped, matching
/// the prior inline behavior).
///
/// # Errors
/// Propagates the upstream-audit insert/commit errors.
#[allow(clippy::too_many_arguments)]
pub(super) async fn distill_and_audit(
    skills: &SkillStore,
    cohort: &Cohort,
    caller: &dyn SkillDistillationCaller,
    learned_in: SessionId,
    judge: Option<&dyn Judge>,
    tenant_id: &str,
    ab_reject_reason: Option<&str>,
) -> ArgoResult<Option<String>> {
    let (proposed_skill_id, distilled) = match distill_and_propose_judged_for_tenant_gated(
        cohort,
        caller,
        skills,
        learned_in,
        judge,
        tenant_id,
        ab_reject_reason,
    )
    .await
    {
        Ok((skill, result)) => (Some(skill.id.to_string()), Some((skill, result))),
        Err(err) => {
            tracing::warn!(target: "argo_skill::self_improve_cron", error = %err, "cluster skipped: distill/propose");
            return Ok(None);
        }
    };
    if let Some((skill, result)) = &distilled {
        emit_upstream_audits(
            skills,
            tenant_id,
            cohort,
            skill,
            result,
            ab_reject_reason.is_none(),
            &learned_in.to_string(),
        )
        .await?;
    }
    Ok(proposed_skill_id)
}

/// Emit a `skill.distilled` audit row onto the self-improve hash-chain
/// ([`AuditKind::SkillDistilled`]). Called from the cron tick AND the CLI
/// real-provider distill path (`argo skill distill-now`), so a distillation
/// call's provider spend + model + outcome is tamper-evident, not just a
/// `tracing` line (DEC-042 p1c). Routes through the shared chain helper —
/// no forked chaining impl.
///
/// # Errors
/// Propagates the audit-row insert / commit errors.
#[allow(clippy::too_many_arguments)]
pub async fn emit_distilled_audit(
    skills: &SkillStore,
    tenant_id: &str,
    vertical: &str,
    action: &str,
    size: usize,
    model: &str,
    cost_usd_micros: i64,
    skill_name: &str,
    skill_version: &str,
) -> ArgoResult<()> {
    let payload = serde_json::json!({
        "tenant_id": tenant_id,
        "vertical": vertical,
        "action": action,
        "size": size,
        "model": model,
        "cost_usd_micros": cost_usd_micros,
        "skill_name": skill_name,
        "skill_version": skill_version,
    });
    append_chained_audit(
        skills,
        AuditKind::SkillDistilled.label(),
        Some("SELF_IMPROVE_DISTILL"),
        None,
        tenant_id,
        &payload,
    )
    .await
}

/// Emit a `skill.proposed_for_review` audit row onto the self-improve
/// hash-chain ([`AuditKind::SkillProposedForReview`]) when a candidate
/// enters the `pending_review` queue, recording HOW it arrived (`source`)
/// so an operator can trace a queued skill's provenance (DEC-042 p1c).
/// Routes through the same chain helper as the distilled + decision rows.
///
/// # Errors
/// Propagates the audit-row insert / commit errors.
pub async fn emit_proposed_for_review_audit(
    skills: &SkillStore,
    tenant_id: &str,
    skill_name: &str,
    skill_version: &str,
    vertical: &str,
    source: &str,
    learned_in: &str,
) -> ArgoResult<()> {
    let payload = serde_json::json!({
        "tenant_id": tenant_id,
        "skill_name": skill_name,
        "skill_version": skill_version,
        "vertical": vertical,
        "source": source,
        "learned_in": learned_in,
    });
    append_chained_audit(
        skills,
        AuditKind::SkillProposedForReview.label(),
        Some("SELF_IMPROVE_PROPOSE"),
        None,
        tenant_id,
        &payload,
    )
    .await
}

/// A/B-eval the cluster representative and decide promotion. `None` skips
/// the cluster (the eval errored — non-fatal to the tick). Lives here so
/// `self_improve_tick_with` stays under the 80-LOC fn budget and
/// `self_improve_cron.rs` stays under the 500-LOC file ceiling.
pub(super) async fn eval_and_decide(
    skills: &SkillStore,
    rep: &SelfImproveTrace,
    policy: &PromotionPolicy,
) -> Option<(SkillAbEvalReport, PromotionDecision)> {
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
            return None;
        }
    };
    let decision = decide_promotion(&ab_report, policy);
    Some((ab_report, decision))
}
