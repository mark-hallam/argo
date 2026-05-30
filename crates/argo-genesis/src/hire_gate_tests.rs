//! Tests for the pure hire-gate primitives.

use super::*;
use crate::comprehension::ComprehensionOutcome;
use crate::pipeline::plan;
use crate::role_contract::RoleContract;

/// A clean comprehension pass (all 5 questions answered) — the common case
/// for compose tests that are exercising the eval/safety axes.
fn comp_pass() -> ComprehensionOutcome {
    ComprehensionOutcome::from_counts(5, 5)
}

/// A confirmed role contract — the common case for compose tests that are
/// exercising the eval/comprehension/safety axes (not the confirmation axis).
/// Uses a real plan's manifest so the contract is non-trivial, and confirms
/// it by echoing its own truthful digest.
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

#[test]
fn eval_floor_passes_at_or_above_half() {
    let o = EvalFloorOutcome::from_counts(5, 10);
    assert!((o.pass_rate - 0.5).abs() < 1e-9);
    assert!(o.passed_floor, "5/10 == 0.5 meets the >=0.5 floor");
    let o = EvalFloorOutcome::from_counts(7, 10);
    assert!(o.passed_floor);
}

#[test]
fn eval_floor_fails_below_half() {
    let o = EvalFloorOutcome::from_counts(4, 10);
    assert!(!o.passed_floor, "4/10 < 0.5 fails the floor");
}

#[test]
fn eval_floor_fails_closed_on_zero_seeds() {
    let o = EvalFloorOutcome::from_counts(0, 0);
    assert!(!o.passed_floor, "no starter suite is itself a floor failure");
    assert_eq!(o.pass_rate, 0.0);
}

#[test]
fn compose_passes_when_evals_pass_and_judge_allows_and_contract_confirmed() {
    let v = GateVerdict::compose(
        EvalFloorOutcome::from_counts(8, 10),
        comp_pass(),
        SafetySignal::Allow,
        confirmed_contract(),
    );
    assert_eq!(v.state, GateState::Pass);
    assert!(v.state.allows_hire());
    assert!(v.reasons.is_empty(), "pass has no reasons: {:?}", v.reasons);
    assert_eq!(v.safety_verdict, "allow");
    assert_eq!(v.eval_passed, 8);
    assert_eq!(v.eval_total, 10);
    assert_eq!(v.comprehension_passed, 5);
    assert_eq!(v.comprehension_total, 5);
    assert_eq!(v.contract_confirmation, "confirmed");
    assert!(v.role_contract.is_some());
    assert_eq!(v.contract_digest.len(), 64);
}

#[test]
fn compose_blocks_when_evals_fail() {
    let v = GateVerdict::compose(
        EvalFloorOutcome::from_counts(2, 10),
        comp_pass(),
        SafetySignal::Allow,
        confirmed_contract(),
    );
    assert_eq!(v.state, GateState::Blocked);
    assert!(!v.state.allows_hire(), "a failing teammate must NOT be hireable");
    assert!(
        v.reasons.iter().any(|r| r.contains("eval floor failed")),
        "expected eval-floor reason; got: {:?}",
        v.reasons
    );
}

#[test]
fn compose_blocks_when_safety_judge_blocks() {
    let v = GateVerdict::compose(
        EvalFloorOutcome::from_counts(10, 10),
        comp_pass(),
        SafetySignal::Block {
            reason: "impersonates a real person".into(),
        },
        confirmed_contract(),
    );
    assert_eq!(v.state, GateState::Blocked);
    assert!(!v.state.allows_hire());
    assert_eq!(v.safety_verdict, "block");
    assert!(v.reasons.iter().any(|r| r.contains("impersonates")));
}

#[test]
fn compose_block_wins_over_pass_evals() {
    // Both signals: evals pass but judge blocks → Blocked, never Pass.
    let v = GateVerdict::compose(
        EvalFloorOutcome::from_counts(10, 10),
        comp_pass(),
        SafetySignal::Block {
            reason: "illegal instructions".into(),
        },
        confirmed_contract(),
    );
    assert_eq!(v.state, GateState::Blocked);
}

