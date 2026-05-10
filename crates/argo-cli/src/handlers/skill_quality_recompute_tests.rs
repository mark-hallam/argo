//! Unit tests for `super::skill_quality_recompute`.
//!
//! The integration smoke (`tests/skill_quality_recompute_smoke.rs`)
//! drives the full binary; these tests cover the in-process matrix
//! from the brief — dry-run-does-not-persist, persist-flow-updates-
//! score, --all-walks-marketplace, --skill-targets-single, --since-
//! filter-passes-through.

use super::*;
use argo_skill::SkillStore;
use argo_store::SessionStore;
use argo_types::SkillId;
use chrono::TimeZone;
use uuid::Uuid;

const SKILL_ALPHA: &str = r#"---
name: alpha
version: "0.1"
description: alpha
provenance:
  source: user
  hash: sha256-deadbeef
capabilities: []
---
# alpha body
"#;

const SKILL_BETA: &str = r#"---
name: beta
version: "0.1"
description: beta
provenance:
  source: user
  hash: sha256-deadbeef
capabilities: []
---
# beta body
"#;

async fn open() -> (Arc<SessionStore>, SkillStore, SkillQualityStore) {
    let inner = Arc::new(SessionStore::open(":memory:").await.unwrap());
    inner.migrate().await.unwrap();
    let skills = SkillStore::new(Arc::clone(&inner));
    let qstore = SkillQualityStore::new(Arc::clone(&inner));
    qstore.ensure_schema().await.unwrap();
    (inner, skills, qstore)
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

fn at(secs: i64) -> chrono::DateTime<chrono::Utc> {
    chrono::Utc.timestamp_opt(secs, 0).unwrap()
}

#[tokio::test]
async fn dry_run_does_not_persist() {
    let (store, skills, qstore) = open().await;
    let installed = skills.install(SKILL_ALPHA, None).await.unwrap();
    // Seed 5 successful tool_result rows so the inputs produce
    // a recognisable score (success_rate=1.0 → 100).
    for i in 0..5 {
        write_tool_result(&store, "alpha", false, 100, at(1_700_000_000 + i).timestamp_micros())
            .await;
    }
    recompute_one(&store, &qstore, &installed, None, at(1_800_000_000), true)
        .await
        .unwrap();
    // --dry-run path: nothing persisted.
    assert!(qstore.latest(installed.id).await.unwrap().is_none());
}

#[tokio::test]
async fn persist_flow_updates_score() {
    let (store, skills, qstore) = open().await;
    let installed = skills.install(SKILL_ALPHA, None).await.unwrap();
    // 6 succeeded, 4 failed → success_rate = 0.6, retry_count = 4.
    for i in 0..6 {
        write_tool_result(&store, "alpha", false, 100, at(1_700_000_000 + i).timestamp_micros())
            .await;
    }
    for i in 6..10 {
        write_tool_result(&store, "alpha", true, 100, at(1_700_000_000 + i).timestamp_micros())
            .await;
    }
    recompute_one(&store, &qstore, &installed, None, at(1_800_000_000), false)
        .await
        .unwrap();
    let persisted = qstore.latest(installed.id).await.unwrap().expect("score row");
    // base = 60, retry penalty = 8 (4 × 2), avg+p99 penalties 0 → 52.
    assert_eq!(persisted.score, 52);
}

#[tokio::test]
async fn run_with_all_walks_marketplace() {
    let (store, skills, qstore) = open().await;
    skills.install(SKILL_ALPHA, None).await.unwrap();
    skills.install(SKILL_BETA, None).await.unwrap();
    // Seed one tool_result row per skill so each has a derivable
    // QualityInputs (otherwise both score 100 via the default).
    let now = at(1_800_000_000);
    write_tool_result(&store, "alpha", false, 50, now.timestamp_micros() - 1_000_000).await;
    write_tool_result(&store, "beta", true, 50, now.timestamp_micros() - 1_000_000).await;
    let targets = resolve_targets(&skills, None, true).await.unwrap();
    assert_eq!(targets.len(), 2);
    for skill in targets {
        recompute_one(&store, &qstore, &skill, None, now, false)
            .await
            .unwrap();
    }
    // alpha persists at 100 (success_rate 1.0); beta at 0
    // (success_rate 0, retry_count 1 → base 0 - 2 = -2 clamped to 0).
    let by_skill = qstore.latest_by_skill().await.unwrap();
    assert_eq!(by_skill.len(), 2);
    assert!(by_skill.values().any(|s| *s == 100));
    assert!(by_skill.values().any(|s| *s == 0));
}

#[tokio::test]
async fn skill_flag_targets_single() {
    let (_, skills, _) = open().await;
    skills.install(SKILL_ALPHA, None).await.unwrap();
    skills.install(SKILL_BETA, None).await.unwrap();
    let targets = resolve_targets(&skills, Some("alpha"), false).await.unwrap();
    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].frontmatter.name, "alpha");
    let missing = resolve_targets(&skills, Some("gamma"), false).await;
    assert!(missing.is_err(), "unknown skill must error");
}

#[tokio::test]
async fn since_filter_passes_through() {
    let (store, skills, qstore) = open().await;
    let installed = skills.install(SKILL_ALPHA, None).await.unwrap();
    // Old failure (would drag the score down) is filtered by --since.
    write_tool_result(&store, "alpha", true, 100, at(1_600_000_000).timestamp_micros()).await;
    // Recent success only.
    write_tool_result(&store, "alpha", false, 100, at(1_700_500_000).timestamp_micros()).await;
    let since = Some(at(1_700_000_000));
    recompute_one(&store, &qstore, &installed, since, at(1_800_000_000), false)
        .await
        .unwrap();
    let persisted = qstore.latest(installed.id).await.unwrap().expect("score row");
    // Only the recent success contributes → score 100.
    assert_eq!(persisted.score, 100);
}

#[test]
fn parse_since_accepts_duration_and_rfc3339() {
    assert!(parse_since(None).unwrap().is_none());
    assert!(parse_since(Some("")).unwrap().is_none());
    let dt = parse_since(Some("2026-05-01T00:00:00Z")).unwrap();
    assert!(dt.is_some());
    let lookback = parse_since(Some("24h")).unwrap();
    assert!(lookback.is_some());
    assert!(parse_since(Some("not-a-time")).is_err());
}

#[test]
fn unknown_skill_id_struct_smoke() {
    // Sanity check: SkillId is the public type the brief's
    // signature originally proposed; we accept names but the
    // store still keys on SkillId — this confirms the type path
    // resolves at the test-crate boundary.
    let _ = SkillId::new();
}
