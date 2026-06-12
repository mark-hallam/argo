//! Tests for the self-improvement cron driver.
//!
//! The full chain runs against in-memory fixtures with NO live model:
//! seeded trace audit rows cluster into a cohort, the deterministic
//! distillation proposes a candidate, the A/B-by-eval measures a real
//! before/after delta, the promotion gate decides + applies, and one
//! hash-chained, tenant-stamped decision audit row lands per cluster.

use super::*;
use argo_store::SessionStore;
use argo_types::TenantId;
use std::sync::Arc;
use uuid::Uuid;

async fn skills() -> SkillStore {
    let session = Arc::new(SessionStore::open(":memory:").await.expect("store"));
    session.migrate().await.expect("migrate");
    SkillStore::new(session)
}

/// A `SKILL.md` whose single eval case requires the output to CONTAIN
/// `required`. The deterministic `BodyContainsExecutor` returns the body,
/// so the case passes iff `body` contains `required`.
fn skill_doc(name: &str, version: &str, body: &str, required: &str) -> String {
    format!(
        "---\n\
         name: {name}\n\
         version: \"{version}\"\n\
         description: a self-improving skill\n\
         provenance:\n  source: builtin\n  hash: sha256-test\n\
         evals:\n  - input: \"q\"\n    expected:\n      kind: contains\n      text: \"{required}\"\n\
         ---\n\
         {body}\n"
    )
}

/// Install a baseline (v1, fails its eval) + supersede with an improved
/// candidate (v2, passes) — a clear A/B winner.
async fn seed_winner_family(skills: &SkillStore, name: &str, required: &str) {
    skills
        .install(&skill_doc(name, "v1", "be helpful", required), None)
        .await
        .expect("install baseline");
    skills
        .supersede(
            &skill_doc(name, "v2", &format!("be helpful; {required}"), required),
            None,
            crate::skill::PrincipalOverride::Agent,
        )
        .await
        .expect("supersede candidate");
}

/// Install two versions that both already pass — no measured improvement
/// (delta 0), which `decide_promotion` resolves to `Hold`.
async fn seed_stable_family(skills: &SkillStore, name: &str, required: &str) {
    let good = format!("be helpful; {required}");
    skills
        .install(&skill_doc(name, "v1", &good, required), None)
        .await
        .expect("install");
    skills
        .supersede(
            &skill_doc(name, "v2", &good, required),
            None,
            crate::skill::PrincipalOverride::Agent,
        )
        .await
        .expect("supersede");
}

/// Install a baseline (v1, PASSES its eval) superseded by a candidate
/// (v2, FAILS its eval) — a clear A/B LOSER: the candidate regresses
/// 1.0 -> 0.0, past the 0.05 reject floor, so `decide_promotion` -> Reject.
async fn seed_loser_family(skills: &SkillStore, name: &str, required: &str) {
    let good = format!("be helpful; {required}");
    skills
        .install(&skill_doc(name, "v1", &good, required), None)
        .await
        .expect("install baseline");
    skills
        .supersede(
            &skill_doc(name, "v2", "be helpful", required),
            None,
            crate::skill::PrincipalOverride::Agent,
        )
        .await
        .expect("supersede candidate");
}

/// Insert one `TRACE_AUDIT_KIND` audit row for `tenant` whose payload
/// names `skill_name` (baseline v1 / candidate v2) and carries
/// `action_sequence` + `success`.
async fn insert_trace(
    skills: &SkillStore,
    tenant: &str,
    skill_name: &str,
    action_sequence: &[&str],
    success: f64,
) {
    let payload = serde_json::json!({
        "vertical_id": "support",
        "action_type": "refund",
        "trajectory": action_sequence,
        "final_text": "done",
        "success_metric": success,
        "skill_name": skill_name,
        "baseline_version": "v1",
        "candidate_version": "v2",
    });
    sqlx::query(
        "INSERT INTO audit_record \
         (id, session_id, kind, payload_json, parent_hash, self_hash, created_at, tenant_id) \
         VALUES (?, NULL, ?, ?, ?, ?, ?, ?)",
    )
    .bind(Uuid::now_v7().as_bytes().as_slice())
    .bind(TRACE_AUDIT_KIND)
    .bind(payload.to_string())
    .bind([0u8; 32].as_slice())
    .bind([0u8; 32].as_slice())
    .bind(1_000_i64)
    .bind(tenant)
    .execute(skills.pool())
    .await
    .expect("insert trace");
}

