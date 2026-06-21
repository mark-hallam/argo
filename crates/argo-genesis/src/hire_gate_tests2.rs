//! Broad / LLM-path eval-floor regression tests for the hire gate (the
//! flagship `POST /api/v1/genesis` SSE path). Split out of
//! `hire_gate_tests.rs` to keep that file under the per-file LOC budget.
//!
//! Proves a legitimate from-scratch (broad/LLM-path) teammate clears its
//! OWN deterministic eval floor while a degenerate/thin one still fails —
//! the discrimination guard that keeps the floor honest, not theater.

use super::*;
use crate::materialize::materialize_spec;
use crate::pipeline::GenesisPlan;
use crate::spec::CapabilitySpec;
use crate::{Archetype, BroadRoleDraft};

/// Deterministic re-implementation of the canonical eval floor's scoring, so
/// the pure crate can prove a teammate clears its OWN starter suite without a
/// provider. Mirrors `routes_genesis_gate::run_eval_floor` exactly: each seed
/// "passes" iff EVERY rubric keyword is a substring of the lowercased
/// behaviour contract (the `EvalMetric::SubstringContains` AND-semantics), and
/// the floor clears iff the pass-rate meets `EVAL_PASS_RATE_FLOOR`.
fn score_floor(inputs: &GateEvalInputs) -> EvalFloorOutcome {
    let passed = inputs
        .seeds
        .iter()
        .filter(|s| {
            s.rubric_keywords
                .iter()
                .all(|k| inputs.contract_text.contains(k.as_str()))
        })
        .count();
    EvalFloorOutcome::from_counts(passed, inputs.seeds.len())
}

/// A legitimate from-scratch (broad/LLM-path) teammate: an off-taxonomy role
/// the archetype classifier would not confidently match, materialized through
/// the SAME `BroadRoleDraft → CapabilitySpec → materialize_spec` seam the live
/// `POST /api/v1/genesis` SSE page uses.
fn vineyard_broad_plan() -> GenesisPlan {
    let draft = BroadRoleDraft {
        charter: "a vineyard logistics coordinator who schedules harvests, \
                  drafts trucking plans, and dispatches haulage to the winery"
            .into(),
        persona: "vinny".into(),
        persona_description: "Coordinates vineyard harvest scheduling and haulage logistics."
            .into(),
        action_vocabulary: vec![
            "schedule_harvest".into(),
            "draft_truck_plan".into(),
            "send_dispatch_notice".into(),
        ],
        success_metrics: vec!["zero missed harvest windows".into()],
        onboarding_script: "Hi, I keep the harvest calendar tight.".into(),
        ..Default::default()
    };
    let spec = CapabilitySpec::from_broad_draft(draft, Archetype::CalendarCoordinator);
    materialize_spec(&spec).expect("broad-path teammate materializes")
}

#[test]
fn broad_path_seeds_tokenize_consistently_with_the_contract() {
    // ROOT-CAUSE REGRESSION: the broad path's eval-seed keywords MUST tokenize
    // the same way the behaviour-contract text does. The old boilerplate
    // description "Genesis-synthesized capability: {name}" produced the merged
    // tokens "genesissynthesized" / "schedulereport" (separators DELETED),
    // which could never be substrings of the contract (which keeps the `-`/`_`
    // boundaries) → 0/10 forever. Assert every rubric keyword the broad path
    // emits is a clean contiguous-alphanumeric run AND actually appears in the
    // teammate's own contract.
    let p = vineyard_broad_plan();
    let inputs = build_gate_eval_inputs(&p).expect("broad-path inputs build");
    for seed in &inputs.seeds {
        for kw in &seed.rubric_keywords {
            assert!(
                kw.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()),
                "keyword {kw:?} must be a clean contiguous-alphanumeric token"
            );
            assert!(
                inputs.contract_text.contains(kw.as_str()),
                "keyword {kw:?} from seed {} is NOT a substring of the teammate's own \
                 behaviour contract — the broad-path 0/10 bug has regressed",
                seed.id
            );
        }
    }
}

#[test]
fn broad_path_teammate_clears_its_own_deterministic_eval_floor() {
    // THE BUG: a legitimate SSE-generated (broad/LLM-path) teammate scored
    // 0/10 and could never become hireable. It must now clear the >=50% floor
    // through the real gate-input builder + the canonical SubstringContains
    // scoring (replicated by `score_floor`).
    let p = vineyard_broad_plan();
    let inputs = build_gate_eval_inputs(&p).expect("broad-path inputs build");
    assert_eq!(inputs.seeds.len(), 10, "ten starter seeds");
    let eval = score_floor(&inputs);
    assert!(
        eval.passed_floor,
        "a legitimate broad-path teammate must clear its own eval floor; \
         got {}/{} ({:.0}%)",
        eval.passed,
        eval.total,
        eval.pass_rate * 100.0
    );
}

#[test]
fn degenerate_teammate_still_fails_the_eval_floor_discrimination_preserved() {
    // DISCRIMINATION (must NOT be theater): the fix makes a *real* teammate
    // pass, but a degenerate/thin one must STILL fail. We take the legitimate
    // teammate's OWN seeds and score them against a thin stub contract (the
    // canonical "empty worker" the floor exists to catch). The same seeds that
    // pass against the rich vineyard contract score 0 here → the floor is
    // genuinely discriminating, not unconditionally green.
    let p = vineyard_broad_plan();
    let real = build_gate_eval_inputs(&p).expect("inputs");
    let stub = GateEvalInputs {
        seeds: real.seeds.clone(),
        // A thin teammate whose contract names none of its supposed domain
        // vocabulary (the empty/stub worker).
        contract_text: "i am a helpful assistant.".to_lowercase(),
    };
    let eval = score_floor(&stub);
    assert!(
        !eval.passed_floor,
        "a thin/degenerate teammate must FAIL the floor; got {}/{} ({:.0}%)",
        eval.passed,
        eval.total,
        eval.pass_rate * 100.0
    );
    assert_eq!(eval.passed, 0, "no seed should match an empty-stub contract");
}

#[test]
fn synthesized_descriptions_are_role_relevant_not_boilerplate() {
    // SEED QUALITY (DEC-025 #7): the broad-path action descriptions must carry
    // the teammate's REAL action verbs + role vocabulary, not the old
    // "Genesis-synthesized capability" boilerplate (whose only matchable token
    // was the word "capability", present in every contract → a meaningless
    // eval). Assert the contract carries the de-snake-cased verbs + domain.
    let p = vineyard_broad_plan();
    let contract = behaviour_contract_text(&p);
    assert!(
        !contract.contains("genesis-synthesized"),
        "the boilerplate description must be gone"
    );
    // Real action verbs (de-snake-cased) appear as plain words.
    assert!(contract.contains("schedule harvest"));
    assert!(contract.contains("draft truck plan"));
    // The role descriptor anchors the descriptions in the teammate's domain.
    assert!(contract.contains("vineyard"));
}
