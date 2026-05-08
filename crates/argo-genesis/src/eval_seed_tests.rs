use super::*;
use crate::archetype::Archetype;
use crate::manifest_writer::build_manifest;
use serde::Deserialize;
use std::collections::BTreeSet;

#[derive(Debug, Deserialize)]
struct SeedDoc {
    seed: Vec<EvalSeed>,
}

fn marketing_actions() -> Vec<SuggestedAction> {
    let manifest = build_manifest(
        "be my marketing intern, run weekly campaigns to cold leads",
        Archetype::MarketingIntern,
        "maya",
        "marketing-intern-maya",
    )
    .expect("manifest");
    suggested_actions_from_manifest(manifest.actions.as_ref().expect("actions"))
}

#[test]
fn generate_returns_ten_to_twenty_seeds_for_demo_role() {
    let seeds = generate(
        "be my marketing intern, run weekly campaigns to cold leads",
        &marketing_actions(),
    )
    .expect("generate");

    assert!((10..=20).contains(&seeds.len()));
}

#[test]
fn difficulty_distribution_is_roughly_expected_split() {
    let seeds = generate("be my sales sdr", &marketing_actions()).expect("generate");
    let easy = seeds
        .iter()
        .filter(|s| s.difficulty == Difficulty::Easy)
        .count();
    let medium = seeds
        .iter()
        .filter(|s| s.difficulty == Difficulty::Medium)
        .count();
    let hard = seeds
        .iter()
        .filter(|s| s.difficulty == Difficulty::Hard)
        .count();

    assert_eq!((easy, medium, hard), (4, 4, 2));
}

#[test]
fn every_seed_has_required_fields_and_keywords() {
    let seeds = generate("support customers", &marketing_actions()).expect("generate");

    for seed in seeds {
        assert!(!seed.id.is_empty());
        assert!(!seed.task.is_empty());
        assert!(!seed.expected_summary.is_empty());
        assert!(!seed.rubric_keywords.is_empty());
        assert!(seed.rubric_keywords.len() <= 5);
        assert!(seed.rubric_keywords.iter().all(|k| {
            k.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        }));
    }
}

#[test]
fn duplicate_ids_do_not_appear() {
    let seeds = generate("coordinate project status", &marketing_actions()).expect("generate");
    let ids: BTreeSet<_> = seeds.iter().map(|s| s.id.as_str()).collect();

    assert_eq!(ids.len(), seeds.len());
}

#[test]
fn to_toml_round_trips() {
    let seeds =
        generate("research market and draft listings", &marketing_actions()).expect("generate");
    let body = to_toml(&seeds);
    let parsed: SeedDoc = toml::from_str(&body).expect("parse seed doc");

    assert_eq!(parsed.seed, seeds);
}

#[test]
fn empty_actions_slice_still_works_from_role_description() {
    let seeds = generate("bookkeeping assistant reconciles invoices", &[]).expect("generate");

    assert_eq!(seeds.len(), 10);
    assert!(
        seeds
            .iter()
            .all(|s| { s.task.contains("bookkeeping") || s.task.contains("summarize_context") })
    );
}
