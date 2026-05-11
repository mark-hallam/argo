//! Unit tests for [`super::skill_rating`] — fresh in-memory pool per
//! test, exercising the three storage surfaces plus idempotency
//! semantics, empty-aggregate behavior, and the comment-length
//! truncation boundary.

use super::*;
use crate::SessionStore;
use sqlx::{Pool, Sqlite};

async fn fresh_pool() -> Pool<Sqlite> {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    store.pool().clone()
}

#[tokio::test]
async fn record_rating_round_trip_up_vote() {
    let pool = fresh_pool().await;
    let skill = SkillId::new();
    let rater = TenantId::new();
    record_rating(&pool, skill, "0.1.0", "vendor", rater, SkillVote::Up, None)
        .await
        .unwrap();
    let (up, down, score) = aggregate_ratings(&pool, skill, "0.1.0").await.unwrap();
    assert_eq!(up, 1);
    assert_eq!(down, 0);
    assert!((score - 1.0).abs() < f32::EPSILON);
}

#[tokio::test]
async fn record_rating_round_trip_down_vote() {
    let pool = fresh_pool().await;
    let skill = SkillId::new();
    let rater = TenantId::new();
    record_rating(
        &pool,
        skill,
        "0.1.0",
        "vendor",
        rater,
        SkillVote::Down,
        Some("flaky"),
    )
    .await
    .unwrap();
    let (up, down, score) = aggregate_ratings(&pool, skill, "0.1.0").await.unwrap();
    assert_eq!(up, 0);
    assert_eq!(down, 1);
    assert!(score.abs() < f32::EPSILON);
}

#[tokio::test]
async fn record_rating_idempotent_on_primary_key() {
    let pool = fresh_pool().await;
    let skill = SkillId::new();
    let rater = TenantId::new();
    // First vote: up.
    record_rating(&pool, skill, "0.1.0", "vendor", rater, SkillVote::Up, None)
        .await
        .unwrap();
    // Same rater re-votes: now down. Should REPLACE not duplicate.
    record_rating(
        &pool,
        skill,
        "0.1.0",
        "vendor",
        rater,
        SkillVote::Down,
        Some("changed mind"),
    )
    .await
    .unwrap();
    let (up, down, _score) = aggregate_ratings(&pool, skill, "0.1.0").await.unwrap();
    assert_eq!(up, 0, "old up-vote should have been replaced");
    assert_eq!(down, 1, "new down-vote should be the only row");
    let rows = recent_ratings(&pool, skill, "0.1.0", 10).await.unwrap();
    assert_eq!(rows.len(), 1, "PK uniqueness preserves single row");
    assert_eq!(rows[0].vote, SkillVote::Down);
    assert_eq!(rows[0].comment.as_deref(), Some("changed mind"));
}

#[tokio::test]
async fn aggregate_ratings_returns_zeros_on_empty() {
    let pool = fresh_pool().await;
    let skill = SkillId::new();
    let (up, down, score) = aggregate_ratings(&pool, skill, "0.1.0").await.unwrap();
    assert_eq!(up, 0);
    assert_eq!(down, 0);
    assert_eq!(score, 0.0, "no divide-by-zero on empty aggregate");
}

#[tokio::test]
async fn aggregate_ratings_mixed_up_and_down() {
    let pool = fresh_pool().await;
    let skill = SkillId::new();
    let raters: Vec<TenantId> = (0..5).map(|_| TenantId::new()).collect();
    record_rating(
        &pool,
        skill,
        "1.0.0",
        "vendor",
        raters[0],
        SkillVote::Up,
        None,
    )
    .await
    .unwrap();
    record_rating(
        &pool,
        skill,
        "1.0.0",
        "vendor",
        raters[1],
        SkillVote::Up,
        None,
    )
    .await
    .unwrap();
    record_rating(
        &pool,
        skill,
        "1.0.0",
        "vendor",
        raters[2],
        SkillVote::Up,
        None,
    )
    .await
    .unwrap();
    record_rating(
        &pool,
        skill,
        "1.0.0",
        "vendor",
        raters[3],
        SkillVote::Down,
        None,
    )
    .await
    .unwrap();
    record_rating(
        &pool,
        skill,
        "1.0.0",
        "vendor",
        raters[4],
        SkillVote::Down,
        None,
    )
    .await
    .unwrap();
    let (up, down, score) = aggregate_ratings(&pool, skill, "1.0.0").await.unwrap();
    assert_eq!(up, 3);
    assert_eq!(down, 2);
    assert!(
        (score - 0.6).abs() < 1e-5,
        "expected 3/5 = 0.6, got {score}"
    );
}

#[tokio::test]
async fn list_for_publisher_filters_and_groups() {
    let pool = fresh_pool().await;
    let skill_a = SkillId::new();
    let skill_b = SkillId::new();
    let skill_c = SkillId::new();
    // Two skills under publisher 'alpha', one under 'beta'.
    record_rating(
        &pool,
        skill_a,
        "1.0.0",
        "alpha",
        TenantId::new(),
        SkillVote::Up,
        None,
    )
    .await
    .unwrap();
    record_rating(
        &pool,
        skill_a,
        "1.0.0",
        "alpha",
        TenantId::new(),
        SkillVote::Up,
        None,
    )
    .await
    .unwrap();
    record_rating(
        &pool,
        skill_b,
        "0.2.0",
        "alpha",
        TenantId::new(),
        SkillVote::Down,
        None,
    )
    .await
    .unwrap();
    record_rating(
        &pool,
        skill_c,
        "0.1.0",
        "beta",
        TenantId::new(),
        SkillVote::Up,
        None,
    )
    .await
    .unwrap();

    let alpha = list_for_publisher(&pool, "alpha").await.unwrap();
    assert_eq!(alpha.len(), 2, "expected 2 distinct skills under 'alpha'");
    // Highest-volume first by ORDER BY: skill_a has 2 votes, skill_b has 1.
    assert_eq!(alpha[0].up_count + alpha[0].down_count, 2);
    assert_eq!(alpha[1].up_count + alpha[1].down_count, 1);
    assert!(alpha.iter().all(|s| s.publisher_tenant == "alpha"));

    let beta = list_for_publisher(&pool, "beta").await.unwrap();
    assert_eq!(beta.len(), 1);
    assert_eq!(beta[0].publisher_tenant, "beta");

    let missing = list_for_publisher(&pool, "gamma").await.unwrap();
    assert!(missing.is_empty());
}

#[tokio::test]
async fn comment_truncated_at_max_len() {
    let pool = fresh_pool().await;
    let skill = SkillId::new();
    let rater = TenantId::new();
    let long_comment: String = "x".repeat(MAX_COMMENT_LEN + 100);
    record_rating(
        &pool,
        skill,
        "1.0.0",
        "vendor",
        rater,
        SkillVote::Up,
        Some(&long_comment),
    )
    .await
    .unwrap();
    let rows = recent_ratings(&pool, skill, "1.0.0", 1).await.unwrap();
    let stored = rows[0].comment.as_deref().unwrap();
    assert_eq!(stored.chars().count(), MAX_COMMENT_LEN);
}

#[tokio::test]
async fn record_rating_rejects_empty_publisher() {
    let pool = fresh_pool().await;
    let skill = SkillId::new();
    let rater = TenantId::new();
    let err = record_rating(&pool, skill, "1.0.0", "", rater, SkillVote::Up, None)
        .await
        .unwrap_err();
    assert!(
        err.to_string().contains("publisher_tenant"),
        "expected publisher_tenant guard, got: {err}"
    );
}
