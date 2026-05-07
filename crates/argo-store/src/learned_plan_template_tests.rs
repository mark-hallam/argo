use super::*;

async fn fresh_pool() -> SqlitePool {
    SqlitePool::connect("sqlite::memory:")
        .await
        .expect("open in-memory pool")
}

fn fixture<'a>(name: &'a str, parent_goal: &'a str) -> NewLearnedPlanTemplate<'a> {
    NewLearnedPlanTemplate {
        name,
        description: "distilled from cohort",
        parent_goal,
        keywords_csv: "promote,launch,announce",
        yaml: "id: shipped-marketing-launch\ntasks:\n  - id: draft\n    worker: \"\"",
        cohort_size: 3,
    }
}

#[tokio::test]
async fn schema_applies_idempotently() {
    let pool = fresh_pool().await;
    ensure_schema(&pool).await.expect("first apply");
    ensure_schema(&pool).await.expect("second apply");
}

#[tokio::test]
async fn insert_and_get_round_trip() {
    let pool = fresh_pool().await;
    let id = insert(&pool, &fixture("marketing-launch-distilled", "promote our blog post"))
        .await
        .expect("insert");
    let row = get(&pool, &id).await.expect("get").expect("row exists");
    assert_eq!(row.name, "marketing-launch-distilled");
    assert_eq!(row.parent_goal, "promote our blog post");
    assert_eq!(row.keywords_csv, "promote,launch,announce");
    assert_eq!(row.cohort_size, 3);
    assert_eq!(row.trust_grade, TrustGrade::Pending);
    assert!(row.promoted_at.is_none());
}

#[tokio::test]
async fn keywords_round_trip_lowercased() {
    let pool = fresh_pool().await;
    let id = insert(&pool, &fixture("t", "g")).await.unwrap();
    let row = get(&pool, &id).await.unwrap().unwrap();
    assert_eq!(row.keywords(), vec!["promote", "launch", "announce"]);
}

#[tokio::test]
async fn promote_marks_row_and_stamps_timestamp() {
    let pool = fresh_pool().await;
    let id = insert(&pool, &fixture("t", "g")).await.unwrap();
    promote(&pool, &id).await.expect("promote");
    let row = get(&pool, &id).await.unwrap().unwrap();
    assert_eq!(row.trust_grade, TrustGrade::Promoted);
    assert!(row.promoted_at.is_some());
}

#[tokio::test]
async fn reject_marks_row_without_promoted_at() {
    let pool = fresh_pool().await;
    let id = insert(&pool, &fixture("t", "g")).await.unwrap();
    reject(&pool, &id).await.expect("reject");
    let row = get(&pool, &id).await.unwrap().unwrap();
    assert_eq!(row.trust_grade, TrustGrade::Rejected);
    assert!(row.promoted_at.is_none());
}

#[tokio::test]
async fn promote_unknown_id_errors() {
    let pool = fresh_pool().await;
    ensure_schema(&pool).await.unwrap();
    let err = promote(&pool, "no-such-id")
        .await
        .expect_err("expected error");
    assert!(format!("{err}").contains("no row"));
}

#[tokio::test]
async fn list_filters_by_trust_grade() {
    let pool = fresh_pool().await;
    let id1 = insert(&pool, &fixture("a", "ga")).await.unwrap();
    let _id2 = insert(&pool, &fixture("b", "gb")).await.unwrap();
    let id3 = insert(&pool, &fixture("c", "gc")).await.unwrap();
    promote(&pool, &id1).await.unwrap();
    reject(&pool, &id3).await.unwrap();

    let pending = list(
        &pool,
        &LearnedPlanTemplateFilters {
            trust_grade: Some(TrustGrade::Pending),
        },
        10,
    )
    .await
    .unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].name, "b");

    let promoted = list(
        &pool,
        &LearnedPlanTemplateFilters {
            trust_grade: Some(TrustGrade::Promoted),
        },
        10,
    )
    .await
    .unwrap();
    assert_eq!(promoted.len(), 1);
    assert_eq!(promoted[0].name, "a");

    let all = list(&pool, &LearnedPlanTemplateFilters::default(), 10)
        .await
        .unwrap();
    assert_eq!(all.len(), 3);
}

#[tokio::test]
async fn list_promoted_helper_returns_only_promoted() {
    let pool = fresh_pool().await;
    let a = insert(&pool, &fixture("a", "ga")).await.unwrap();
    let _b = insert(&pool, &fixture("b", "gb")).await.unwrap();
    promote(&pool, &a).await.unwrap();
    let promoted = list_promoted(&pool).await.unwrap();
    assert_eq!(promoted.len(), 1);
    assert_eq!(promoted[0].id, a);
}

#[tokio::test]
async fn trust_grade_string_round_trip() {
    assert_eq!(TrustGrade::Pending.as_str(), "pending");
    assert_eq!(TrustGrade::Promoted.as_str(), "promoted");
    assert_eq!(TrustGrade::Rejected.as_str(), "rejected");
    assert_eq!(TrustGrade::from_str_lossy("pending"), TrustGrade::Pending);
    assert_eq!(TrustGrade::from_str_lossy("promoted"), TrustGrade::Promoted);
    assert_eq!(TrustGrade::from_str_lossy("rejected"), TrustGrade::Rejected);
    // Unknown / forward-rolled values fall back to Pending.
    assert_eq!(TrustGrade::from_str_lossy("future-grade"), TrustGrade::Pending);
}
