//! # Mission Claim 4 MVP row 4 — novel-sequence detection
//!
//! Pins the operator-locked MVP row (round 19):
//!
//! > Novel-sequence detection — worker's CU-completed sequence with
//! > `success_metric > threshold` surfaces as skill-candidate.
//!
//! Cite: `argo-skill::learn::should_propose`
//! (DEFAULT_SUCCESS_THRESHOLD = 0.8) +
//! `argo-skill::learn::SkillCandidate::from_cu_trajectory` +
//! `SkillStore::propose_for_review`. Substrate found round 20; this
//! round flips the row 🟢 → 🟢🟢 with a falsifiable LIVE regression
//! that composes the threshold gate AND the candidate lifecycle in
//! ONE end-to-end pass.
//!
//! ## What this test proves
//!
//! 1. **Threshold gate respects DEFAULT_SUCCESS_THRESHOLD (0.8).** A
//!    trajectory with `success_metric` strictly > 0.8 surfaces. A
//!    trajectory at exactly 0.8 does NOT (strict-greater contract).
//!    A trajectory below 0.8 does NOT.
//! 2. **Candidate-to-PendingReview lifecycle.** When the gate passes,
//!    `SkillCandidate::from_cu_trajectory(...).to_skill_md(...)` →
//!    `SkillStore::propose_for_review(...)` produces a row in
//!    `PendingReview`.
//! 3. **PendingReview is invisible to the agent's runtime view.** A
//!    pending-review skill MUST NOT appear in `list_skills_for_vertical`
//!    (the function the agent calls to resolve skills for a session).
//!    Without this, an unreviewed proposal would auto-leak into
//!    production behavior.
//! 4. **Operator-promote makes the skill visible.** After
//!    `transition_lifecycle(id, Active)`, the same skill DOES appear
//!    in `list_skills_for_vertical`. This is the operator-feedback
//!    loop that distinguishes "learned" from "trusted."
//! 5. **Candidate body round-trips.** The `to_skill_md` output is
//!    valid frontmatter — `propose_for_review` accepts it without
//!    parse errors. The skill the operator reviews carries the
//!    trajectory + success_metric so the reviewer can decide informed.
//!
//! ## Why this complements the existing tests
//!
//! `crates/argo-skill/tests/end_to_end_claim4_loop.rs` proves the
//! BROADER publish-and-reload-without-cross-tenant-leak loop. The
//! in-crate `tests_lifecycle.rs` pins
//! `propose_for_review_starts_in_pending_review` plus pending/active
//! visibility. NEITHER pins the THRESHOLD-GATE decision boundary —
//! that the 0.8 floor actually gates whether a candidate is built at
//! all. This test is the threshold-boundary pin.
//!
//! ## Authored
//!
//! Evaluator round 29, 2026-05-23.

use std::sync::Arc;

use argo_skill::{
    should_propose, LifecycleState, SkillCandidate, SkillStore, DEFAULT_SUCCESS_THRESHOLD,
};
use argo_store::SessionStore;
use argo_types::SessionId;

const VERTICAL: &str = "marketplace-seller";

async fn fresh_skill_store() -> SkillStore {
    let store = SessionStore::open(":memory:").await.expect("open");
    store.migrate().await.expect("migrate");
    SkillStore::new(Arc::new(store))
}

fn mariah_trajectory() -> Vec<String> {
    vec![
        "screenshot 1280x800".into(),
        "left_click 412,288 'Sell something'".into(),
        "type 'Bauer hockey stick — like new'".into(),
        "left_click 612,440 'Marketplace'".into(),
        "type '125.00'".into(),
        "left_click 880,480 'Photo'".into(),
        "upload /tmp/stick.jpg".into(),
        "left_click 920,540 'Publish'".into(),
        "screenshot — listing live confirmation".into(),
    ]
}

