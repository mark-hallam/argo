//! Tests for the teammate-eval scorecard + golden-case loader.

use super::*;
use crate::teammate::{ScenarioOutcome, TeammateScenario};
use crate::teammate_runner::GoldenCase;

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

/// Scores 1.0 on every axis of [`scenario`].
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

fn case(name: &str, outcome: ScenarioOutcome) -> GoldenCase {
    GoldenCase {
        scenario: scenario(name),
        outcome,
    }
}

#[test]
fn all_perfect_suite_scores_every_axis_one() {
    let cases = vec![case("a", good_outcome()), case("b", good_outcome())];
    let card = build_scorecard("mariah", &cases, DEFAULT_THRESHOLD);
    assert_eq!(card.teammate, "mariah");
    assert_eq!(card.total, 2);
    assert_eq!(card.passed, 2);
    assert!((card.pass_rate() - 1.0).abs() < 1e-9);
    assert_eq!(card.axis_means, [1.0; 6]);
}

#[test]
fn axis_means_average_across_scenarios() {
    // Case a: perfect (correctness 1.0). Case b: output lacks "counter"
    // → correctness 0.0, all other axes 1.0. So correctness mean = 0.5,
    // every other axis mean = 1.0.
    let mut bad = good_outcome();
    bad.output = "Sounds good, deal.".into();
    let cases = vec![case("a", good_outcome()), case("b", bad)];
    let card = build_scorecard("mariah", &cases, DEFAULT_THRESHOLD);
    assert!((card.axis_mean("correctness").unwrap() - 0.5).abs() < 1e-9);
    assert!((card.axis_mean("escalation").unwrap() - 1.0).abs() < 1e-9);
    assert!((card.axis_mean("budget").unwrap() - 1.0).abs() < 1e-9);
    // case a passes (all axes >=0.7); case b fails (correctness 0.0).
    assert_eq!(card.passed, 1);
    assert_eq!(card.total, 2);
}

#[test]
fn weakest_axis_points_at_the_lowest_mean() {
    // Make BUDGET the weakest axis: both cases blow the latency budget.
    let mut slow = good_outcome();
    slow.elapsed_ms = 99_000; // over budget_ms 5_000 → budget axis 0.0
    let cases = vec![case("a", slow.clone()), case("b", slow)];
    let card = build_scorecard("mariah", &cases, DEFAULT_THRESHOLD);
    let (axis, mean) = card.weakest_axis().expect("non-empty suite has a weakest axis");
    assert_eq!(axis, "budget");
    assert!((mean - 0.0).abs() < 1e-9);
}

#[test]
fn weakest_axis_ties_resolve_to_earliest_axis() {
    // A fully-perfect suite: every axis mean is 1.0 (a tie). The earliest
    // axis in AXIS_NAMES order (correctness) is reported.
    let card = build_scorecard("mariah", &[case("a", good_outcome())], DEFAULT_THRESHOLD);
    assert_eq!(card.weakest_axis().map(|(n, _)| n), Some("correctness"));
}

#[test]
fn empty_suite_has_no_weakest_axis_and_vacuous_pass_rate() {
    let card = build_scorecard("nobody", &[], DEFAULT_THRESHOLD);
    assert_eq!(card.total, 0);
    assert_eq!(card.passed, 0);
    assert!((card.pass_rate() - 1.0).abs() < 1e-9);
    assert_eq!(card.axis_means, [0.0; 6]);
    assert_eq!(card.weakest_axis(), None);
}

#[test]
fn axis_mean_unknown_axis_is_none() {
    let card = build_scorecard("mariah", &[case("a", good_outcome())], DEFAULT_THRESHOLD);
    assert_eq!(card.axis_mean("nonexistent"), None);
}

#[test]
fn render_matches_design_shape_and_marks_weakest() {
    let mut slow = good_outcome();
    slow.elapsed_ms = 99_000; // budget axis → 0.0, the weakest
    let card = build_scorecard("mariah", &[case("a", slow)], DEFAULT_THRESHOLD);
    let text = card.render();
    assert!(text.starts_with("teammate: mariah\n"), "got:\n{text}");
    assert!(text.contains("scenarios: 1"), "got:\n{text}");
    assert!(text.contains("correctness  1.00"), "got:\n{text}");
    assert!(text.contains("budget       0.00   <- weakest"), "got:\n{text}");
    // exactly one weakest marker
    assert_eq!(text.matches("<- weakest").count(), 1, "got:\n{text}");
}

#[test]
fn scorecard_serde_round_trips() {
    let card = build_scorecard("mariah", &[case("a", good_outcome())], DEFAULT_THRESHOLD);
    let json = serde_json::to_string(&card).expect("serialize");
    let back: Scorecard = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(card, back);
}

#[test]
fn default_threshold_is_seven_decile() {
    assert!((DEFAULT_THRESHOLD - 0.7).abs() < f64::EPSILON);
}

// ---- parse_golden_cases (pure JSONL scenario-bank parser) ----------
// Filesystem-free so these need no tempfile dep; `load_golden_cases` is a
// one-line `read_to_string` + `parse_golden_cases` wrapper.

#[test]
fn parses_golden_cases_from_jsonl_skipping_blanks() {
    let a = serde_json::to_string(&case("a", good_outcome())).unwrap();
    let b = serde_json::to_string(&case("b", good_outcome())).unwrap();
    // Two cases separated by a blank line (which must be skipped).
    let raw = format!("{a}\n\n{b}\n");
    let loaded = parse_golden_cases(&raw).expect("parse");
    assert_eq!(loaded.len(), 2);
    assert_eq!(loaded[0].scenario.name, "a");
    assert_eq!(loaded[1].scenario.name, "b");
}

#[test]
fn parse_round_trips_through_build_scorecard() {
    let line = serde_json::to_string(&case("rt", good_outcome())).unwrap();
    let cases = parse_golden_cases(&line).expect("parse");
    let card = build_scorecard("mariah", &cases, DEFAULT_THRESHOLD);
    assert_eq!(card.total, 1);
    assert_eq!(card.passed, 1);
}

#[test]
fn parse_reports_the_bad_line_number() {
    let good = serde_json::to_string(&case("ok", good_outcome())).unwrap();
    // line 1 good, line 2 malformed.
    let raw = format!("{good}\n{{not valid json\n");
    let err = parse_golden_cases(&raw).expect_err("malformed line must error");
    assert!(err.contains("line 2"), "got: {err}");
}

#[test]
fn parse_empty_input_yields_no_cases() {
    assert!(parse_golden_cases("").expect("ok").is_empty());
    assert!(parse_golden_cases("\n\n  \n").expect("ok").is_empty());
}