/// Seed three similar traces (one cluster) for a family + tenant.
async fn seed_cluster(skills: &SkillStore, tenant: &str, family: &str) {
    let seq = ["lookup", "verify", "refund", "confirm"];
    for _ in 0..3 {
        insert_trace(skills, tenant, family, &seq, 0.9).await;
    }
}

async fn decision_rows(skills: &SkillStore) -> Vec<serde_json::Value> {
    let rows: Vec<(String, Option<String>)> = sqlx::query_as(
        "SELECT payload_json, tenant_id FROM audit_record WHERE kind = ?",
    )
    .bind(DECISION_AUDIT_KIND)
    .fetch_all(skills.pool())
    .await
    .expect("decision rows");
    rows.into_iter()
        .map(|(p, _)| serde_json::from_str(&p).expect("payload json"))
        .collect()
}

#[tokio::test]
async fn full_chain_promotes_a_clear_winner() {
    let skills = skills().await;
    let tenant = TenantId::new().to_string();
    seed_winner_family(&skills, "refund-skill", "refund window is 30 days").await;
    seed_cluster(&skills, &tenant, "refund-skill").await;

    let report = self_improve_tick(&skills).await.expect("tick runs");

    assert_eq!(report.traces_read, 3, "all three traces read");
    assert_eq!(report.candidate_clusters, 1, "one distillation candidate");
    assert_eq!(report.promoted, 1, "the clear winner is promoted");
    assert_eq!(report.outcomes.len(), 1, "one decision per cluster");
    let outcome = &report.outcomes[0];
    assert_eq!(outcome.skill_name, "refund-skill");
    assert_eq!(outcome.tenant_id, tenant);
    assert!(outcome.decision.is_promote(), "got {:?}", outcome.decision);
    assert!(outcome.proposed_skill_id.is_some(), "distillation proposed a skill");

    // promote_with_regression_gate actually applied: v1 (loser) is now
    // superseded_by the v2 winner.
    let superseded: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM skill WHERE name = ? AND version = 'v1' AND superseded_by IS NOT NULL",
    )
    .bind("refund-skill")
    .fetch_one(skills.pool())
    .await
    .expect("count superseded");
    assert_eq!(superseded, 1, "baseline v1 superseded by the promoted v2");
}

#[tokio::test]
async fn decision_audit_row_is_greppable_by_skill_and_tenant() {
    let skills = skills().await;
    let tenant = TenantId::new().to_string();
    seed_winner_family(&skills, "refund-skill", "refund window is 30 days").await;
    seed_cluster(&skills, &tenant, "refund-skill").await;

    self_improve_tick(&skills).await.expect("tick runs");

    let rows = decision_rows(&skills).await;
    assert_eq!(rows.len(), 1, "exactly one decision audit row");
    let row = &rows[0];
    assert_eq!(row["tenant_id"], tenant, "tenant stamped");
    assert_eq!(row["skill_name"], "refund-skill", "greppable by skill name");
    assert_eq!(row["outcome_status"], "promote");
    assert_eq!(row["candidate_version"], "v2");
    // skill_id is the candidate version's real skill id (greppable).
    assert!(
        row["skill_id"].as_str().is_some_and(|s| !s.is_empty()),
        "decision row carries the candidate skill id"
    );
    // The measured delta is real (0.0 -> 1.0), not a rig.
    assert!((row["pass_rate_delta"].as_f64().unwrap() - 1.0).abs() < 1e-9);
}

