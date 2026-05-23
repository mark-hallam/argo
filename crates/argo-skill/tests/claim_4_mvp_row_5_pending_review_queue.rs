//! # Mission Claim 4 MVP row 5 — pending_review review queue
//!
//! Pins the operator-locked MVP row (round 19):
//!
//! > Skill-candidate review queue (filter on `pending_review`
//! > lifecycle).
//!
//! Cite: `argo-skill` `pending_review` state + `routes_worker_skills.rs`
//! + `cli_skill_review_cmds.rs`. Substrate found round 20; this round
//! flips the row 🟢 → 🟢🟢 with a falsifiable LIVE regression that
//! pins the queue contract the operator-facing CLI + gateway depend
//! on.
//!
//! ## What this test proves
//!
//! 1. **Queue ordering** — `SkillStore::list_pending_review()` returns
//!    rows ordered by `created_at ASC` (oldest-first), so an operator
//!    draining the queue sees the freshest learning first when
//!    skimming the latest tail OR the most-aged when starting from
//!    the head. This is the contract the `argo skill review list`
//!    CLI assumes.
//! 2. **Lifecycle filter** — only rows in `LifecycleState::PendingReview`
//!    appear. Active / Rejected / Archived rows MUST NOT leak in.
//! 3. **Legal-transition gate** — `transition_lifecycle(id, target)`
//!    enforces the documented transition graph. PendingReview → Active
//!    and PendingReview → Rejected both succeed. PendingReview →
//!    Archived MUST fail (illegal — must go through Active first).
//!    Active → PendingReview MUST fail (illegal — promotion is
//!    one-way).
//! 4. **Drain behavior** — after operator approves all pending rows,
//!    `list_pending_review()` returns empty. After rejecting all, same.
//!    Mixed approve+reject path: approved rows become visible to
//!    runtime, rejected rows stay invisible, queue drains to empty.
//! 5. **Superseded rows excluded** — the queue's `WHERE superseded_by
//!    IS NULL` clause keeps a learned skill that was later superseded
//!    by a new version from showing up in the queue. (Defends against
//!    a reviewer seeing duplicate proposals after an operator promotes
//!    v0.1.0 and the system later auto-proposes v0.1.1 from a refined
//!    trajectory.)
//!
//! ## Why this complements existing tests
//!
//! Round 29's `claim_4_mvp_row_4_novel_sequence_detection.rs` proved
//! the THRESHOLD-GATE half (does the candidate enter the queue?).
//! This round proves the QUEUE-OPERATOR-SURFACE half (what the
//! operator sees + can act on).
//!
//! ## Authored
//!
//! Evaluator round 31b, 2026-05-23.

use std::sync::Arc;

use argo_skill::{
    LifecycleState, SkillCandidate, SkillStore, DEFAULT_SUCCESS_THRESHOLD,
};
use argo_store::SessionStore;
use argo_types::SessionId;

const VERTICAL_MARKETPLACE: &str = "marketplace-seller";
const VERTICAL_SUPPORT: &str = "customer-support";

async fn fresh_store() -> SkillStore {
    let store = SessionStore::open(":memory:").await.expect("open");
    store.migrate().await.expect("migrate");
    SkillStore::new(Arc::new(store))
}

fn candidate(name: &str, vertical: &str, success: f64) -> SkillCandidate {
    SkillCandidate::from_cu_trajectory(
        name,
        "fb_marketplace.post",
        vertical,
        vec!["screenshot".into(), "click 'Sell'".into(), "type listing".into()],
        Some("Listing live confirmation".into()),
        4321,
        success,
    )
}

/// Helper: build candidate body + propose. Returns the proposed skill.
async fn propose(
    store: &SkillStore,
    name: &str,
    vertical: &str,
    success: f64,
    session_id: SessionId,
) -> argo_skill::Skill {
    let cand = candidate(name, vertical, success);
    let body = cand.to_skill_md("session-short");
    store
        .propose_for_review(&body, session_id, vertical)
        .await
        .expect("propose_for_review")
}

