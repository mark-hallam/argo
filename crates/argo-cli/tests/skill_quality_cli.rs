//! Integration tests for `argo skill quality*` (Stream
//! `claude-skill-quality-judge`). Covers the full
//! recompute → quality → quality-list cycle plus the
//! --quality-min filter on `argo skill list`.

mod common;

use argo_skill::SkillStore;
use argo_store::SessionStore;
use common::argo_bin;
use serde_json::Value;
use std::path::Path;
use std::process::{Command, Output};
use std::sync::Arc;
use tempfile::TempDir;

const SKILL_BODY: &str = r#"---
name: spec-skill
version: "0.1"
description: spec-skill
provenance:
  source: user
  hash: sha256-deadbeef
capabilities: []
---
# spec-skill

A skill body for testing.
"#;

struct World {
    home: TempDir,
    work: TempDir,
}

#[tokio::test]
async fn recompute_then_quality_then_list_full_cycle() {
    let world = seed(SKILL_BODY).await;

    let recomputed = run(
        &world,
        &[
            "skill",
            "quality-recompute",
            "spec-skill",
            "--success-rate",
            "0.6",
            "--retry-count",
            "5",
            "--json",
        ],
    );
    assert_success(&recomputed);
    let payload: Value = serde_json::from_slice(&recomputed.stdout).unwrap();
    let arr = payload["recomputed"].as_array().unwrap();
    assert_eq!(arr.len(), 1);
    let score = arr[0]["score"].as_u64().unwrap();
    // base = 0.6 * 100 = 60; retry penalty = 10 → 50.
    assert_eq!(score, 50);

    let single = run(
        &world,
        &["skill", "quality", "spec-skill", "--json"],
    );
    assert_success(&single);
    let detail: Value = serde_json::from_slice(&single.stdout).unwrap();
    assert_eq!(detail["score"], 50);

    let listed = run(&world, &["skill", "quality-list", "--json"]);
    assert_success(&listed);
    let rows: Value = serde_json::from_slice(&listed.stdout).unwrap();
    assert_eq!(rows.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn skill_list_quality_min_filters_low_scoring_skills() {
    let world = seed(SKILL_BODY).await;
    let recomputed = run(
        &world,
        &[
            "skill",
            "quality-recompute",
            "spec-skill",
            "--success-rate",
            "0.4",
            "--json",
        ],
    );
    assert_success(&recomputed);

    let above_threshold = run(
        &world,
        &["skill", "list", "--quality-min", "80", "--json"],
    );
    assert_success(&above_threshold);
    // The skill scored 40 (0.4 * 100); --quality-min 80 should drop it.
    let lines: Vec<&[u8]> = above_threshold.stdout.split(|b| *b == b'\n').collect();
    let non_empty: Vec<&[u8]> = lines.into_iter().filter(|l| !l.is_empty()).collect();
    assert!(non_empty.is_empty(), "expected no rows but got {non_empty:?}");

    let below_threshold = run(
        &world,
        &["skill", "list", "--quality-min", "30", "--json"],
    );
    assert_success(&below_threshold);
    let lines: Vec<&[u8]> = below_threshold.stdout.split(|b| *b == b'\n').collect();
    let non_empty_count = lines.into_iter().filter(|l| !l.is_empty()).count();
    assert_eq!(non_empty_count, 1);
}

async fn seed(skill_md: &str) -> World {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(work.path().join(".skycot")).unwrap();
    let store = Arc::new(
        SessionStore::open(home.path().join("sessions.db"))
            .await
            .unwrap(),
    );
    store.migrate().await.unwrap();
    let skills = SkillStore::new(Arc::clone(&store));
    skills.install(skill_md, None).await.unwrap();
    World { home, work }
}

fn run(world: &World, args: &[&str]) -> Output {
    run_in(&world.home, world.work.path(), args)
}

fn run_in(home: &TempDir, cwd: &Path, args: &[&str]) -> Output {
    Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .current_dir(cwd)
        .args(args)
        .output()
        .unwrap()
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "status: {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
