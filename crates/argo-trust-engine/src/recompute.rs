//! Per-worker recompute + tick-all-workers driver. The one place
//! the engine WRITES anything.
//!
//! Writes flow through the existing
//! [`argo_identity::IdentityStore::set_worker_trust_grade`] surface
//! so the read endpoint (`GET /api/v1/workers/<id>/trust-grade`)
//! sees the new value on the next poll. The audit emit uses
//! [`argo_audit::AuditSink::write_with_tenant`] so the row carries
//! the correct tenant scoping for the
//! `mission-enterprise-cross-tenant-audit` chain.

use crate::ratchet::{self, grade_to_u8, u8_to_grade, RatchetDecision};
use crate::scoring;
use argo_audit::{AuditKind, AuditSink};
use argo_identity::{IdentityStore, WorkerId};
use argo_skill::TrustGrade;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use serde_json::json;
use std::str::FromStr;
use std::sync::Arc;

/// Recompute one worker's grade. Returns the ratchet decision that
/// was applied. The caller owns the [`AuditSink`] — typically the
/// gateway-bound sink for synchronous recomputes, a fresh sink for
/// the cron-loop tick.
///
/// # Errors
/// Returns [`ArgoError::Provider`] when scoring queries fail, when
/// the identity write fails, or when the audit emit fails.
pub async fn recompute_worker(
    store: &SessionStore,
    sink: &AuditSink,
    tenant: &TenantId,
    worker_id: &str,
    current_grade: TrustGrade,
) -> ArgoResult<RatchetDecision> {
    let breakdown = scoring::compute_score(store, tenant, worker_id).await?;
    let decision = ratchet::ratchet_grade(current_grade, breakdown.total);
    match &decision {
        RatchetDecision::Hold => Ok(decision),
        RatchetDecision::Promote { from, to } | RatchetDecision::Demote { from, to } => {
            let new_u8 = grade_to_u8(*to);
            let parsed_worker = WorkerId::from_str(worker_id)
                .map_err(|e| ArgoError::Provider(format!("trust_engine worker id parse: {e}")))?;
            let identities = IdentityStore::new(store);
            identities
                .set_worker_trust_grade(*tenant, parsed_worker, new_u8)
                .await?;

            let decision_label = match &decision {
                RatchetDecision::Promote { .. } => "promote",
                RatchetDecision::Demote { .. } => "demote",
                RatchetDecision::Hold => "hold",
            };
            let payload = json!({
                "worker_id": worker_id,
                "tenant_id": tenant.to_string(),
                "from_grade": format!("{from:?}"),
                "to_grade": format!("{to:?}"),
                "score_total": breakdown.total,
                "score_breakdown": breakdown,
                "decision": decision_label,
            });
            sink.write_with_tenant(
                None,
                AuditKind::TrustGradeChanged,
                payload,
                None,
                &tenant.to_string(),
            )
            .await?;
            Ok(decision)
        }
    }
}

/// Walk every `(tenant_id, worker_id, trust_grade)` triple in the
/// `worker` table and recompute each. Returns the count of workers
/// whose grade actually moved this tick. Per-worker errors are
/// logged and do not abort the whole sweep.
pub(crate) async fn tick_all_workers(store: Arc<SessionStore>) -> ArgoResult<usize> {
    let rows = scan_workers(&store).await?;
    if rows.is_empty() {
        return Ok(0);
    }
    let mut changed = 0_usize;
    for (tenant_uuid, worker_id_str, trust_u8) in rows {
        let tenant = TenantId(tenant_uuid);
        let current_grade = u8_to_grade(trust_u8).unwrap_or(TrustGrade::LearnedUnreviewed);
        let sink = AuditSink::new_with_tenant(Arc::clone(&store), tenant.to_string());
        match recompute_worker(&store, &sink, &tenant, &worker_id_str, current_grade).await {
            Ok(RatchetDecision::Hold) => {}
            Ok(decision) => {
                changed += 1;
                tracing::info!(
                    target: "trust_engine::tick",
                    tenant_id = %tenant,
                    worker_id = %worker_id_str,
                    decision = ?decision,
                    "trust grade moved"
                );
            }
            Err(e) => {
                tracing::warn!(
                    target: "trust_engine::tick",
                    tenant_id = %tenant,
                    worker_id = %worker_id_str,
                    error = %e,
                    "per-worker recompute failed"
                );
            }
        }
    }
    Ok(changed)
}

/// Dry-run variant of [`tick_all_workers`]. Computes scores and the
/// ratchet decision but skips both the identity-store write and the
/// audit-emit; logs each decision for operator inspection.
pub(crate) async fn tick_all_workers_dry_run(store: Arc<SessionStore>) -> ArgoResult<usize> {
    let rows = scan_workers(&store).await?;
    let mut would_change = 0_usize;
    for (tenant_uuid, worker_id_str, trust_u8) in rows {
        let tenant = TenantId(tenant_uuid);
        let current_grade = u8_to_grade(trust_u8).unwrap_or(TrustGrade::LearnedUnreviewed);
        let breakdown = match scoring::compute_score(&store, &tenant, &worker_id_str).await {
            Ok(b) => b,
            Err(e) => {
                tracing::warn!(
                    target: "trust_engine::tick",
                    error = %e,
                    "dry-run scoring failed"
                );
                continue;
            }
        };
        let decision = ratchet::ratchet_grade(current_grade, breakdown.total);
        if decision != RatchetDecision::Hold {
            would_change += 1;
        }
        tracing::info!(
            target: "trust_engine::tick",
            tenant_id = %tenant,
            worker_id = %worker_id_str,
            score = breakdown.total,
            decision = ?decision,
            "dry-run trust decision"
        );
    }
    Ok(would_change)
}

async fn scan_workers(
    store: &SessionStore,
) -> ArgoResult<Vec<(uuid::Uuid, String, u8)>> {
    let rows: Vec<(String, String, i64)> = sqlx::query_as(
        "SELECT tenant_id, id, COALESCE(trust_grade, 0) \
         FROM worker WHERE archived_at IS NULL",
    )
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("trust_engine worker scan: {e}")))?;
    let mut out = Vec::with_capacity(rows.len());
    for (tenant_raw, worker_raw, trust_raw) in rows {
        let Ok(tenant_uuid) = uuid::Uuid::parse_str(&tenant_raw) else {
            continue;
        };
        let trust = u8::try_from(trust_raw.max(0)).unwrap_or(0);
        out.push((tenant_uuid, worker_raw, trust));
    }
    Ok(out)
}
