//! Gate-level adversarial proofs for the LLM-rubric composition axis
//! (Genesis P7, DEC-025 #7).
//!
//! These prove the rubric folds into the FULL [`crate::GateVerdict`] precedence
//! as an ADDITIVE axis on top of the deterministic floor: it can only TIGHTEN
//! the verdict (block a thin teammate / hold a degraded one), NEVER loosen it,
//! and the deterministic floor stays the fail-closed baseline. The pure
//! per-dimension scoring + degrade-fail-safe parsing proofs live in
//! `rubric_tests.rs`; these prove the COMPOSITION.

use crate::comprehension::ComprehensionOutcome;
use crate::hire_gate::{EvalFloorOutcome, GateState, GateVerdict, SafetySignal};
use crate::hire_gate_contract::ContractGateInput;
use crate::pipeline::plan;
use crate::role_contract::RoleContract;
use crate::rubric::{RubricOutcome, RubricVerdict, RUBRIC_PASS_FLOOR};
use crate::sop::SopComprehensionOutcome;

/// A clean comprehension pass (all 5 questions answered).
fn comp_pass() -> ComprehensionOutcome {
    ComprehensionOutcome::from_counts(5, 5)
}

/// A confirmed role contract — uses a real plan's manifest, confirmed by
/// echoing its own truthful digest.
fn confirmed_contract() -> ContractGateInput {
    let p = plan("be my marketing intern, run weekly campaigns to cold leads", "maya")
        .expect("plan builds");
    let digest = RoleContract::derive_from_manifest(&p.manifest).digest();
    ContractGateInput::from_plan(&p, Some(&digest))
}

/// An unconfirmed contract (no operator digest supplied).
fn unconfirmed_contract() -> ContractGateInput {
    let p = plan("be my marketing intern, run weekly campaigns to cold leads", "maya")
        .expect("plan builds");
    ContractGateInput::from_plan(&p, None)
}

/// A graded rubric outcome with all four dimensions at `score` (clears both the
/// per-dimension and mean floors when `score >= 0.6`).
fn rubric_uniform(score: f64) -> RubricOutcome {
    RubricOutcome::from_verdict(RubricVerdict::Graded {
        scope_clarity: score,
        action_coherence: score,
        governance_posture: score,
        role_fit: score,
        rationale: String::new(),
    })
}

#[test]
fn rubric_not_applicable_leaves_a_passing_gate_unchanged() {
    // P7 default posture (flag off): a not-applicable rubric is a vacuous pass —
    // a fully-green deterministic gate still passes, the rubric did not affect it.
    let base = GateVerdict::compose(
        EvalFloorOutcome::from_counts(8, 10),
        comp_pass(),
        SopComprehensionOutcome::not_applicable(),
        SafetySignal::Allow,
        confirmed_contract(),
    );
    let v = base.with_rubric(RubricOutcome::not_applicable());
    assert_eq!(v.state, GateState::Pass);
    assert!(v.state.allows_hire());
    assert!(!v.rubric_applicable);
    assert_eq!(v.rubric_verdict, "not_applicable");
    assert!(
        !v.reasons.iter().any(|r| r.contains("rubric")),
        "a not-applicable rubric must add no reason; got {:?}",
        v.reasons
    );
}

#[test]
fn rubric_pass_on_an_otherwise_green_gate_stays_pass() {
    // A capable teammate (every other axis green) that ALSO clears the LLM
    // rubric is hireable, and the rubric verdict + score are surfaced honestly.
    let base = GateVerdict::compose(
        EvalFloorOutcome::from_counts(10, 10),
        comp_pass(),
        SopComprehensionOutcome::not_applicable(),
        SafetySignal::Allow,
        confirmed_contract(),
    );
    let v = base.with_rubric(rubric_uniform(0.9));
    assert_eq!(v.state, GateState::Pass, "reasons={:?}", v.reasons);
    assert!(v.state.allows_hire());
    assert!(v.rubric_applicable);
    assert!(!v.rubric_degraded);
    assert_eq!(v.rubric_verdict, "pass");
    assert!(v.rubric_score >= RUBRIC_PASS_FLOOR);
}

#[test]
fn rubric_failure_hard_blocks_a_teammate_the_floor_let_through() {
    // THE P7 POINT (cannot be gamed): a teammate that PASSES the deterministic
    // eval floor + comprehension + safety + a confirmed contract — i.e. would be
    // hireable under P0–P6 — is HARD BLOCKED when the richer LLM rubric judges
    // it thin/incoherent. This is the substring-floor gap the rubric closes.
    let base = GateVerdict::compose(
        EvalFloorOutcome::from_counts(10, 10),
        comp_pass(),
        SopComprehensionOutcome::not_applicable(),
        SafetySignal::Allow,
        confirmed_contract(),
    );
    assert_eq!(base.state, GateState::Pass, "the deterministic gate alone passes this teammate");

    // The LLM rubric judges it incoherent (low coherence + governance).
    let thin = RubricOutcome::from_verdict(RubricVerdict::Graded {
        scope_clarity: 0.8,
        action_coherence: 0.2,
        governance_posture: 0.1,
        role_fit: 0.2,
        rationale: "names the domain but no coherent capability".into(),
    });
    let v = base.with_rubric(thin);
    assert_eq!(
        v.state,
        GateState::Blocked,
        "the rubric must hard-block a thin teammate the floor missed; reasons={:?}",
        v.reasons
    );
    assert!(!v.state.allows_hire());
    assert_eq!(v.rubric_verdict, "block");
    assert!(
        v.reasons.iter().any(|r| r.contains("LLM capability rubric failed")),
        "expected a rubric-failure reason; got {:?}",
        v.reasons
    );
}

