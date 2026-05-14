//! Claim-3 coverage for Genesis eval seed emission.
//!
//! This test stays fully offline: the current Genesis pipeline uses
//! deterministic archetype selection and deterministic starter seed
//! generation, so no provider transcript or API key is needed.

use argo_genesis::eval_seed::{Difficulty, EvalSeed};
use argo_genesis::{plan, write_vertical};
use serde::Deserialize;
use std::collections::BTreeSet;

#[derive(Debug, Deserialize)]
struct SeedDoc {
    seed: Vec<EvalSeed>,
}

fn fixture_charter() -> &'static str {
    include_str!("fixtures/eval_seed_test_charter.md")
}

#[test]
fn generated_vertical_carries_schema_valid_eval_seed_fixtures() {
    let plan = plan(fixture_charter(), "maya").expect("fixture charter plans");
    let dir = tempfile::tempdir().expect("tempdir");
    let out = dir.path().join(&plan.vertical_id);
    write_vertical(&plan, &out).expect("vertical writes");

    let seed_body = std::fs::read_to_string(out.join("eval-seed.toml"))
        .expect("deterministic eval seed toml exists");
    let seed_doc: SeedDoc = toml::from_str(&seed_body).expect("eval seed toml schema");

    assert!(
        seed_doc.seed.len() >= 5,
        "Genesis must emit at least 5 eval fixtures, got {}",
        seed_doc.seed.len()
    );
    assert_unique_ids(&seed_doc.seed);
    assert!(seed_doc.seed.iter().all(seed_has_required_fields));
    assert!(
        seed_doc.seed.iter().any(is_safety_seed),
        "expected at least one safety/honesty-style seed: {:#?}",
        seed_doc.seed
    );

    let jsonl_body =
        std::fs::read_to_string(out.join("eval/seed.jsonl")).expect("legacy eval jsonl exists");
    assert!(
        jsonl_body.lines().all(|line| {
            serde_json::from_str::<serde_json::Value>(line)
                .map(|value| {
                    value.get("id").is_some()
                        && value.get("prompt").is_some()
                        && value.get("expect_action_type").is_some()
                })
                .unwrap_or(false)
        }),
        "legacy eval jsonl must remain schema-valid"
    );

    let (_registry, errors) = argo_vertical::load_dir(dir.path()).expect("round-trip load");
    assert!(errors.is_empty(), "vertical load errors: {errors:?}");
}

#[test]
fn vague_charter_surfaces_quality_warning_instead_of_silent_zero_eval() {
    let (plan, report) = argo_genesis::plan_with_quality("help with marketing when asked", "maya")
        .expect("vague-but-classifiable charter plans");
    let eval_seed_check = report
        .checks
        .iter()
        .find(|check| check.name == "eval_seed_min")
        .expect("eval seed quality check exists");

    assert!(
        !eval_seed_check.passed,
        "legacy eval/seed.jsonl path should warn when it has too few seeds"
    );
    assert!(
        report.format_failures().contains("eval_seed_min"),
        "quality report must name the warning"
    );

    let dir = tempfile::tempdir().expect("tempdir");
    let out = dir.path().join(&plan.vertical_id);
    write_vertical(&plan, &out).expect("vertical still materializes");
    let seed_body = std::fs::read_to_string(out.join("eval-seed.toml"))
        .expect("deterministic eval seed toml exists");
    let seed_doc: SeedDoc = toml::from_str(&seed_body).expect("eval seed toml schema");
    assert!(
        seed_doc.seed.len() >= 5,
        "warning path must not silently emit zero deterministic eval seeds"
    );
}

fn assert_unique_ids(seeds: &[EvalSeed]) {
    let ids: BTreeSet<_> = seeds.iter().map(|seed| seed.id.as_str()).collect();
    assert_eq!(ids.len(), seeds.len(), "eval seed ids must be unique");
}

fn seed_has_required_fields(seed: &EvalSeed) -> bool {
    !seed.id.trim().is_empty()
        && !seed.task.trim().is_empty()
        && !seed.expected_summary.trim().is_empty()
        && !seed.rubric_keywords.is_empty()
}

fn is_safety_seed(seed: &EvalSeed) -> bool {
    if seed.difficulty == Difficulty::Hard {
        return true;
    }
    let text = format!(
        "{} {} {}",
        seed.task,
        seed.expected_summary,
        seed.rubric_keywords.join(" ")
    )
    .to_ascii_lowercase();
    ["risk", "review", "evidence", "approval", "human"]
        .iter()
        .any(|tag| text.contains(tag))
}
