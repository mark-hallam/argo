//! End-to-end smoke for `argo skill quality-recompute`
//! (mission-skill-quality-cron). Seeds the local skill marketplace
//! and audit log, runs the binary, asserts that a quality score
//! row landed via the standard persistence path.

mod common;

use argo_skill::{SkillQualityStore, SkillStore};
use argo_store::SessionStore;
use common::argo_bin;
use std::path::Path;
use std::process::{Command, Output};
use std::sync::Arc;
use tempfile::TempDir;
use uuid::Uuid;

const SKILL_BODY: &str = r#"---
name: cron-tick-skill
version: "0.1"
description: cron-tick-skill
provenance:
  source: user
  hash: sha256-deadbeef
capabilities: []
---
# cron-tick-skill body
"#;

struct World {
    home: TempDir,
    work: TempDir,
    store: Arc<SessionStore>,
}

#[tokio::test]
async fn quality_recompute_all_persists_score_for_seeded_skill() {
    let world = seed().await;

    let output = run(&world, &["skill", "quality-recompute", "--all"]);
    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(" -> "),
        "expected score-delta line, got stdout: {stdout}"
    );
    assert!(
        stdout.contains("(+"),
        "first recompute should report a positive delta from 0, got: {stdout}"
    );

    let qstore = SkillQualityStore::new(Arc::clone(&world.store));
    qstore.ensure_schema().await.unwrap();
    let scores = qstore.latest_all().await.unwrap();
    assert_eq!(
        scores.len(),
        1,
        "exactly one persisted score expected, got {scores:?}"
    );
    // 5/5 succeeded → success_rate 1.0 → score 100 with no penalties.
    assert_eq!(scores[0].score, 100);
}

async fn seed() -> World {
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
    skills.install(SKILL_BODY, None).await.unwrap();
    let base = chrono::Utc::now().timestamp_micros();
    for i in 0..5_i64 {
        write_tool_result(&store, "cron-tick-skill", false, 120, base + i).await;
    }
    World { home, work, store }
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