#[tokio::test]
async fn tenant_a_tick_never_clusters_tenant_b_traces() {
    let skills = skills().await;
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    seed_winner_family(&skills, "family-a", "alpha guidance").await;
    seed_winner_family(&skills, "family-b", "beta guidance").await;
    seed_cluster(&skills, &tenant_a.to_string(), "family-a").await;
    seed_cluster(&skills, &tenant_b.to_string(), "family-b").await;

    let config = SelfImproveConfig {
        tenant_id: Some(tenant_a),
        ..SelfImproveConfig::default()
    };
    let report = self_improve_tick_with(
        &skills,
        config,
        None,
        &DeterministicDistillationCaller,
    )
    .await
    .expect("tick runs");

    // Only tenant A's three traces are read; tenant B is invisible.
    assert_eq!(report.traces_read, 3, "tenant filter applied in SQL");
    assert_eq!(report.outcomes.len(), 1);
    assert_eq!(report.outcomes[0].tenant_id, tenant_a.to_string());
    assert_eq!(report.outcomes[0].skill_name, "family-a");

    // No decision row was ever written for tenant B.
    let rows = decision_rows(&skills).await;
    assert!(
        rows.iter().all(|r| r["tenant_id"] == tenant_a.to_string()),
        "no tenant-b decision leaked: {rows:?}"
    );
}

#[tokio::test]
async fn no_improvement_holds_and_does_not_promote() {
    let skills = skills().await;
    let tenant = TenantId::new().to_string();
    seed_stable_family(&skills, "stable-skill", "refund window is 30 days").await;
    seed_cluster(&skills, &tenant, "stable-skill").await;

    let report = self_improve_tick(&skills).await.expect("tick runs");

    assert_eq!(report.promoted, 0, "no improvement → no promotion");
    assert_eq!(report.outcomes.len(), 1, "still records the decision");
    assert!(
        matches!(report.outcomes[0].decision, PromotionDecision::Hold { .. }),
        "got {:?}",
        report.outcomes[0].decision
    );
    // A decision audit row is written even on Hold.
    let rows = decision_rows(&skills).await;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["outcome_status"], "hold");
}

/// Pending-review (distilled-proposal) skill rows scoped to a tenant —
/// mirrors the strict `tenant_id = ?` predicate the per-tenant reads use,
/// so it proves the row is structurally invisible to any other tenant.
async fn pending_review_tenants(skills: &SkillStore) -> Vec<String> {
    sqlx::query_scalar::<_, String>(
        "SELECT tenant_id FROM skill \
         WHERE lifecycle_state = 'pending_review' AND superseded_by IS NULL",
    )
    .fetch_all(skills.pool())
    .await
    .expect("pending-review tenant rows")
}

/// DEC-042 Stage-2 P-1 (dec042-p1a): the self-improve tick reads traces
/// tenant-scoped, so the distilled proposal it lands must carry the SAME
/// tenant — NOT the migration placeholder.
#[tokio::test]
async fn distilled_skill_row_is_bound_to_originating_tenant() {
    let skills = skills().await;
    let tenant = TenantId::new().to_string();
    seed_winner_family(&skills, "refund-skill", "refund window is 30 days").await;
    seed_cluster(&skills, &tenant, "refund-skill").await;

    let report = self_improve_tick(&skills).await.expect("tick runs");
    assert!(
        report.outcomes[0].proposed_skill_id.is_some(),
        "distillation proposed a skill to bind"
    );

    // The single distilled (pending_review) row is bound to T, not the
    // placeholder.
    let pending = pending_review_tenants(&skills).await;
    assert_eq!(pending.len(), 1, "one distilled proposal: {pending:?}");
    assert_eq!(pending[0], tenant, "distilled row bound to originating tenant");
    assert_ne!(
        pending[0],
        crate::store::PLACEHOLDER_TENANT_ID,
        "must NOT land under the migration placeholder"
    );
}

/// Cross-tenant negative pin: tenant B's tick lands B's distilled
/// proposal under B; tenant A NEVER sees a pending row under A's id (and
/// vice-versa). The strict per-tenant read filter (`tenant_id = ?`) makes
/// each tenant's distilled proposal invisible to the other.
#[tokio::test]
async fn distilled_proposal_never_leaks_across_tenants() {
    let skills = skills().await;
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    seed_winner_family(&skills, "family-b", "beta guidance").await;
    seed_cluster(&skills, &tenant_b.to_string(), "family-b").await;

    // Run the tick scoped to tenant B only.
    let config = SelfImproveConfig {
        tenant_id: Some(tenant_b),
        ..SelfImproveConfig::default()
    };
    self_improve_tick_with(&skills, config, None, &DeterministicDistillationCaller)
        .await
        .expect("tick runs");

    let pending = pending_review_tenants(&skills).await;
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0], tenant_b.to_string(), "bound to B");
    assert!(
        !pending.iter().any(|t| t == &tenant_a.to_string()),
        "tenant A must never see B's distilled proposal: {pending:?}"
    );
    assert!(
        !pending.iter().any(|t| t == crate::store::PLACEHOLDER_TENANT_ID),
        "no placeholder-bound leak: {pending:?}"
    );
}

