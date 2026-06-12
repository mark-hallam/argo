//! Audit-chain helpers for the self-improvement cron tick. Extracted from
//! `self_improve_cron.rs` (mechanical, no behaviour change) to keep that
//! file under the per-file LOC budget. Owns the ONE hash-chained,
//! tenant-stamped decision audit row each cluster outcome emits.

use super::{SelfImproveTrace, DECISION_ACTION, DECISION_AUDIT_KIND};
use crate::ab_eval::{run_ab_eval, SkillAbEvalReport};
use crate::self_improve_loop::{decide_promotion, PromotionDecision, PromotionPolicy};
use crate::store::SkillStore;
use argo_types::{ArgoError, ArgoResult};
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
    let canonical = serde_json::to_string(&payload)
        .map_err(|e| ArgoError::Schema(format!("self_improve audit json: {e}")))?;
    // `BEGIN IMMEDIATE` serializes the read-then-insert so concurrent
    // appends can't fork the chain (fix-audit-chain-append-serialization).
    let mut tx = skills
        .pool()
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(|e| ArgoError::Provider(format!("self_improve audit tx begin: {e}")))?;
    let parent = last_audit_hash(&mut tx).await?;
    let self_hash = chain_hash(&parent, DECISION_AUDIT_KIND, &canonical);
    let audit_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO audit_record (id, session_id, kind, payload_json, \
         parent_hash, self_hash, created_at, principal_id, tenant_id, \
         worker_id, channel, action_type, outcome_status, \
         approver_id, cost_band, data_classification) \
         VALUES (?, NULL, ?, ?, ?, ?, ?, NULL, ?, NULL, NULL, ?, ?, NULL, NULL, NULL)",
    )
    .bind(audit_id.as_bytes().as_slice())
    .bind(DECISION_AUDIT_KIND)
    .bind(&canonical)
    .bind(parent.as_slice())
    .bind(self_hash.as_slice())
    .bind(Utc::now().timestamp_micros())
    .bind(&rep.tenant_id)
    .bind(DECISION_ACTION)
    .bind(status)
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
