//! Tests for the historical-replay seam: pairing real interactions to
//! rubrics by name (pure) + scoring them through the existing runner +
//! scorecard via an in-memory [`ProductionOutcomeSource`].

use super::*;
use crate::teammate::{ScenarioOutcome, TeammateScenario};

fn scenario(name: &str, should_escalate: bool) -> TeammateScenario {
    TeammateScenario {
        name: name.into(),
        situation: "a customer offer".into(),
        expected_substrings: vec!["counter".into()],
        should_escalate,
        required_skills: vec![],
        forbidden_claim: None,
        budget_ms: 5_000,
        budget_usd_micros: 20_000,
    }
}

/// An outcome that scores 1.0 on every axis for `scenario(_, false)`.
fn good_outcome() -> ScenarioOutcome {
    ScenarioOutcome {
        output: "I'll counter at $82.".into(),
        escalated: false,
        skills_used: vec![],
        honest: true,
        elapsed_ms: 100,
        cost_usd_micros: 100,
    }
}

/// An outcome missing the expected substring → correctness 0.0 → fails.
fn bad_outcome() -> ScenarioOutcome {
    ScenarioOutcome {
        output: "ok, deal.".into(),
        escalated: false,
        skills_used: vec![],
        honest: true,
        elapsed_ms: 100,
        cost_usd_micros: 100,
    }
}

fn interaction(scenario_name: &str, outcome: ScenarioOutcome) -> ReplayedInteraction {
    ReplayedInteraction {
        scenario_name: scenario_name.into(),
        outcome,
    }
}

// ---- pairing (pure) -------------------------------------------------

#[test]
fn pairs_interaction_to_rubric_by_name() {
    let scenarios = vec![scenario("support/a", false)];
    let interactions = vec![interaction("support/a", good_outcome())];
    let pairing = pair_replayed(&scenarios, &interactions);
    assert_eq!(pairing.cases.len(), 1);
    assert_eq!(pairing.cases[0].scenario.name, "support/a");
    assert!(pairing.unmatched.is_empty());
    assert!(pairing.uncovered.is_empty());
}

#[test]
fn unknown_scenario_name_is_unmatched_not_dropped_silently() {
    let scenarios = vec![scenario("support/a", false)];
    let interactions = vec![
        interaction("support/a", good_outcome()),
        interaction("ghost/x", good_outcome()),
    ];
    let pairing = pair_replayed(&scenarios, &interactions);
    assert_eq!(pairing.cases.len(), 1);
    assert_eq!(pairing.unmatched, vec!["ghost/x".to_string()]);
}

#[test]
fn rubric_with_no_interaction_is_reported_uncovered() {
    let scenarios = vec![scenario("support/a", false), scenario("support/b", false)];
    let interactions = vec![interaction("support/a", good_outcome())];
    let pairing = pair_replayed(&scenarios, &interactions);
    assert_eq!(pairing.uncovered, vec!["support/b".to_string()]);
}

#[test]
fn unmatched_is_sorted_and_cases_keep_interaction_order() {
    let scenarios = vec![scenario("s1", false)];
    let interactions = vec![
        interaction("zzz", good_outcome()),
        interaction("s1", good_outcome()),
        interaction("aaa", good_outcome()),
    ];
    let pairing = pair_replayed(&scenarios, &interactions);
    // unmatched sorted deterministically
    assert_eq!(pairing.unmatched, vec!["aaa".to_string(), "zzz".to_string()]);
    // the single matched case is the s1 interaction
    assert_eq!(pairing.cases.len(), 1);
    assert_eq!(pairing.cases[0].scenario.name, "s1");
}

// ---- end-to-end through the source + existing scorer ----------------

#[tokio::test]
async fn replay_report_scores_real_outcomes_via_runner_and_scorecard() {
    let scenarios = vec![scenario("s/pass", false), scenario("s/fail", false)];
    let source = VecOutcomeSource::new(vec![
        interaction("s/pass", good_outcome()),
        interaction("s/fail", bad_outcome()),
    ]);
    let out = replay_report(&source, "mariah", &scenarios, 50, 0.5)
        .await
        .expect("replay report");

    assert_eq!(out.teammate, "mariah");
    assert_eq!(out.scored, 2);
    assert_eq!(out.report.total, 2);
    assert_eq!(out.report.passed, 1); // good passes, bad fails
    assert_eq!(out.scorecard.total, 2);
    assert!((out.scorecard.pass_rate() - 0.5).abs() < 1e-9);
    assert!(out.unmatched.is_empty());
    assert!(out.uncovered.is_empty());
}

#[tokio::test]
async fn replay_report_respects_limit() {
    let scenarios = vec![scenario("s/a", false)];
    let source = VecOutcomeSource::new(vec![
        interaction("s/a", good_outcome()),
        interaction("s/a", good_outcome()),
        interaction("s/a", good_outcome()),
    ]);
    let out = replay_report(&source, "t", &scenarios, 2, 0.5)
        .await
        .expect("ok");
    assert_eq!(out.scored, 2); // limit caps the fetch
}

#[tokio::test]
async fn replay_report_empty_source_is_vacuous_and_all_uncovered() {
    let scenarios = vec![scenario("s/a", false), scenario("s/b", false)];
    let source = VecOutcomeSource::default();
    let out = replay_report(&source, "t", &scenarios, 50, 0.5)
        .await
        .expect("ok");
    assert_eq!(out.scored, 0);
    assert_eq!(out.report.total, 0);
    assert!((out.report.pass_rate() - 1.0).abs() < 1e-9); // vacuous
    assert_eq!(out.uncovered.len(), 2);
}

#[tokio::test]
async fn unmatched_interactions_are_not_scored() {
    let scenarios = vec![scenario("known", false)];
    let source = VecOutcomeSource::new(vec![
        interaction("known", good_outcome()),
        interaction("unknown", good_outcome()),
    ]);
    let out = replay_report(&source, "t", &scenarios, 50, 0.5)
        .await
        .expect("ok");
    assert_eq!(out.scored, 1);
    assert_eq!(out.unmatched, vec!["unknown".to_string()]);
}

// ---- serde shape stability -----------------------------------------

#[test]
fn replayed_interaction_serde_roundtrips() {
    let it = interaction("s/a", good_outcome());
    let json = serde_json::to_string(&it).expect("serialize");
    let back: ReplayedInteraction = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(it, back);
}

#[tokio::test]
async fn replay_report_serde_roundtrips() {
    let scenarios = vec![scenario("s/a", false)];
    let source = VecOutcomeSource::new(vec![interaction("s/a", good_outcome())]);
    let out = replay_report(&source, "t", &scenarios, 50, 0.5)
        .await
        .expect("ok");
    let json = serde_json::to_string(&out).expect("serialize");
    let back: ReplayReport = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(out, back);
}
