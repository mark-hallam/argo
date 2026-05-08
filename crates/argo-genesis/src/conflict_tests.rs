use super::*;
use crate::plan;
use argo_types::ArgoError;
use argo_vertical::{ActionDeclaration, ActionRiskLevel, ExecutorDeclaration};
use std::path::Path;

fn write_existing(root: &Path, manifest: &VerticalManifest, memory: &str) {
    let dir = root.join(&manifest.vertical.id);
    std::fs::create_dir_all(dir.join("memory")).unwrap();
    let body = toml::to_string_pretty(manifest).unwrap();
    std::fs::write(dir.join("vertical.toml"), body).unwrap();
    std::fs::write(dir.join("memory/schema.toml"), memory).unwrap();
}

fn generated_sales() -> GenesisPlan {
    plan("sales sdr for b2b saas outbound prospecting", "sam").unwrap()
}

#[test]
fn identical_role_to_existing_vertical_scores_high() {
    let generated = generated_sales();
    let dir = tempfile::tempdir().unwrap();
    write_existing(
        dir.path(),
        &generated.manifest,
        "schema_version = \"1\"\n[namespaces.working]\n[namespaces.episodic]\n[namespaces.skills]\n",
    );

    let report = detect(&generated, dir.path()).expect("detect");

    assert!(highest_score(&report).unwrap() >= 0.9, "{report:?}");
}

#[test]
fn action_overlap_without_memory_overlap_contributes_to_score() {
    let generated = generated_sales();
    let dir = tempfile::tempdir().unwrap();
    let mut existing = generated.manifest.clone();
    existing.vertical.id = "different-role".into();
    existing.vertical.name = "Different Role".into();
    existing.vertical.description = "Handles inventory adjustments".into();
    let actions = existing.actions.as_mut().unwrap();
    actions.actions.remove("log_call_outcome");
    actions.actions.insert(
        "adjust_inventory".into(),
        ActionDeclaration {
            risk: ActionRiskLevel::Low,
            requires_approval: false,
            description: "Adjust an internal inventory note".into(),
            executor: ExecutorDeclaration::default(),
            ..ActionDeclaration::default()
        },
    );
    write_existing(
        dir.path(),
        &existing,
        "schema_version = \"1\"\n[[scope]]\nid = \"stock\"\n",
    );

    let report = detect(&generated, dir.path()).expect("detect");
    let found = &report.matches[0];

    assert!((0.17..=0.25).contains(&found.score), "{found:?}");
    assert!(found
        .reasons
        .iter()
        .any(|r| r.contains("action vocabulary")));
    assert!(!found.reasons.iter().any(|r| r.contains("memory namespace")));
}

#[test]
fn empty_verticals_root_returns_empty_matches() {
    let dir = tempfile::tempdir().unwrap();
    let report = detect(&generated_sales(), dir.path()).expect("detect");

    assert!(report.matches.is_empty());
    assert_eq!(highest_score(&report), None);
}

#[test]
fn missing_verticals_root_returns_io_error() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("missing");
    let err = detect(&generated_sales(), &missing).expect_err("missing root");

    assert!(matches!(err, ArgoError::Io(_)));
}

#[test]
fn every_match_score_is_in_range() {
    let generated = generated_sales();
    let dir = tempfile::tempdir().unwrap();
    write_existing(
        dir.path(),
        &generated.manifest,
        "schema_version = \"1\"\n[[scope]]\nid = \"working\"\n",
    );

    let report = detect(&generated, dir.path()).expect("detect");

    assert!(report
        .matches
        .iter()
        .all(|m| (0.0..=1.0).contains(&m.score)));
}
