//! Unit tests for `ab_test`. Lives in a sibling file so the
//! main module stays under the per-file 500-LOC budget.
use super::{
    create_run, ensure_ab_schema, get_run, maybe_complete, maybe_complete_with_quality,
    promote, record_outcome, select_variant, significance, variant_counts,
    QualityWeightConfig, Variant, VariantCounts, WinnerReason,
    CHI_SQUARE_CRIT_DF1_ALPHA_05,
};
use crate::quality::{QualityInputs, SkillQualityStore};
use argo_store::SessionStore;
use argo_types::SkillId;
use chrono::{Duration, Utc};
use std::sync::Arc;

async fn open_store() -> SessionStore {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    ensure_ab_schema(&store).await.unwrap();
    store
}

async fn insert_skill(store: &SessionStore, name: &str, version: &str) {
    insert_skill_with_id(store, name, version, &uuid::Uuid::now_v7()).await;
}

async fn insert_skill_with_id(
    store: &SessionStore,
    name: &str,
    version: &str,
    id: &uuid::Uuid,
) {
    let id_bytes = id.as_bytes().to_vec();
    sqlx::query(
        "INSERT INTO skill (id, name, version, body, frontmatter, source, hash, learned_in, superseded_by, created_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id_bytes)
    .bind(name)
    .bind(version)
    .bind("body")
    .bind("{}")
    .bind("user")
    .bind(vec![0u8; 32])
    .bind(Option::<&[u8]>::None)
    .bind(Option::<&[u8]>::None)
    .bind(0_i64)
    .execute(store.pool())
    .await
    .unwrap();
}

async fn open_store_arc() -> Arc<SessionStore> {
    Arc::new(open_store().await)
}

async fn make_qstore(store: Arc<SessionStore>) -> SkillQualityStore {
    let qstore = SkillQualityStore::new(store);
    qstore.ensure_schema().await.unwrap();
    qstore
}

fn inputs_for_score(success_rate: f64) -> QualityInputs {
    // Drives `compute_score` to `success_rate * 100` with no
    // latency or retry penalty so tests can pin scores precisely.
    QualityInputs {
        success_rate,
        avg_latency_ms: 0.0,
        retry_count: 0,
        p99_latency_ms: 0.0,
    }
}

/// Set up a run + outcomes with the requested per-variant success
/// counts (out of 100 trials each). `seed` UUIDs differentiate
/// skill ids so tests sharing an in-memory store don't collide.
async fn setup_run(
    store: &SessionStore,
    seed_a: u128,
    seed_b: u128,
    successes_a: usize,
    successes_b: usize,
) -> (String, SkillId, SkillId) {
    let a_uuid = uuid::Uuid::from_u128(seed_a);
    let b_uuid = uuid::Uuid::from_u128(seed_b);
    insert_skill_with_id(store, "demo", "v1", &a_uuid).await;
    insert_skill_with_id(store, "demo", "v2", &b_uuid).await;
    let run_id = create_run(store, "demo", "v1", "v2", 100).await.unwrap();
    for i in 0..100 {
        record_outcome(store, &run_id, &format!("a-{i}"), Variant::A, i < successes_a, Some(10))
            .await
            .unwrap();
    }
    for i in 0..100 {
        record_outcome(store, &run_id, &format!("b-{i}"), Variant::B, i < successes_b, Some(10))
            .await
            .unwrap();
    }
    (run_id, SkillId(a_uuid), SkillId(b_uuid))
}

#[test]
fn select_variant_is_deterministic() {
    let v1 = select_variant("run-1", "inv-1");
    let v2 = select_variant("run-1", "inv-1");
    assert_eq!(v1, v2);
}

#[test]
fn select_variant_splits_roughly_evenly_over_many_inputs() {
    let mut a_count = 0;
    let mut b_count = 0;
    for i in 0..1000 {
        match select_variant("run-1", &format!("inv-{i}")) {
            Variant::A => a_count += 1,
            Variant::B => b_count += 1,
        }
    }
    // Expect ~500/500. Allow ±100 for hash variance.
    assert!((400..=600).contains(&a_count), "a_count = {a_count}");
    assert!((400..=600).contains(&b_count), "b_count = {b_count}");
}

#[test]
fn significance_refuses_below_min_sample() {
    let a = VariantCounts { trials: 10, successes: 9 };
    let b = VariantCounts { trials: 10, successes: 1 };
    let sig = significance(a, b, 30);
    assert!(!sig.significant);
}

#[test]
fn significance_detects_clear_winner_above_min_sample() {
    let a = VariantCounts { trials: 100, successes: 90 };
    let b = VariantCounts { trials: 100, successes: 30 };
    let sig = significance(a, b, 30);
    assert!(sig.significant);
    assert!(sig.chi_square > CHI_SQUARE_CRIT_DF1_ALPHA_05);
}

#[test]
fn significance_does_not_fire_on_equal_rates() {
    let a = VariantCounts { trials: 100, successes: 50 };
    let b = VariantCounts { trials: 100, successes: 50 };
    let sig = significance(a, b, 30);
    assert!(!sig.significant);
}

#[tokio::test]
async fn create_run_rejects_identical_variants() {
    let store = open_store().await;
    insert_skill(&store, "demo", "v1").await;
    let err = create_run(&store, "demo", "v1", "v1", 100).await.unwrap_err();
    assert!(err.to_string().contains("must differ"));
}

#[tokio::test]
async fn create_run_rejects_unknown_variant_version() {
    let store = open_store().await;
    insert_skill(&store, "demo", "v1").await;
    let err = create_run(&store, "demo", "v1", "v2", 100).await.unwrap_err();
    assert!(err.to_string().contains("no version 'v2'"));
}

#[tokio::test]
async fn run_round_trip_records_outcomes_and_completes() {
    let store = open_store().await;
    insert_skill(&store, "demo", "v1").await;
    insert_skill(&store, "demo", "v2").await;
    let run_id = create_run(&store, "demo", "v1", "v2", 100).await.unwrap();
    // 60 successes for A, 30 for B (clear winner)
    for i in 0..100 {
        record_outcome(&store, &run_id, &format!("inv-{i}"), Variant::A, i < 60, Some(10))
            .await
            .unwrap();
    }
    for i in 0..100 {
        record_outcome(&store, &run_id, &format!("inv-b-{i}"), Variant::B, i < 30, Some(10))
            .await
            .unwrap();
    }
    let sig = maybe_complete(&store, &run_id, 30).await.unwrap().unwrap();
    assert!(sig.significant);
    let run = get_run(&store, &run_id).await.unwrap().unwrap();
    assert_eq!(run.status, "completed");
    assert_eq!(run.winner.as_deref(), Some("v1"));
}

#[tokio::test]
async fn promote_marks_loser_superseded_by_winner() {
    let store = open_store().await;
    insert_skill(&store, "demo", "v1").await;
    insert_skill(&store, "demo", "v2").await;
    let run_id = create_run(&store, "demo", "v1", "v2", 50).await.unwrap();
    // Force a winner manually for this test
    sqlx::query("UPDATE skill_ab_test_run SET status='completed', winner='v2' WHERE id = ?")
        .bind(&run_id)
        .execute(store.pool())
        .await
        .unwrap();
    let winner = promote(&store, &run_id, None).await.unwrap();
    assert_eq!(winner, "v2");
    let row: Option<(Option<Vec<u8>>,)> = sqlx::query_as(
        "SELECT superseded_by FROM skill WHERE name = ? AND version = ?",
    )
    .bind("demo")
    .bind("v1")
    .fetch_optional(store.pool())
    .await
    .unwrap();
    assert!(row.unwrap().0.is_some(), "loser should be marked superseded");
}

#[tokio::test]
async fn record_outcome_is_idempotent_on_invocation_id() {
    let store = open_store().await;
    insert_skill(&store, "demo", "v1").await;
    insert_skill(&store, "demo", "v2").await;
    let run_id = create_run(&store, "demo", "v1", "v2", 5).await.unwrap();
    record_outcome(&store, &run_id, "inv-1", Variant::A, true, None)
        .await
        .unwrap();
    record_outcome(&store, &run_id, "inv-1", Variant::A, false, None)
        .await
        .unwrap();
    let (a, _) = variant_counts(&store, &run_id).await.unwrap();
    assert_eq!(a.trials, 1);
}

// ---- Quality-aware promote (mission-skill-ab-quality-promote) ----

#[tokio::test]
async fn quality_tips_decision_when_both_variants_have_recent_score() {
    let store = open_store_arc().await;
    // A leads on raw success rate (60% vs 30%) but has a much
    // lower quality score (20 vs 100). With weight=0.3 the
    // weighted formula picks B: 0.21 + 0.30 = 0.51 > 0.42 + 0.06 = 0.48.
    let (run_id, a_id, b_id) = setup_run(&store, 0x10, 0x11, 60, 30).await;
    let qstore = make_qstore(Arc::clone(&store)).await;
    let now = Utc::now();
    qstore.record(a_id, &inputs_for_score(0.20), now).await.unwrap();
    qstore.record(b_id, &inputs_for_score(1.00), now).await.unwrap();
    let (sig, reason) = maybe_complete_with_quality(
        &store,
        &run_id,
        30,
        Some(&qstore),
        QualityWeightConfig::default(),
    )
    .await
    .unwrap();
    assert!(sig.unwrap().significant);
    assert_eq!(reason, Some(WinnerReason::WeightedQuality));
    assert_eq!(get_run(&store, &run_id).await.unwrap().unwrap().winner.as_deref(), Some("v2"));
}

#[tokio::test]
async fn falls_through_to_success_rate_when_only_one_variant_has_quality() {
    let store = open_store_arc().await;
    let (run_id, a_id, _b_id) = setup_run(&store, 0x20, 0x21, 60, 30).await;
    let qstore = make_qstore(Arc::clone(&store)).await;
    // Only A has a quality score.
    qstore.record(a_id, &inputs_for_score(0.20), Utc::now()).await.unwrap();
    let (_, reason) = maybe_complete_with_quality(
        &store,
        &run_id,
        30,
        Some(&qstore),
        QualityWeightConfig::default(),
    )
    .await
    .unwrap();
    assert_eq!(reason, Some(WinnerReason::SuccessRate));
    // A wins on raw success rate even though its quality is lower
    // — the quality data wasn't actionable because B had no score.
    assert_eq!(get_run(&store, &run_id).await.unwrap().unwrap().winner.as_deref(), Some("v1"));
}

#[tokio::test]
async fn falls_through_to_success_rate_when_neither_variant_has_quality() {
    let store = open_store_arc().await;
    let (run_id, _, _) = setup_run(&store, 0x30, 0x31, 60, 30).await;
    let qstore = make_qstore(Arc::clone(&store)).await;
    let (_, reason) = maybe_complete_with_quality(
        &store,
        &run_id,
        30,
        Some(&qstore),
        QualityWeightConfig::default(),
    )
    .await
    .unwrap();
    assert_eq!(reason, Some(WinnerReason::SuccessRate));
    assert_eq!(get_run(&store, &run_id).await.unwrap().unwrap().winner.as_deref(), Some("v1"));
}

#[tokio::test]
async fn stale_quality_score_is_treated_as_missing() {
    let store = open_store_arc().await;
    let (run_id, a_id, b_id) = setup_run(&store, 0x40, 0x41, 60, 30).await;
    let qstore = make_qstore(Arc::clone(&store)).await;
    // Both scores recorded but B's is older than the 7-day cutoff
    // → B treated as missing → fall-through to success rate.
    qstore.record(a_id, &inputs_for_score(0.20), Utc::now()).await.unwrap();
    let stale = Utc::now() - Duration::days(10);
    qstore.record(b_id, &inputs_for_score(1.00), stale).await.unwrap();
    let (_, reason) = maybe_complete_with_quality(
        &store,
        &run_id,
        30,
        Some(&qstore),
        QualityWeightConfig::default(),
    )
    .await
    .unwrap();
    assert_eq!(reason, Some(WinnerReason::SuccessRate));
    assert_eq!(get_run(&store, &run_id).await.unwrap().unwrap().winner.as_deref(), Some("v1"));
}

#[tokio::test]
async fn weight_knob_changes_decision() {
    let store = open_store_arc().await;
    // Same raw shape as the quality_tips test (A=60%, B=30%; A
    // quality=20, B quality=100). With weight=0.0 quality is
    // ignored → A wins on success rate. With weight=0.3 → B wins.
    let (run_id, a_id, b_id) = setup_run(&store, 0x50, 0x51, 60, 30).await;
    let qstore = make_qstore(Arc::clone(&store)).await;
    let now = Utc::now();
    qstore.record(a_id, &inputs_for_score(0.20), now).await.unwrap();
    qstore.record(b_id, &inputs_for_score(1.00), now).await.unwrap();
    let (_, reason) = maybe_complete_with_quality(
        &store,
        &run_id,
        30,
        Some(&qstore),
        QualityWeightConfig {
            weight_quality: 0.0,
            max_score_age_days: 7,
        },
    )
    .await
    .unwrap();
    assert_eq!(reason, Some(WinnerReason::SuccessRate));
    assert_eq!(get_run(&store, &run_id).await.unwrap().unwrap().winner.as_deref(), Some("v1"));
}
