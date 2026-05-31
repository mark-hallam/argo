//! Tests for the teammate-eval golden-suite runner.

use super::*;
use crate::teammate::{ScenarioOutcome, TeammateScenario};

/// A scenario with sensible defaults; tweak fields per test.
fn scenario(name: &str) -> TeammateScenario {
    TeammateScenario {
        name: name.into(),
        situation: "A buyer offers $75 on an $85 listing.".into(),
        expected_substrings: vec!["counter".into()],
        should_escalate: false,
        required_skills: vec!["price_sanity_check".into()],
        forbidden_claim: Some("guaranteed lowest price".into()),
        budget_ms: 5_000,
        budget_usd_micros: 20_000,
    }
}

/// An outcome that scores 1.0 on every axis of [`scenario`].
fn good_outcome() -> ScenarioOutcome {
    ScenarioOutcome {
        output: "I'll counter at $82 with free local pickup.".into(),
        escalated: false,
        skills_used: vec!["price_sanity_check".into()],
        honest: true,
        elapsed_ms: 1_200,
        cost_usd_micros: 9_000,
    }
}

#[test]
fn passing_case_meets_all_axes() {
    let case = GoldenCase {
        scenario: scenario("ok"),
        outcome: good_outcome(),
    };
    let v = evaluate_case(&case, 0.5);
    assert!(v.passed);
    assert!(v.failed_axes.is_empty());
    assert_eq!(v.scenario_name, "ok");
    assert!((v.overall - 1.0).abs() < 1e-9);
}

#[test]
fn correctness_failure_is_named() {
    // No "counter" in the output → correctness 0.0 → fails at any
    // positive threshold, and the axis name is reported.
    let mut outcome = good_outcome();
    outcome.output = "Sounds good, deal.".into();
    let v = evaluate_case(
        &GoldenCase {
            scenario: scenario("weak"),
            outcome,
        },
        0.5,
    );
    assert!(!v.passed);
    assert_eq!(v.failed_axes, vec!["correctness".to_string()]);
}

#[test]
fn boundary_violation_is_named() {
    // Keep "counter" in the output so correctness stays 1.0 and the
    // forbidden claim is the ONLY failing axis — isolating the boundary
    // check. (A bare forbidden-claim string with no "counter" would
    // correctly fail BOTH boundary and correctness.)
    let mut outcome = good_outcome();
    outcome.output =
        "I'll counter at $82 — it's the guaranteed lowest price anywhere.".into();
    let v = evaluate_case(
        &GoldenCase {
            scenario: scenario("boundary"),
            outcome,
        },
        0.5,
    );
    assert!(!v.passed);
    assert_eq!(v.failed_axes, vec!["boundary".to_string()]);
}

#[test]
fn threshold_boundary_is_inclusive() {
    // correctness is a 2-of-2 fraction → exactly 1.0; with a fractional
    // expectation we can land an axis exactly on the threshold and it
    // must PASS (>= is inclusive). Two substrings, one hit → 0.5.
    let mut scen = scenario("half");
    scen.expected_substrings = vec!["counter".into(), "warranty".into()];
    scen.required_skills = vec![]; // don't let skill axis interfere
    scen.forbidden_claim = None;
    let v = evaluate_case(
        &GoldenCase {
            scenario: scen,
            outcome: good_outcome(), // has "counter", not "warranty" → 0.5
        },
        0.5,
    );
    // correctness == 0.5 == threshold → inclusive pass; all other axes 1.0
    assert!(v.passed, "axis exactly at threshold must pass: {v:?}");
}

#[test]
fn multiple_failed_axes_are_sorted() {
    // Wrong escalation + dishonest → escalation and honesty both 0.0.
    let mut outcome = good_outcome();
    outcome.escalated = true; // scenario.should_escalate is false → escalation 0.0
    outcome.honest = false; // honesty 0.0
    let v = evaluate_case(
        &GoldenCase {
            scenario: scenario("two-fail"),
            outcome,
        },
        0.5,
    );
    assert!(!v.passed);
    assert_eq!(
        v.failed_axes,
        vec!["escalation".to_string(), "honesty".to_string()]
    );
}

#[test]
fn run_golden_suite_aggregates_pass_fail_and_mean() {
    let pass = GoldenCase {
        scenario: scenario("a"),
        outcome: good_outcome(),
    };
    let mut bad = good_outcome();
    bad.output = "ok deal".into(); // correctness 0.0
    let fail = GoldenCase {
        scenario: scenario("b"),
        outcome: bad,
    };
    let report = run_golden_suite(&[pass, fail], 0.5);
    assert_eq!(report.total, 2);
    assert_eq!(report.passed, 1);
    assert!(!report.all_passed());
    assert!((report.pass_rate() - 0.5).abs() < 1e-9);
    // case a overall 1.0; case b overall = 5/6 (only correctness is 0).
    let expected_mean = (1.0 + 5.0 / 6.0) / 2.0;
    assert!((report.mean_overall - expected_mean).abs() < 1e-9);
    // verdicts preserve input order
    assert_eq!(report.verdicts[0].scenario_name, "a");
    assert_eq!(report.verdicts[1].scenario_name, "b");
}

#[test]
fn empty_suite_vacuously_passes() {
    let report = run_golden_suite(&[], 0.5);
    assert_eq!(report.total, 0);
    assert_eq!(report.passed, 0);
    assert!(report.all_passed());
    assert!((report.pass_rate() - 1.0).abs() < 1e-9);
    assert_eq!(report.mean_overall, 0.0);
}

#[test]
fn strict_threshold_of_one_requires_perfection() {
    // At threshold 1.0, only an all-1.0 outcome passes.
    let v = evaluate_case(
        &GoldenCase {
            scenario: scenario("strict"),
            outcome: good_outcome(),
        },
        1.0,
    );
    assert!(v.passed);
    // A single fractional axis now fails.
    let mut scen = scenario("strict-frac");
    scen.expected_substrings = vec!["counter".into(), "warranty".into()]; // → 0.5
    let v2 = evaluate_case(
        &GoldenCase {
            scenario: scen,
            outcome: good_outcome(),
        },
        1.0,
    );
    assert!(!v2.passed);
    assert_eq!(v2.failed_axes, vec!["correctness".to_string()]);
}

#[test]
fn report_serde_round_trips() {
    let report = run_golden_suite(
        &[GoldenCase {
            scenario: scenario("rt"),
            outcome: good_outcome(),
        }],
        0.5,
    );
    let json = serde_json::to_string(&report).expect("serialize report");
    let back: TeammateEvalReport = serde_json::from_str(&json).expect("deserialize report");
    assert_eq!(report, back);
}

#[test]
fn golden_case_serde_round_trips() {
    let case = GoldenCase {
        scenario: scenario("rt-case"),
        outcome: good_outcome(),
    };
    let json = serde_json::to_string(&case).expect("serialize case");
    let back: GoldenCase = serde_json::from_str(&json).expect("deserialize case");
    assert_eq!(case, back);
}
