//! Tests for the pure hire-gate primitives.

use super::*;
use crate::pipeline::plan;

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
fn compose_passes_when_evals_pass_and_judge_allows() {
    let v = GateVerdict::compose(EvalFloorOutcome::from_counts(8, 10), SafetySignal::Allow);
    assert_eq!(v.state, GateState::Pass);
    assert!(v.state.allows_hire());
    assert!(v.reasons.is_empty(), "pass has no reasons: {:?}", v.reasons);
    assert_eq!(v.safety_verdict, "allow");
    assert_eq!(v.eval_passed, 8);
    assert_eq!(v.eval_total, 10);
}

#[test]
fn compose_blocks_when_evals_fail() {
    let v = GateVerdict::compose(EvalFloorOutcome::from_counts(2, 10), SafetySignal::Allow);
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
        SafetySignal::Block {
            reason: "impersonates a real person".into(),
        },
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
        SafetySignal::Block {
            reason: "illegal instructions".into(),
        },
    );
    assert_eq!(v.state, GateState::Blocked);
}

#[test]
fn compose_needs_review_when_judge_unavailable() {
    // Degraded mode: evals pass, judge couldn't run → needs_review, NOT pass.
    let v = GateVerdict::compose(
        EvalFloorOutcome::from_counts(10, 10),
        SafetySignal::Unavailable {
            reason: "provider quota exhausted".into(),
        },
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
        SafetySignal::Unavailable {
            reason: "no provider".into(),
        },
    );
    assert_eq!(v.state, GateState::Blocked);
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
