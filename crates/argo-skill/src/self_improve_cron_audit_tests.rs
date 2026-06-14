//! DEC-042 p1c — the distill + proposal events made tamper-evident.
//!
//! The CHECKABLE VERIFIER for the "audited" leg: one self-improve tick over a
//! qualifying cohort emits `skill.distilled` → `skill.proposed_for_review` →
//! `skill.self_improve.decision` as ONE end-to-end-verifying hash-chain.
//! Reuses the parent module's seed helpers (`super::`).

use super::{seed_cluster, seed_loser_family, seed_stable_family, skills};
use crate::self_improve_cron::{self_improve_tick, DECISION_AUDIT_KIND};
use crate::store::SkillStore;
use argo_types::TenantId;
use sha2::{Digest, Sha256};

/// The three self-improve chain kinds, in seq order (excludes the trace rows
/// the seed inserts, which are also `session_id IS NULL`). Returns
/// `(kind, payload_json, parent_hash, self_hash)` per row.
async fn chain_rows(skills: &SkillStore) -> Vec<(String, String, Vec<u8>, Vec<u8>)> {
    sqlx::query_as(
        "SELECT kind, payload_json, parent_hash, self_hash FROM audit_record \
         WHERE session_id IS NULL AND kind IN \
           ('skill.distilled', 'skill.proposed_for_review', ?) \
         ORDER BY seq ASC",
    )
    .bind(DECISION_AUDIT_KIND)
    .fetch_all(skills.pool())
    .await
    .expect("chain rows")
}

/// Recompute `self_hash` exactly as `self_improve_cron_helpers::chain_hash`
/// does (sha256 of parent ‖ kind ‖ NUL ‖ payload) — so the test PROVES the
/// chain rather than trusting the stored bytes.
fn recompute(parent: &[u8], kind: &str, payload: &str) -> Vec<u8> {
    let mut h = Sha256::new();
    h.update(parent);
    h.update(kind.as_bytes());
    h.update(b"\0");
    h.update(payload.as_bytes());
    h.finalize().to_vec()
}

/// Criterion (a): the Hold (stable) family yields a clean three-row chain
/// (no promotion row interleaves), in order, hash-verified end-to-end.
#[tokio::test]
async fn tick_emits_distilled_proposed_decision_in_one_verified_chain() {
    let skills = skills().await;
    let tenant = TenantId::new().to_string();
    seed_stable_family(&skills, "stable-skill", "refund window is 30 days").await;
    seed_cluster(&skills, &tenant, "stable-skill").await;

    self_improve_tick(&skills).await.expect("tick runs");

    let chain = chain_rows(&skills).await;
    assert_eq!(chain.len(), 3, "exactly three self-improve chain rows");
    assert_eq!(chain[0].0, "skill.distilled");
    assert_eq!(chain[1].0, "skill.proposed_for_review");
    assert_eq!(chain[2].0, DECISION_AUDIT_KIND);

    // Each self_hash is a REAL hash of its own (parent, kind, payload)…
    for (kind, payload, parent, self_hash) in &chain {
        assert_eq!(
            &recompute(parent, kind, payload),
            self_hash,
            "row {kind} self_hash must derive from parent+kind+payload"
        );
    }
    // …and the links are contiguous: each parent is the prior self_hash.
    assert_eq!(chain[1].2, chain[0].3, "proposed chains onto distilled");
    assert_eq!(chain[2].2, chain[1].3, "decision chains onto proposed");

    // Payloads carry the load-bearing provenance fields.
    let distilled: serde_json::Value =
        serde_json::from_str(&chain[0].1).expect("distilled json");
    assert_eq!(distilled["tenant_id"], tenant);
    assert_eq!(distilled["model"], "deterministic");
    assert_eq!(distilled["cost_usd_micros"], 0);
    assert_eq!(distilled["action"], "refund");
    assert!(distilled["skill_name"].as_str().is_some_and(|s| !s.is_empty()));

    let proposed: serde_json::Value =
        serde_json::from_str(&chain[1].1).expect("proposed json");
    assert_eq!(proposed["source"], "distilled");
    assert_eq!(proposed["vertical"], "support");
    assert!(proposed["learned_in"].as_str().is_some_and(|s| !s.is_empty()));
}

/// A gated-Reject candidate still emits `skill.distilled` (the provider spend
/// happened) but NOT `skill.proposed_for_review` (it never enters the review
/// queue) — the chain is distilled → decision.
#[tokio::test]
async fn rejected_candidate_emits_distilled_but_not_proposed() {
    let skills = skills().await;
    let tenant = TenantId::new().to_string();
    seed_loser_family(&skills, "loser-skill", "refund window is 30 days").await;
    seed_cluster(&skills, &tenant, "loser-skill").await;

    self_improve_tick(&skills).await.expect("tick runs");

    let chain = chain_rows(&skills).await;
    let kinds: Vec<&str> = chain.iter().map(|r| r.0.as_str()).collect();
    assert_eq!(
        kinds,
        vec!["skill.distilled", DECISION_AUDIT_KIND],
        "reject path: distilled + decision, no proposed_for_review"
    );
    assert_eq!(
        chain[1].2, chain[0].3,
        "decision chains directly onto distilled when proposal is skipped"
    );
}
