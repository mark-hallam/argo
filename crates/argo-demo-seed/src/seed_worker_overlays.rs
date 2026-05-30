//! Per-worker overlay seeders — skill assignments + committed-retrain
//! history for the canonical v1 demo hero workers.
//!
//! Split out of `seed.rs` to keep that file under the 500-LOC ceiling
//! (mechanical extraction, no behavior change). Both entry points are
//! called from `crate::seed::run_canonical_v1_seed` after the skill
//! catalog + worker rows exist. Mirrors the `skill_ir_eval` sibling-
//! module precedent.

use std::sync::Arc;

use argo_audit::{AuditKind, AuditSink};
use argo_identity::IdentityStore;
use argo_store::SessionStore;
use argo_types::TenantId;
use chrono::Utc;
use uuid::Uuid;

use crate::SeedResult;

/// Assign a couple of real catalog skills to each hero worker so Claim 4's
/// adoption surfaces (`/skills/<id>/usage`, "skills learned this month")
/// show genuine usage instead of an empty state. The skill ids are the
/// same tenant-scoped `skill` rows `seed_demo_marketplace_skills` created;
/// `worker_skill.created_at` defaults to now, so each row lands inside the
/// current month. Real data — the workers genuinely carry these skills in
/// their layered lookup. Idempotent: `add_worker_skill` upserts.
pub(crate) async fn seed_worker_skill_assignments(
    store: &Arc<SessionStore>,
    tenant_id: TenantId,
    worker_ids: &[argo_identity::WorkerId],
) -> SeedResult<usize> {
    if worker_ids.is_empty() {
        return Ok(0);
    }
    // Catalog skill ids for the tenant (16-byte blob → hyphenated uuid,
    // the form `/skills` + `worker_skill.skill_id` key on).
    let blobs: Vec<Vec<u8>> = sqlx::query_scalar(
        "SELECT id FROM skill \
         WHERE tenant_id = ? AND lifecycle_state = 'active' AND superseded_by IS NULL \
         ORDER BY name ASC LIMIT 8",
    )
    .bind(tenant_id.to_string())
    .fetch_all(store.pool())
    .await
    .map_err(|e| argo_types::ArgoError::Provider(format!("list skills for assignment: {e}")))?;
    let skill_ids: Vec<String> = blobs
        .iter()
        .filter(|b| b.len() == 16)
        .map(|b| {
            let mut a = [0u8; 16];
            a.copy_from_slice(b);
            Uuid::from_bytes(a).to_string()
        })
        .collect();
    if skill_ids.is_empty() {
        return Ok(0);
    }
    let identities = IdentityStore::new(store);
    let mut assigned = 0usize;
    // Each worker learns two skills (round-robin over the catalog) so
    // per-skill usage counts are non-trivial and >0 workers appear.
    for (i, wid) in worker_ids.iter().enumerate() {
        for k in 0..2usize {
            let sid = &skill_ids[(i + k) % skill_ids.len()];
            identities.add_worker_skill(*wid, sid, 70 + (k as i64) * 10).await?;
            assigned += 1;
        }
    }
    Ok(assigned)
}

/// Seed a handful of committed retrains for the lead hero workers so
/// the worker Overview tab's "Recent changes" panel renders real
/// history (newest-first, inside the 14-day undo window) instead of an
/// empty mock. Writes `worker.retrain.committed` audit rows — the exact
/// shape `routes_retrain::commit` persists and `GET /retrain/recent`
/// reads back — so this is real data, not a parallel store. Idempotent:
/// if any committed-retrain row already exists for the tenant, the set
/// was seeded by a prior run and the whole block is skipped (re-seeding
/// to add a teammate must not duplicate the panel).
pub(crate) async fn seed_retrain_records(
    store: &Arc<SessionStore>,
    tenant_id: TenantId,
    worker_ids: &[argo_identity::WorkerId],
) -> SeedResult<usize> {
    if worker_ids.is_empty() {
        return Ok(0);
    }
    let existing: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_record WHERE kind = 'http.request' \
           AND tenant_id = ? \
           AND json_extract(payload_json, '$.endpoint') = 'worker.retrain.committed'",
    )
    .bind(tenant_id.to_string())
    .fetch_one(store.pool())
    .await
    .map_err(|e| argo_types::ArgoError::Provider(format!("retrain seed probe: {e}")))?;
    if existing > 0 {
        return Ok(0);
    }

    // (worker index, classification, structural_change, days_ago) — one
    // of each of the five classifications so every mapped badge renders.
    let plan: [(usize, &str, serde_json::Value, i64); 5] = [
        (
            0,
            "memory_fact",
            serde_json::json!({ "text": "Northwind pays NET-30, not NET-15." }),
            2,
        ),
        (
            0,
            "vocabulary_patch",
            serde_json::json!({ "from": "client", "to": "customer" }),
            4,
        ),
        (
            0,
            "policy_rule",
            serde_json::json!({
                "predicate_json": { "raw": "Always CC the account owner on refunds over $500." },
                "action_type": "refund",
            }),
            6,
        ),
        (
            1,
            "skill_correction",
            serde_json::json!({ "note": "Use the Q2 pricing sheet, not Q1." }),
            3,
        ),
        (
            1,
            "action_ban",
            serde_json::json!({ "action_type": "send_sms" }),
            5,
        ),
    ];

    let sink = AuditSink::new_with_tenant(Arc::clone(store), tenant_id.to_string());
    let mut inserted = 0usize;
    for (idx, classification, structural_change, days_ago) in plan {
        let worker_id = worker_ids[idx % worker_ids.len()];
        let retrain_id = Uuid::now_v7();
        let applied_at = (Utc::now() - chrono::Duration::days(days_ago)).to_rfc3339();
        sink.write(
            None,
            AuditKind::HttpRequest,
            serde_json::json!({
                "endpoint": "worker.retrain.committed",
                "retrain_id": retrain_id.to_string(),
                "tenant_id": tenant_id.to_string(),
                "worker_id": worker_id.to_string(),
                "classification": classification,
                "structural_change": structural_change,
                "applied_at": applied_at,
            }),
            None,
        )
        .await?;
        inserted += 1;
    }
    sink.flush_pending().await?;
    Ok(inserted)
}
