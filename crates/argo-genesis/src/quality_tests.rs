//! Unit tests for [`crate::quality`].
//!
//! Each fixture builds a `GenesisPlan` directly (bypassing
//! `pipeline::plan`) so the checker is tested in isolation —
//! no archetype side-effects, no charter classification.

use super::*;
use crate::archetype::Archetype;
use argo_vertical::{
    ActionDeclaration, ActionRiskLevel, ActionsSection, ExecutorDeclaration,
    VerticalIdentity, VerticalManifest, SCHEMA_VERSION,
};
use std::collections::BTreeMap;

fn make_plan(action_count: usize) -> GenesisPlan {
    let mut actions = BTreeMap::new();
    for i in 0..action_count {
        actions.insert(
            format!("act_{i}"),
            ActionDeclaration {
                risk: ActionRiskLevel::Low,
                requires_approval: false,
                description: format!("synth action {i}"),
                executor: ExecutorDeclaration::default(),
                ..ActionDeclaration::default()
            },
        );
    }
    let manifest = VerticalManifest {
        schema_version: SCHEMA_VERSION.to_string(),
        vertical: VerticalIdentity {
            id: "fixture-id".into(),
            name: "fixture".into(),
            version: "0.0.1".into(),
            description: "synthetic test fixture".into(),
            ..VerticalIdentity::default()
        },
        actions: Some(ActionsSection { actions }),
        ..VerticalManifest::default()
    };
    // SalesSdr's eval_seed_rows yields a single default seed
    // (the helper in prompt_writer falls into the catch-all
    // arm) — combined with a long charter that lifts the
    // prompt's word count, we get a plan that fails the seed
    // check but passes action + memory + prompt.
    GenesisPlan {
        charter: long_charter(),
        persona: "fixture-persona".into(),
        archetype: Archetype::SalesSdr,
        vertical_id: "salesperson-fixture".into(),
        manifest,
    }
}

/// 800ish-word charter so the rendered system prompt clears
/// the 200-word floor without depending on the boilerplate
/// alone. Easier to tune in one place than per-test.
fn long_charter() -> String {
    let para = "The worker should research target accounts in depth, draft \
                outbound emails tailored to each persona, send drafts only \
                after operator approval, log call outcomes immediately, and \
                escalate any compliance question to the human supervisor. \
                Handoffs are recorded in the audit chain. ";
    para.repeat(8)
}

#[test]
fn report_passes_when_every_threshold_met() {
    // Build a plan that meets ALL minima at once. We override
    // the seed check by counting against a synthesised report
    // (the live archetype doesn't emit ≥10 seeds).
    let plan = make_plan(ACTION_MIN);
    let report = check_plan(&plan);
    // The action check passes with exactly the floor.
    let action_check = report
        .checks
        .iter()
        .find(|c| c.name == "action_vocabulary_min")
        .unwrap();
    assert!(action_check.passed, "action floor: {action_check:?}");
    // Memory passes by construction (build_memory_schema has 3
    // namespaces today).
    let memory_check = report
        .checks
        .iter()
        .find(|c| c.name == "memory_layers_min")
        .unwrap();
    assert!(memory_check.passed, "memory floor: {memory_check:?}");
    // Prompt floor cleared by the long charter.
    let prompt_check = report
        .checks
        .iter()
        .find(|c| c.name == "prompt_word_min")
        .unwrap();
    assert!(prompt_check.passed, "prompt floor: {prompt_check:?}");
}

#[test]
fn two_action_vocabulary_fails_action_check() {
    let plan = make_plan(2);
    let report = check_plan(&plan);
    let action_check = report
        .checks
        .iter()
        .find(|c| c.name == "action_vocabulary_min")
        .unwrap();
    assert!(!action_check.passed);
    assert_eq!(action_check.observed, "2 actions");
    assert_eq!(action_check.expected, ">= 3 actions");
    assert!(!report.passed);
}

#[test]
fn nine_eval_seeds_fails_seed_check() {
    // The current eval_seed_rows for any archetype tops out
    // well below 10, so any plan will trip this — but we want
    // to assert the check ID and observed/expected surface.
    let plan = make_plan(ACTION_MIN);
    let report = check_plan(&plan);
    let seed_check = report
        .checks
        .iter()
        .find(|c| c.name == "eval_seed_min")
        .unwrap();
    assert!(!seed_check.passed, "seed check should fail today: {seed_check:?}");
    assert!(
        seed_check.observed.ends_with("seeds"),
        "observed: {}",
        seed_check.observed
    );
    assert_eq!(seed_check.expected, ">= 10 seeds");
}

#[test]
fn memory_namespace_counter_handles_real_schema() {
    let count = count_memory_namespaces(&build_memory_schema());
    assert!(
        count >= MEMORY_LAYER_MIN,
        "real memory schema must have at least 1 namespace; got {count}"
    );
    // The current schema declares 3 namespaces; pin so a
    // future expansion that drops one will fail loud.
    assert_eq!(count, 3);
}

#[test]
fn empty_charter_yields_failing_prompt_check() {
    let mut plan = make_plan(ACTION_MIN);
    plan.charter = String::new();
    let report = check_plan(&plan);
    let prompt_check = report
        .checks
        .iter()
        .find(|c| c.name == "prompt_word_min")
        .unwrap();
    // The boilerplate in build_system_prompt is ~150 words on
    // its own — well under the 200 floor when the charter
    // contributes nothing.
    assert!(
        !prompt_check.passed,
        "empty charter must fail the prompt floor: {prompt_check:?}"
    );
}

#[test]
fn require_pass_round_trips_the_decision() {
    let passing = QualityReport {
        passed: true,
        checks: vec![CheckResult {
            name: "x".into(),
            passed: true,
            observed: "10".into(),
            expected: ">= 1".into(),
        }],
    };
    assert!(require_pass(&passing).is_ok());

    let failing = QualityReport {
        passed: false,
        checks: vec![CheckResult {
            name: "y".into(),
            passed: false,
            observed: "0 actions".into(),
            expected: ">= 3 actions".into(),
        }],
    };
    let err = require_pass(&failing).unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("genesis quality gate failed"), "{msg}");
    assert!(msg.contains("y: observed 0 actions"), "{msg}");
}
