//! Integration tests for `argo skill quality*` (Stream
//! `claude-skill-quality-judge`, refreshed for
//! `mission-skill-quality-cron`). Covers the full
//! `recompute → quality → quality-list` cycle plus the
//! `--quality-min` filter on `argo skill list`, now driven through
//! audit-derived inputs rather than the (removed) operator-input
//! stand-in flags.

mod common;

use argo_skill::SkillStore;
use argo_store::SessionStore;
use common::argo_bin;
use serde_json::Value;
use std::path::Path;
use std::process::{Command, Output};
use std::sync::Arc;
use tempfile::TempDir;
use uuid::Uuid;

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
    let world = seed_with_audit(SKILL_BODY, "spec-skill", 6, 4).await;

    let recomputed = run(
        &world,
        &["skill", "quality-recompute", "--skill", "spec-skill"],
    );
    assert_success(&recomputed);
    let stdout = String::from_utf8_lossy(&recomputed.stdout);
    assert!(
        stdout.contains(" -> "),
        "expected score-delta line in stdout, got: {stdout}"
    );

    let single = run(&world, &["skill", "quality", "spec-skill", "--json"]);
    assert_success(&single);
    let detail: Value = serde_json::from_slice(&single.stdout).unwrap();
    // base = 60, retry penalty = 4 × 2 = 8, latency penalties 0 → 52.
    assert_eq!(detail["score"], 52);

    let listed = run(&world, &["skill", "quality-list", "--json"]);
    assert_success(&listed);
    let rows: Value = serde_json::from_slice(&listed.stdout).unwrap();
    assert_eq!(rows.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn skill_list_quality_min_filters_low_scoring_skills() {
    let world = seed_with_audit(SKILL_BODY, "spec-skill", 4, 6).await;
    let recomputed = run(
        &world,
        &["skill", "quality-recompute", "--skill", "spec-skill"],
    );
    assert_success(&recomputed);

    let above_threshold = run(&world, &["skill", "list", "--quality-min", "80", "--json"]);
    assert_success(&above_threshold);
    let lines: Vec<&[u8]> = above_threshold.stdout.split(|b| *b == b'\n').collect();
    let non_empty: Vec<&[u8]> = lines.into_iter().filter(|l| !l.is_empty()).collect();
    assert!(non_empty.is_empty(), "expected no rows but got {non_empty:?}");

    let below_threshold = run(&world, &["skill", "list", "--quality-min", "20", "--json"]);
    assert_success(&below_threshold);
    let lines: Vec<&[u8]> = below_threshold.stdout.split(|b| *b == b'\n').collect();
    let non_empty_count = lines.into_iter().filter(|l| !l.is_empty()).count();
    assert_eq!(non_empty_count, 1);
}

#[tokio::test]
async fn dry_run_does_not_persist_via_cli() {
    let world = seed_with_audit(SKILL_BODY, "spec-skill", 6, 4).await;
    let recomputed = run(
        &world,
        &[
            "skill",
            "quality-recompute",
            "--skill",
            "spec-skill",
            "--dry-run",
        ],
    );
    assert_success(&recomputed);
    // No persisted row → `quality-list` returns []
    let listed = run(&world, &["skill", "quality-list", "--json"]);
    assert_success(&listed);
    let rows: Value = serde_json::from_slice(&listed.stdout).unwrap();
    assert_eq!(rows.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn all_with_empty_marketplace_prints_no_skills_found() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(work.path().join(".skycot")).unwrap();
    let store = Arc::new(
        SessionStore::open(home.path().join("sessions.db"))
            .await
            .unwrap(),
    );
    store.migrate().await.unwrap();
    let world = World { home, work };
    let output = run(&world, &["skill", "quality-recompute", "--all"]);
    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("no skills found"),
        "expected `no skills found`, got: {stdout}"
    );
}

#[tokio::test]
async fn skill_and_all_together_are_rejected() {
    let world = seed_with_audit(SKILL_BODY, "spec-skill", 1, 0).await;
    let output = run(
        &world,
        &[
            "skill",
            "quality-recompute",
            "--all",
            "--skill",
            "spec-skill",
        ],
    );
    assert!(
        !output.status.success(),
        "clap should reject --all + --skill, got success: {}",
        String::from_utf8_lossy(&output.stderr),
    );
}

async fn seed_with_audit(
    skill_md: &str,
    tool_name: &str,
    successes: usize,
    failures: usize,
) -> World {
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
    let base = chrono::Utc::now().timestamp_micros();
    for i in 0..successes {
        write_tool_result(&store, tool_name, false, 100, base + i64::try_from(i).unwrap()).await;
    }
    for i in 0..failures {
        write_tool_result(
            &store,
            tool_name,
            true,
            100,
            base + i64::try_from(successes + i).unwrap(),
        )
        .await;
    }
    World { home, work }
}

async fn write_tool_result(
    store: &SessionStore,
    tool: &str,
    is_error: bool,
    duration_ms: u64,
    created_at_micros: i64,
) {
    let id = Uuid::now_v7();
    let payload = serde_json::json!({
        "tool": tool,
        "call_id": Uuid::now_v7().to_string(),
        "is_error": is_error,
        "duration_ms": duration_ms,
    });
    let payload_json = serde_json::to_string(&payload).unwrap();
    let parent_hash = vec![0u8; 32];
    let self_hash = vec![0u8; 32];
    sqlx::query(
        "INSERT INTO audit_record \
         (id, session_id, kind, payload_json, parent_hash, self_hash, created_at) \
         VALUES (?, NULL, 'tool_result', ?, ?, ?, ?)",
    )
    .bind(id.as_bytes().as_slice())
    .bind(payload_json)
    .bind(parent_hash)
    .bind(self_hash)
    .bind(created_at_micros)
    .execute(store.pool())
    .await
    .unwrap();
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