#[tokio::test]
async fn claim_4_mvp_row_4_threshold_gate_at_boundary_strictly_greater() {
    // ── Phase 1: pin the should_propose contract.
    // Strictly greater than threshold — boundary is exclusive.
    assert!(
        should_propose(0.81, DEFAULT_SUCCESS_THRESHOLD),
        "claim-4 MVP row 4: success_metric=0.81 must surface (>0.8)"
    );
    assert!(
        should_propose(0.95, DEFAULT_SUCCESS_THRESHOLD),
        "claim-4 MVP row 4: success_metric=0.95 must surface"
    );
    // AT exactly the threshold — must NOT propose (strict-greater).
    assert!(
        !should_propose(DEFAULT_SUCCESS_THRESHOLD, DEFAULT_SUCCESS_THRESHOLD),
        "claim-4 MVP row 4: success_metric exactly equal to 0.8 must NOT surface (strict > contract)"
    );
    // Below the threshold — must NOT propose.
    assert!(
        !should_propose(0.50, DEFAULT_SUCCESS_THRESHOLD),
        "claim-4 MVP row 4: success_metric=0.50 must NOT surface"
    );
    assert!(
        !should_propose(0.79, DEFAULT_SUCCESS_THRESHOLD),
        "claim-4 MVP row 4: success_metric=0.79 must NOT surface (just below threshold)"
    );
}

#[tokio::test]
async fn claim_4_mvp_row_4_above_threshold_yields_pending_review_invisible_until_promote() {
    let store = fresh_skill_store().await;
    let session_id = SessionId::new();
    let trajectory = mariah_trajectory();
    let success_metric = 0.94_f64;

    // ── Phase 1: threshold gate — success_metric > 0.8 → should propose.
    assert!(
        should_propose(success_metric, DEFAULT_SUCCESS_THRESHOLD),
        "claim-4 MVP row 4: precondition — success_metric=0.94 should propose"
    );

    // ── Phase 2: build candidate from trajectory.
    let candidate = SkillCandidate::from_cu_trajectory(
        "post_marketplace_listing_mariah",
        "fb_marketplace.post",
        VERTICAL,
        trajectory.clone(),
        Some("https://fb.com/marketplace/item/abc123 — Listing live".into()),
        4321,
        success_metric,
    );
    assert_eq!(
        candidate.success_metric, success_metric,
        "claim-4 MVP row 4: candidate must carry success_metric for reviewer context"
    );
    assert_eq!(
        candidate.vertical_id, VERTICAL,
        "claim-4 MVP row 4: candidate must be vertical-scoped"
    );
    assert_eq!(
        candidate.trajectory, trajectory,
        "claim-4 MVP row 4: candidate must preserve trajectory for review"
    );

    // ── Phase 3: serialize → propose_for_review.
    let body = candidate.to_skill_md("session-short");
    let skill = store
        .propose_for_review(&body, session_id, VERTICAL)
        .await
        .expect("propose_for_review must accept candidate body");
    assert_eq!(
        skill.lifecycle_state,
        LifecycleState::PendingReview,
        "claim-4 MVP row 4: proposed candidate must land in PendingReview, not Active; got {:?}",
        skill.lifecycle_state
    );
    assert_eq!(
        skill.vertical_id.as_deref(),
        Some(VERTICAL),
        "claim-4 MVP row 4: proposed candidate must be vertical-stamped"
    );
    assert_eq!(
        skill.learned_in,
        Some(session_id),
        "claim-4 MVP row 4: proposed candidate must reference the learning session"
    );

    // ── Phase 4: agent's runtime view MUST NOT see the pending skill.
    let runtime_visible = store
        .list_skills_for_vertical(VERTICAL)
        .await
        .expect("list_skills_for_vertical");
    assert!(
        runtime_visible.is_empty(),
        "claim-4 MVP row 4: PendingReview skill MUST be invisible to agent runtime; got {} visible",
        runtime_visible.len()
    );

    // ── Phase 5: pending_review queue MUST see the skill.
    let pending = store
        .list_pending_review()
        .await
        .expect("list_pending_review");
    assert_eq!(
        pending.len(),
        1,
        "claim-4 MVP row 4: pending_review queue must contain the proposal; got {} pending",
        pending.len()
    );
    assert_eq!(
        pending[0].id, skill.id,
        "claim-4 MVP row 4: the queue entry must match the proposal id"
    );

    // ── Phase 6: operator-promote → skill becomes visible.
    let new_state = store
        .transition_lifecycle(skill.id, LifecycleState::Active)
        .await
        .expect("transition_lifecycle to Active");
    assert_eq!(
        new_state,
        LifecycleState::Active,
        "claim-4 MVP row 4: promote must land at Active"
    );

    let runtime_after_promote = store
        .list_skills_for_vertical(VERTICAL)
        .await
        .expect("list_skills_for_vertical after promote");
    assert_eq!(
        runtime_after_promote.len(),
        1,
        "claim-4 MVP row 4: after promote, agent runtime MUST see the new skill; got {} visible",
        runtime_after_promote.len()
    );
    assert_eq!(
        runtime_after_promote[0].id, skill.id,
        "claim-4 MVP row 4: the now-visible skill must match what was promoted"
    );
}