/// DEC-042 Stage-2 P-1 (dec042-p1b): the eval floor is a GATE, not
/// advisory. A candidate whose measured A/B eval triggers `Reject` must
/// NOT sit in `pending_review` after the tick — it is installed as
/// `Rejected` (row exists for audit, never queued for operator approval).
#[tokio::test]
async fn rejected_candidate_is_not_in_pending_review() {
    let skills = skills().await;
    let tenant = TenantId::new().to_string();
    seed_loser_family(&skills, "loser-skill", "refund window is 30 days").await;
    seed_cluster(&skills, &tenant, "loser-skill").await;

    let report = self_improve_tick(&skills).await.expect("tick runs");

    assert_eq!(report.promoted, 0, "a regression is never promoted");
    assert_eq!(report.outcomes.len(), 1, "still records the decision");
    assert!(
        matches!(report.outcomes[0].decision, PromotionDecision::Reject { .. }),
        "got {:?}",
        report.outcomes[0].decision
    );
    // The negative pin: NOTHING is in pending_review — the distilled
    // proposal was gated to Rejected before the queue, not after.
    assert!(
        pending_review_tenants(&skills).await.is_empty(),
        "a rejected candidate must NOT be in pending_review"
    );
    // Audit coherence: the distilled row still EXISTS, as Rejected, bound
    // to the originating tenant.
    let rejected: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM skill WHERE lifecycle_state = 'rejected' AND tenant_id = ?",
    )
    .bind(&tenant)
    .fetch_one(skills.pool())
    .await
    .expect("count rejected");
    assert_eq!(
        rejected, 1,
        "distilled proposal installed as Rejected, bound to tenant"
    );
    assert!(
        report.outcomes[0].proposed_skill_id.is_some(),
        "the gated (Rejected) proposal still carries a skill id for the audit row"
    );
    // Decision-audit coverage preserved on Reject.
    let rows = decision_rows(&skills).await;
    assert_eq!(rows.len(), 1, "one decision audit row even on reject");
    assert_eq!(rows[0]["outcome_status"], "reject");
}

#[tokio::test]
async fn empty_audit_log_is_a_clean_noop() {
    let skills = skills().await;
    let report = self_improve_tick(&skills).await.expect("tick runs");
    assert_eq!(report.traces_read, 0);
    assert_eq!(report.candidate_clusters, 0);
    assert_eq!(report.promoted, 0);
    assert!(report.outcomes.is_empty());
}

/// Tenant-purity guard (adversarial finding 2026-06-12): on an UNSCOPED
/// tick (`tenant_id = None`, the shipped default) similarity clustering is
/// tenant-blind — tenants A and B seeding the same trajectory shape land in
/// ONE cluster. The guard must skip the mixed cluster entirely: one
/// tenant's trace content must never distill into a skill bound (and
/// visible) to another tenant.
#[tokio::test]
async fn unscoped_tick_skips_clusters_spanning_multiple_tenants() {
    let skills = skills().await;
    let tenant_a = TenantId::new().to_string();
    let tenant_b = TenantId::new().to_string();
    seed_winner_family(&skills, "mix-skill", "shared guidance").await;
    seed_cluster(&skills, &tenant_a, "mix-skill").await;
    seed_cluster(&skills, &tenant_b, "mix-skill").await;

    let report = self_improve_tick(&skills).await.expect("tick runs");

    assert_eq!(report.traces_read, 6, "unscoped tick reads both tenants");
    assert!(
        report.outcomes.is_empty(),
        "mixed-tenant cluster must be skipped, not distilled: {:?}",
        report.outcomes
    );
    assert!(
        pending_review_tenants(&skills).await.is_empty(),
        "no distilled proposal may land from a mixed-tenant cluster"
    );
}