#[test]
fn compose_needs_review_when_judge_unavailable() {
    // Degraded mode: evals pass, judge couldn't run → needs_review, NOT pass.
    let v = GateVerdict::compose(
        EvalFloorOutcome::from_counts(10, 10),
        comp_pass(),
        SafetySignal::Unavailable {
            reason: "provider quota exhausted".into(),
        },
        confirmed_contract(),
    );
    assert_eq!(v.state, GateState::NeedsReview);
    assert!(
        !v.state.allows_hire(),
        "needs_review must NOT auto-activate a teammate"
    );
    assert_eq!(v.safety_verdict, "unavailable");
    assert!(v.reasons.iter().any(|r| r.contains("degraded")));
}

#[test]
fn compose_failed_evals_block_even_if_judge_unavailable() {
    // A hard fail (eval) outranks a degraded signal.
    let v = GateVerdict::compose(
        EvalFloorOutcome::from_counts(1, 10),
        comp_pass(),
        SafetySignal::Unavailable {
            reason: "no provider".into(),
        },
        confirmed_contract(),
    );
    assert_eq!(v.state, GateState::Blocked);
}

#[test]
fn compose_blocks_when_comprehension_fails() {
    // Evals pass + judge allows, but the teammate's contract failed a
    // governance comprehension question → hard block (an invariant miss).
    let v = GateVerdict::compose(
        EvalFloorOutcome::from_counts(10, 10),
        ComprehensionOutcome::from_counts(4, 5),
        SafetySignal::Allow,
        confirmed_contract(),
    );
    assert_eq!(v.state, GateState::Blocked);
    assert!(!v.state.allows_hire(), "a comprehension miss must NOT be hireable");
    assert!(
        v.reasons.iter().any(|r| r.contains("comprehension")),
        "expected a comprehension reason; got: {:?}",
        v.reasons
    );
    assert_eq!(v.comprehension_passed, 4);
    assert_eq!(v.comprehension_total, 5);
}

#[test]
fn compose_comprehension_block_outranks_degraded_safety() {
    // A hard comprehension fail outranks a merely-degraded safety signal.
    let v = GateVerdict::compose(
        EvalFloorOutcome::from_counts(10, 10),
        ComprehensionOutcome::from_counts(0, 5),
        SafetySignal::Unavailable {
            reason: "no provider".into(),
        },
        confirmed_contract(),
    );
    assert_eq!(v.state, GateState::Blocked);
}

// --- Operator-confirmation axis (DEC-025 #1 + #4) ---------------------------

#[test]
fn compose_holds_for_review_when_contract_unconfirmed() {
    // Every other axis is green, but the operator has NOT confirmed the role
    // contract → needs_review (human-review-before-activation is ON), NEVER
    // Pass. This is the load-bearing P4 guarantee: no activation without a
    // confirmed contract.
    let v = GateVerdict::compose(
        EvalFloorOutcome::from_counts(10, 10),
        comp_pass(),
        SafetySignal::Allow,
        unconfirmed_contract(),
    );
    assert_eq!(v.state, GateState::NeedsReview);
    assert!(
        !v.state.allows_hire(),
        "an unconfirmed contract must NOT auto-activate the teammate"
    );
    assert_eq!(v.contract_confirmation, "unconfirmed");
    assert!(
        v.reasons.iter().any(|r| r.contains("awaiting operator confirmation")),
        "expected an awaiting-confirmation reason; got: {:?}",
        v.reasons
    );
}

#[test]
fn compose_hard_blocks_on_a_mismatched_contract_confirmation() {
    // The operator echoed a digest that does NOT match the teammate's truthful
    // contract (stale or forged) → HARD block, not merely needs_review. A
    // confirmation must never bind to a teammate the operator never saw.
    let p = plan("be my marketing intern, run weekly campaigns", "maya").expect("plan");
    let bogus = ContractGateInput::from_plan(&p, Some("deadbeef".repeat(8).as_str()));
    let v = GateVerdict::compose(
        EvalFloorOutcome::from_counts(10, 10),
        comp_pass(),
        SafetySignal::Allow,
        bogus,
    );
    assert_eq!(v.state, GateState::Blocked);
    assert!(!v.state.allows_hire());
    assert_eq!(v.contract_confirmation, "mismatch");
    assert!(
        v.reasons.iter().any(|r| r.contains("confirmation mismatch")),
        "expected a mismatch reason; got: {:?}",
        v.reasons
    );
}