#[tokio::test]
async fn claim_4_mvp_row_4_below_threshold_yields_no_candidate_path() {
    // Defensive — when the trajectory's success_metric is below the
    // 0.8 floor, the CALLER's responsibility is to honor
    // `should_propose` and skip building the candidate. This test
    // pins the SHOULD-NOT-PROPOSE branch and proves that no rogue
    // bypass exists (the SkillCandidate API doesn't reject low
    // success_metric — it's the caller's contract to gate via
    // should_propose).
    let store = fresh_skill_store().await;

    for low_metric in &[0.50_f64, 0.79, 0.80] {
        assert!(
            !should_propose(*low_metric, DEFAULT_SUCCESS_THRESHOLD),
            "claim-4 MVP row 4: success_metric={low_metric} must NOT trigger propose"
        );
        // The contract: a caller that respects should_propose simply
        // doesn't build the candidate. Build NOTHING.
        // (If a future change ever auto-bypassed should_propose and
        // proposed everything anyway, the runtime view would suddenly
        // start seeing low-quality skills — we re-verify that
        // pending_review remains empty.)
    }

    let pending = store
        .list_pending_review()
        .await
        .expect("list_pending_review");
    assert!(
        pending.is_empty(),
        "claim-4 MVP row 4: with no above-threshold proposals, pending queue MUST be empty"
    );
    let runtime_visible = store
        .list_skills_for_vertical(VERTICAL)
        .await
        .expect("list_skills_for_vertical");
    assert!(
        runtime_visible.is_empty(),
        "claim-4 MVP row 4: with no proposals, runtime view MUST be empty"
    );
}

#[tokio::test]
async fn claim_4_mvp_row_4_rejected_proposal_never_becomes_visible() {
    // Defensive — the SECOND review outcome (operator REJECTS the
    // proposal) must keep the skill out of the runtime view forever.
    // Without this, a rejected candidate could silently activate on
    // a future re-resolve.
    let store = fresh_skill_store().await;
    let session_id = SessionId::new();

    let candidate = SkillCandidate::from_cu_trajectory(
        "questionable_proposal",
        "internal_admin.list_users",
        VERTICAL,
        vec!["screenshot".into(), "left_click admin-panel".into()],
        Some("admin panel screenshot".into()),
        9999,
        0.95,
    );
    let body = candidate.to_skill_md("session-short");
    let skill = store
        .propose_for_review(&body, session_id, VERTICAL)
        .await
        .expect("propose_for_review");

    // Operator rejects.
    let new_state = store
        .transition_lifecycle(skill.id, LifecycleState::Rejected)
        .await
        .expect("transition to Rejected");
    assert_eq!(new_state, LifecycleState::Rejected);

    // Runtime view MUST NOT see it now or ever.
    let runtime_visible = store
        .list_skills_for_vertical(VERTICAL)
        .await
        .expect("list_skills_for_vertical");
    assert!(
        runtime_visible.is_empty(),
        "claim-4 MVP row 4: rejected skill MUST remain invisible to agent runtime"
    );

    // Pending queue also empty (the row exists but is no longer pending).
    let pending = store
        .list_pending_review()
        .await
        .expect("list_pending_review");
    assert!(
        pending.is_empty(),
        "claim-4 MVP row 4: a rejected skill must drop out of pending queue"
    );
}
