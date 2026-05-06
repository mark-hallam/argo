//! Unit tests for `ab_test`. Lives in a sibling file so the
//! main module stays under the per-file 500-LOC budget.
use super::{
    create_run, ensure_ab_schema, get_run, maybe_complete, promote, record_outcome,
    select_variant, significance, variant_counts, Variant, VariantCounts,
    CHI_SQUARE_CRIT_DF1_ALPHA_05,
};
use argo_store::SessionStore;

async fn open_store() -> SessionStore {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    ensure_ab_schema(&store).await.unwrap();
    store
}

async fn insert_skill(store: &SessionStore, name: &str, version: &str) {
    let id = uuid::Uuid::now_v7().as_bytes().to_vec();
    sqlx::query(
        "INSERT INTO skill (id, name, version, body, frontmatter, source, hash, learned_in, superseded_by, created_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id)
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
