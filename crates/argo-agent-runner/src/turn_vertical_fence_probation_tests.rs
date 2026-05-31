//! Store-backed proof that the Genesis P3 probation signal is resolved
//! at the runtime dispatch seam from the SAME worker-row read the
//! capability fence uses (DEC-025 #2).
//!
//! These exercise the REAL chain a freshly-hired teammate takes:
//! `IdentityStore::create_worker_from_draft` (the single hire chokepoint
//! both the flagship SSE path AND `/onboarding/first-hire` funnel
//! through) → persisted persona `provenance_chain` → the runtime
//! `genesis_signals_for_worker` read. Proving the runtime read reports
//! `InProbation` for a just-hired Genesis worker, and `Normal` for a
//! non-Genesis / no-worker session, closes the data-plane half of the
//! enforcement; the gate-precedence half is proven by
//! `turn_helpers_gate::gate_decision_tests`.

use super::{
    genesis_signals_for_worker, GenesisProvenance, ProbationStatus, TruthBoundaryBinding,
};
use argo_identity::{ApprovalPolicyChoice, HireDraft, HireDutyDraft, IdentityStore};
use argo_store::SessionStore;
use argo_types::TenantId;
use std::sync::Arc;

async fn store() -> Arc<SessionStore> {
    let store = Arc::new(SessionStore::open(":memory:").await.expect("open store"));
    store.migrate().await.expect("migrate");
    store
}

fn genesis_draft(plan_id: Option<&str>) -> HireDraft {
    HireDraft {
        suggested_name: "Maya".into(),
        charter: "Own outbound marketplace follow-up end-to-end.".into(),
        suggested_vertical: "marketplace-seller".into(),
        approval_policy: ApprovalPolicyChoice {
            level: "balanced".into(),
        },
        duties: vec![HireDutyDraft {
            cron: "0 9 * * 1-5".into(),
            entry_action: "follow_up".into(),
            description: "Send approved follow-up.".into(),
            default_enabled: true,
        }],
        voice_profile: None,
        tagline: None,
        genesis_plan_id: plan_id.map(str::to_string),
    }
}

#[tokio::test]
async fn freshly_hired_genesis_worker_is_in_probation_at_runtime() {
    // Hire through the REAL chokepoint with a Genesis plan id (the shape
    // both the SSE hire and /onboarding/first-hire produce — first-hire
    // sets `genesis_plan_id: Some(plan.vertical_id)`).
    let store = store().await;
    let tenant = TenantId::new();
    let id = IdentityStore::new(&store)
        .create_worker_from_draft(&genesis_draft(Some("marketing-intern-maya")), tenant, "human:founder")
        .await
        .expect("hire genesis worker");

    // The runtime read the dispatch seam performs at session boot must
    // report Genesis provenance (fence), active probation (P3), AND the
    // truth-boundary binding (P5 (a)) — all three from one worker-row read.
    let signals = genesis_signals_for_worker(&store, tenant, Some(id)).await;
    assert_eq!(signals.provenance, GenesisProvenance::Genesis);
    assert_eq!(
        signals.probation,
        ProbationStatus::InProbation,
        "a just-hired Genesis teammate must be in probation at the dispatch seam"
    );
    assert!(signals.probation.is_active());
    assert_eq!(
        signals.truth_boundary,
        TruthBoundaryBinding::Bound,
        "a Genesis teammate must carry the conservative truth-boundary binding (P5 a)"
    );
}

#[tokio::test]
async fn non_genesis_worker_is_not_in_probation_or_truth_boundary_bound() {
    // A worker hired WITHOUT a Genesis plan id (the pre-Genesis fallback)
    // is not a generated teammate: no fence, no probation window, no
    // auto-bound claim policy.
    let store = store().await;
    let tenant = TenantId::new();
    let id = IdentityStore::new(&store)
        .create_worker_from_draft(&genesis_draft(None), tenant, "human:founder")
        .await
        .expect("hire plain worker");

    let signals = genesis_signals_for_worker(&store, tenant, Some(id)).await;
    assert_eq!(signals.provenance, GenesisProvenance::None);
    assert_eq!(signals.probation, ProbationStatus::Normal);
    assert!(!signals.probation.is_active());
    assert_eq!(signals.truth_boundary, TruthBoundaryBinding::None);
}

#[tokio::test]
async fn ad_hoc_session_with_no_worker_is_not_in_probation() {
    // No `worker_id` (an ad-hoc chat session) → normal, no probation, no
    // truth-boundary binding.
    let store = store().await;
    let signals = genesis_signals_for_worker(&store, TenantId::new(), None).await;
    assert_eq!(signals.provenance, GenesisProvenance::None);
    assert_eq!(signals.probation, ProbationStatus::Normal);
    assert_eq!(signals.truth_boundary, TruthBoundaryBinding::None);
}

#[tokio::test]
async fn unknown_worker_id_degrades_to_normal_not_brick() {
    // A missing worker row (transient store miss) degrades to normal —
    // bricking the turn is worse than the historical binding. (Documented
    // fail-open-on-unreadable-row; flagged for the adversarial pass.)
    let store = store().await;
    let signals =
        genesis_signals_for_worker(&store, TenantId::new(), Some(argo_store::WorkerId::new())).await;
    assert_eq!(signals.provenance, GenesisProvenance::None);
    assert_eq!(signals.probation, ProbationStatus::Normal);
}