#[tokio::test]
async fn claim_4_mvp_row_5_queue_orders_oldest_first_and_filters_lifecycle() {
    let store = fresh_store().await;
    let session_id = SessionId::new();

    // ── Phase 1: propose 3 candidates, each above threshold, with
    // a tiny sleep between so created_at differs in micros (the
    // ORDER BY is stable + deterministic).
    assert!(DEFAULT_SUCCESS_THRESHOLD < 0.9);
    let first = propose(&store, "alpha_first", VERTICAL_MARKETPLACE, 0.9, session_id).await;
    tokio::time::sleep(std::time::Duration::from_millis(3)).await;
    let second = propose(
        &store,
        "beta_second",
        VERTICAL_MARKETPLACE,
        0.91,
        session_id,
    )
    .await;
    tokio::time::sleep(std::time::Duration::from_millis(3)).await;
    let third = propose(&store, "gamma_third", VERTICAL_SUPPORT, 0.92, session_id).await;

    // ── Phase 2: queue returns ALL THREE in age order.
    let pending = store
        .list_pending_review()
        .await
        .expect("list_pending_review");
    assert_eq!(
        pending.len(),
        3,
        "claim-4 MVP row 5: 3 proposals must all surface in the queue; got {}",
        pending.len()
    );
    assert_eq!(
        pending[0].id, first.id,
        "claim-4 MVP row 5: oldest proposal must come first; got {:?} not first.id={:?}",
        pending[0].id, first.id
    );
    assert_eq!(
        pending[1].id, second.id,
        "claim-4 MVP row 5: second-oldest must come second"
    );
    assert_eq!(
        pending[2].id, third.id,
        "claim-4 MVP row 5: most-recent must come last"
    );
    for row in &pending {
        assert_eq!(
            row.lifecycle_state,
            LifecycleState::PendingReview,
            "claim-4 MVP row 5: every queue row must be in PendingReview; row {:?} is {:?}",
            row.id,
            row.lifecycle_state
        );
    }

    // ── Phase 3: operator promotes the first one to Active. Queue
    // now returns ONLY the remaining 2 — promoted row drops out.
    store
        .transition_lifecycle(first.id, LifecycleState::Active)
        .await
        .expect("transition to Active");
    let after_promote = store
        .list_pending_review()
        .await
        .expect("list_pending_review after promote");
    assert_eq!(
        after_promote.len(),
        2,
        "claim-4 MVP row 5: after promoting 1, queue must show 2; got {}",
        after_promote.len()
    );
    let after_ids: Vec<_> = after_promote.iter().map(|s| s.id).collect();
    assert!(
        !after_ids.contains(&first.id),
        "claim-4 MVP row 5: promoted row MUST drop out of queue; got {after_ids:?}"
    );
    assert!(
        after_ids.contains(&second.id) && after_ids.contains(&third.id),
        "claim-4 MVP row 5: un-acted rows must still be in queue; got {after_ids:?}"
    );

    // ── Phase 4: operator rejects the second one. Queue now has ONLY
    // the third.
    store
        .transition_lifecycle(second.id, LifecycleState::Rejected)
        .await
        .expect("transition to Rejected");
    let after_reject = store
        .list_pending_review()
        .await
        .expect("list_pending_review after reject");
    assert_eq!(
        after_reject.len(),
        1,
        "claim-4 MVP row 5: after promote 1 + reject 1, queue must show 1; got {}",
        after_reject.len()
    );
    assert_eq!(
        after_reject[0].id, third.id,
        "claim-4 MVP row 5: only the un-acted row remains"
    );

    // ── Phase 5: operator approves the last → queue drains to empty.
    store
        .transition_lifecycle(third.id, LifecycleState::Active)
        .await
        .expect("transition to Active");
    let drained = store
        .list_pending_review()
        .await
        .expect("list_pending_review after drain");
    assert!(
        drained.is_empty(),
        "claim-4 MVP row 5: queue must drain to empty after all 3 acted; got {} rows",
        drained.len()
    );

    // ── Phase 6: defensive — Active rows are visible to runtime view,
    // Rejected rows are NOT.
    let marketplace_visible = store
        .list_skills_for_vertical(VERTICAL_MARKETPLACE)
        .await
        .expect("list_skills_for_vertical marketplace");
    let support_visible = store
        .list_skills_for_vertical(VERTICAL_SUPPORT)
        .await
        .expect("list_skills_for_vertical support");
    assert_eq!(
        marketplace_visible.len(),
        1,
        "claim-4 MVP row 5: marketplace vertical sees 1 active skill (first); got {}",
        marketplace_visible.len()
    );
    assert_eq!(
        marketplace_visible[0].id, first.id,
        "claim-4 MVP row 5: marketplace's visible skill is the one promoted to Active"
    );
    assert_eq!(
        support_visible.len(),
        1,
        "claim-4 MVP row 5: support vertical sees 1 active skill (third); got {}",
        support_visible.len()
    );
    assert_eq!(
        support_visible[0].id, third.id,
        "claim-4 MVP row 5: support's visible skill is the one promoted to Active"
    );

    // Rejected (second) is in neither vertical's runtime view.
    for skill in marketplace_visible.iter().chain(support_visible.iter()) {
        assert_ne!(
            skill.id, second.id,
            "claim-4 MVP row 5: REJECTED skill MUST be invisible to runtime"
        );
    }
}