#[test]
fn compose_mismatch_block_outranks_degraded_safety() {
    // A forged/stale confirmation is a hard fail that outranks a merely
    // degraded safety signal.
    let p = plan("be my marketing intern, run weekly campaigns", "maya").expect("plan");
    let bogus = ContractGateInput::from_plan(&p, Some("00ff".repeat(16).as_str()));
    let v = GateVerdict::compose(
        EvalFloorOutcome::from_counts(10, 10),
        comp_pass(),
        SafetySignal::Unavailable {
            reason: "no provider".into(),
        },
        bogus,
    );
    assert_eq!(v.state, GateState::Blocked);
}

#[test]
fn compose_cannot_pass_without_a_contract_to_confirm() {
    // No contract at all (manifest unavailable) → cannot confirm scope, holds
    // for review even with a (vacuous) confirmation. Belt-and-braces: there
    // is nothing to surface for the operator, so the teammate can't activate.
    let input = ContractGateInput::resolve(None, None);
    let v = GateVerdict::compose(
        EvalFloorOutcome::from_counts(10, 10),
        comp_pass(),
        SafetySignal::Allow,
        input,
    );
    assert_ne!(v.state, GateState::Pass, "no contract → never pass");
    assert!(!v.state.allows_hire());
}

#[test]
fn confirmation_resolves_correctly_against_the_truthful_digest() {
    let p = plan("be my marketing intern, run weekly campaigns", "maya").expect("plan");
    let truthful = RoleContract::derive_from_manifest(&p.manifest).digest();

    // No digest → unconfirmed.
    let none = ContractGateInput::from_plan(&p, None);
    assert!(matches!(none.confirmation, ContractConfirmation::Unconfirmed));

    // Correct digest → confirmed.
    let ok = ContractGateInput::from_plan(&p, Some(&truthful));
    assert!(matches!(ok.confirmation, ContractConfirmation::Confirmed));

    // Wrong digest → mismatch, carrying both digests for the audit.
    let wrong = ContractGateInput::from_plan(&p, Some("not-the-digest"));
    match wrong.confirmation {
        ContractConfirmation::Mismatch { supplied, expected } => {
            assert_eq!(supplied, "not-the-digest");
            assert_eq!(expected, truthful);
        }
        other => panic!("expected mismatch, got {other:?}"),
    }
}

#[test]
fn gate_state_tokens_are_stable() {
    assert_eq!(GateState::Pass.as_str(), "pass");
    assert_eq!(GateState::Blocked.as_str(), "blocked");
    assert_eq!(GateState::NeedsReview.as_str(), "needs_review");
}

#[test]
fn build_gate_eval_inputs_yields_ten_seeds_and_contract() {
    let p = plan("be my marketing intern, run weekly campaigns to cold leads", "maya").unwrap();
    let inputs = build_gate_eval_inputs(&p).expect("inputs build");
    assert_eq!(inputs.seeds.len(), 10, "ten starter seeds");
    assert!(!inputs.contract_text.is_empty());
    // Contract text is lowercased and carries the charter words.
    assert!(inputs.contract_text.contains("marketing"));
    // Every seed carries at least one rubric keyword.
    for seed in &inputs.seeds {
        assert!(!seed.rubric_keywords.is_empty(), "seed {} has no keywords", seed.id);
    }
}

#[test]
fn judge_input_for_plan_projects_charter_and_actions() {
    let p = plan("be my marketing intern, run weekly campaigns to cold leads", "maya").unwrap();
    let ji = judge_input_for_plan(&p);
    assert_eq!(ji.charter, p.charter);
    assert_eq!(ji.persona_description, p.persona);
    assert!(
        !ji.action_vocabulary.is_empty(),
        "the marketing plan declares actions"
    );
}

#[test]
fn behaviour_contract_text_distinguishes_rich_from_thin() {
    // A real plan's contract mentions its archetype label + actions.
    let p = plan("be my marketing intern, run weekly campaigns to cold leads", "maya").unwrap();
    let rich = behaviour_contract_text(&p);
    assert!(rich.contains("marketing"));
    assert!(rich.len() > 200, "a real contract is substantial");
}