#[test]
fn rubric_degraded_holds_for_review_never_silent_pass() {
    // DEGRADE FAIL-SAFE: every deterministic axis is green + the contract is
    // confirmed (so without the rubric this is a clean Pass), but the rubric was
    // ON and the LLM grader could not run → needs_review (held in dry-run),
    // NEVER a silent pass.
    let base = GateVerdict::compose(
        EvalFloorOutcome::from_counts(10, 10),
        comp_pass(),
        SopComprehensionOutcome::not_applicable(),
        SafetySignal::Allow,
        confirmed_contract(),
    );
    assert_eq!(base.state, GateState::Pass);

    let v = base.with_rubric(RubricOutcome::unavailable("provider quota exhausted"));
    assert_eq!(
        v.state,
        GateState::NeedsReview,
        "a degraded rubric must hold for review, never pass; reasons={:?}",
        v.reasons
    );
    assert!(!v.state.allows_hire(), "needs_review must NOT auto-activate");
    assert!(v.rubric_applicable);
    assert!(v.rubric_degraded);
    assert_eq!(v.rubric_verdict, "unavailable");
    assert!(
        v.reasons.iter().any(|r| r.contains("rubric unavailable")
            && r.contains("deterministic eval floor still applied")),
        "the degraded reason must note the deterministic floor still applied; got {:?}",
        v.reasons
    );
}

#[test]
fn deterministic_floor_block_outranks_a_passing_rubric() {
    // THE BASELINE STAYS (no theater): a teammate that FAILS the deterministic
    // eval floor stays HARD BLOCKED even if the LLM rubric would happily pass it.
    // The rubric is an ADDITIONAL lens, never a bypass of the fail-closed floor.
    let base = GateVerdict::compose(
        EvalFloorOutcome::from_counts(1, 10), // floor fails
        comp_pass(),
        SopComprehensionOutcome::not_applicable(),
        SafetySignal::Allow,
        confirmed_contract(),
    );
    assert_eq!(base.state, GateState::Blocked);

    let v = base.with_rubric(rubric_uniform(1.0)); // perfect rubric
    assert_eq!(
        v.state,
        GateState::Blocked,
        "a passing rubric must NEVER rescue a teammate the deterministic floor blocked"
    );
    assert!(!v.state.allows_hire());
    // The rubric is still recorded honestly as a pass on its own axis.
    assert_eq!(v.rubric_verdict, "pass");
}

#[test]
fn safety_block_outranks_a_passing_rubric() {
    // A safety-judge Block also outranks a passing rubric — the rubric can't
    // override the structural/semantic safety floor.
    let base = GateVerdict::compose(
        EvalFloorOutcome::from_counts(10, 10),
        comp_pass(),
        SopComprehensionOutcome::not_applicable(),
        SafetySignal::Block { reason: "illegal instructions".into() },
        confirmed_contract(),
    );
    assert_eq!(base.state, GateState::Blocked);
    let v = base.with_rubric(rubric_uniform(1.0));
    assert_eq!(v.state, GateState::Blocked, "safety block must survive a passing rubric");
}

#[test]
fn rubric_failure_outranks_a_merely_degraded_safety_signal() {
    // A hard rubric block (the teammate is genuinely thin) outranks a merely
    // degraded safety signal — block beats needs_review, consistent with every
    // other axis.
    let base = GateVerdict::compose(
        EvalFloorOutcome::from_counts(10, 10),
        comp_pass(),
        SopComprehensionOutcome::not_applicable(),
        SafetySignal::Unavailable { reason: "no provider".into() },
        confirmed_contract(),
    );
    assert_eq!(base.state, GateState::NeedsReview, "degraded safety alone is needs_review");

    let thin = RubricOutcome::from_verdict(RubricVerdict::Graded {
        scope_clarity: 0.2,
        action_coherence: 0.2,
        governance_posture: 0.2,
        role_fit: 0.2,
        rationale: "thin".into(),
    });
    let v = base.with_rubric(thin);
    assert_eq!(
        v.state,
        GateState::Blocked,
        "a hard rubric block must outrank a degraded safety signal"
    );
}

#[test]
fn rubric_pass_does_not_clear_an_unconfirmed_contract() {
    // A passing rubric does NOT substitute for operator confirmation — an
    // unconfirmed contract still holds the teammate for review (human-review-
    // before-activation stays ON regardless of how capable the rubric judged it).
    let base = GateVerdict::compose(
        EvalFloorOutcome::from_counts(10, 10),
        comp_pass(),
        SopComprehensionOutcome::not_applicable(),
        SafetySignal::Allow,
        unconfirmed_contract(),
    );
    assert_eq!(base.state, GateState::NeedsReview);
    let v = base.with_rubric(rubric_uniform(0.95));
    assert_eq!(
        v.state,
        GateState::NeedsReview,
        "a passing rubric must not clear an unconfirmed contract"
    );
    assert!(!v.state.allows_hire());
    assert_eq!(v.rubric_verdict, "pass");
    assert!(v.reasons.iter().any(|r| r.contains("awaiting operator confirmation")));
}