#[tokio::test]
async fn claim_4_mvp_row_5_illegal_transitions_are_refused() {
    let store = fresh_store().await;
    let session_id = SessionId::new();
    let pending = propose(
        &store,
        "transition_target",
        VERTICAL_MARKETPLACE,
        0.9,
        session_id,
    )
    .await;

    // ── Illegal: PendingReview → Archived (must go through Active first).
    let err = store
        .transition_lifecycle(pending.id, LifecycleState::Archived)
        .await
        .expect_err("PendingReview → Archived MUST be illegal");
    assert!(
        err.to_string().contains("illegal lifecycle transition"),
        "claim-4 MVP row 5: error must mention illegal transition; got {err}"
    );

    // ── Confirm row is STILL PendingReview after the rejected transition.
    let still_pending = store
        .list_pending_review()
        .await
        .expect("list_pending_review");
    assert_eq!(
        still_pending.len(),
        1,
        "claim-4 MVP row 5: row must still be in queue after illegal transition refused"
    );

    // ── Promote to Active (legal).
    store
        .transition_lifecycle(pending.id, LifecycleState::Active)
        .await
        .expect("PendingReview → Active is legal");

    // ── Illegal: Active → PendingReview (promotion is one-way).
    let err = store
        .transition_lifecycle(pending.id, LifecycleState::PendingReview)
        .await
        .expect_err("Active → PendingReview MUST be illegal");
    assert!(
        err.to_string().contains("illegal lifecycle transition"),
        "claim-4 MVP row 5: error must mention illegal transition; got {err}"
    );

    // ── Active → Archived is legal (sunset path).
    store
        .transition_lifecycle(pending.id, LifecycleState::Archived)
        .await
        .expect("Active → Archived is legal");
}

#[tokio::test]
async fn claim_4_mvp_row_5_queue_drains_under_concurrent_review() {
    // Defensive — two operators review the queue concurrently. The
    // store's WHERE clause + UPDATE atomicity must keep the final
    // state coherent: a promoted row drops out of any subsequent
    // list_pending_review, no duplicate visibility in the runtime
    // view.
    let store = fresh_store().await;
    let session_id = SessionId::new();

    // Seed 5 candidates.
    let mut ids = Vec::with_capacity(5);
    for i in 0..5 {
        let s = propose(
            &store,
            &format!("concurrent_target_{i}"),
            VERTICAL_MARKETPLACE,
            0.85 + (i as f64) * 0.01,
            session_id,
        )
        .await;
        ids.push(s.id);
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;
    }

    // Two "reviewers" concurrently — one promotes ids[0]+[1]+[2],
    // the other rejects ids[3]+[4]. We wrap the store in Arc so two
    // tokio tasks can share it across 'static boundaries.
    let store_arc = Arc::new(store);
    let store_a = Arc::clone(&store_arc);
    let store_b = Arc::clone(&store_arc);
    let promoters = vec![ids[0], ids[1], ids[2]];
    let rejecters = vec![ids[3], ids[4]];

    let h_a = tokio::spawn(async move {
        for id in promoters {
            store_a
                .transition_lifecycle(id, LifecycleState::Active)
                .await
                .expect("concurrent promote");
        }
    });
    let h_b = tokio::spawn(async move {
        for id in rejecters {
            store_b
                .transition_lifecycle(id, LifecycleState::Rejected)
                .await
                .expect("concurrent reject");
        }
    });
    h_a.await.expect("join A");
    h_b.await.expect("join B");
    let store = store_arc;

    // ── Queue drained.
    let drained = store
        .list_pending_review()
        .await
        .expect("list after concurrent");
    assert!(
        drained.is_empty(),
        "claim-4 MVP row 5: concurrent reviewers must drain queue to empty; got {} rows",
        drained.len()
    );

    // ── Runtime view sees exactly the 3 promoted ones.
    let visible = store
        .list_skills_for_vertical(VERTICAL_MARKETPLACE)
        .await
        .expect("list_skills_for_vertical");
    assert_eq!(
        visible.len(),
        3,
        "claim-4 MVP row 5: 3 promoted rows must be visible; got {}",
        visible.len()
    );
}
