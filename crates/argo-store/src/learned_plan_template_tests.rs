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
    promote(&pool, &id, &NoopSkillIngestSink).await.expect("promote");
    let row = get(&pool, &id).await.unwrap().unwrap();
    assert_eq!(row.trust_grade, TrustGrade::Promoted);
    assert!(row.promoted_at.is_some());
}

#[tokio::test]
async fn reject_marks_row_without_promoted_at() {
    let pool = fresh_pool().await;
    let id = insert(&pool, &fixture("t", "g")).await.unwrap();
    reject(&pool, &id, &NoopSkillIngestSink).await.expect("reject");
    let row = get(&pool, &id).await.unwrap().unwrap();
    assert_eq!(row.trust_grade, TrustGrade::Rejected);
    assert!(row.promoted_at.is_none());
}

#[tokio::test]
async fn promote_unknown_id_errors() {
    let pool = fresh_pool().await;
    ensure_schema(&pool).await.unwrap();
    let err = promote(&pool, "no-such-id", &NoopSkillIngestSink)
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
    promote(&pool, &id1, &NoopSkillIngestSink).await.unwrap();
    reject(&pool, &id3, &NoopSkillIngestSink).await.unwrap();

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
    promote(&pool, &a, &NoopSkillIngestSink).await.unwrap();
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

/// Capturing sink used by the sink-wiring tests. Records every
/// `ingest_from_template` / `demote_from_template` call so the
/// tests can assert that `promote` / `reject` forward to the sink
/// (and only after the row has flipped to its terminal grade).
#[derive(Debug, Default)]
struct CapturingSink {
    ingested: std::sync::Mutex<Vec<LearnedPlanTemplateRow>>,
    demoted: std::sync::Mutex<Vec<LearnedPlanTemplateRow>>,
}

#[async_trait::async_trait]
impl SkillIngestSink for CapturingSink {
    async fn ingest_from_template(&self, row: &LearnedPlanTemplateRow) -> ArgoResult<()> {
        self.ingested.lock().unwrap().push(row.clone());
        Ok(())
    }
    async fn demote_from_template(&self, row: &LearnedPlanTemplateRow) -> ArgoResult<()> {
        self.demoted.lock().unwrap().push(row.clone());
        Ok(())
    }
}

#[tokio::test]
async fn promote_invokes_sink_with_post_flip_row() {
    let pool = fresh_pool().await;
    let id = insert(&pool, &fixture("t", "g")).await.unwrap();
    let sink = CapturingSink::default();
    promote(&pool, &id, &sink).await.expect("promote");
    let ingested = sink.ingested.lock().unwrap();
    assert_eq!(ingested.len(), 1, "sink should see exactly one ingest");
    assert_eq!(ingested[0].id, id);
    assert_eq!(ingested[0].trust_grade, TrustGrade::Promoted);
    assert!(
        ingested[0].promoted_at.is_some(),
        "row passed to sink should already carry promoted_at"
    );
    assert!(sink.demoted.lock().unwrap().is_empty());
}

#[tokio::test]
async fn reject_invokes_demote_with_rejected_row() {
    let pool = fresh_pool().await;
    let id = insert(&pool, &fixture("t", "g")).await.unwrap();
    let sink = CapturingSink::default();
    reject(&pool, &id, &sink).await.expect("reject");
    let demoted = sink.demoted.lock().unwrap();
    assert_eq!(demoted.len(), 1, "sink should see exactly one demote");
    assert_eq!(demoted[0].id, id);
    assert_eq!(demoted[0].trust_grade, TrustGrade::Rejected);
    assert!(sink.ingested.lock().unwrap().is_empty());
}

#[tokio::test]
async fn sink_error_propagates_after_grade_flip() {
    use std::sync::atomic::{AtomicBool, Ordering};

    struct FailingSink {
        called: AtomicBool,
    }
    #[async_trait::async_trait]
    impl SkillIngestSink for FailingSink {
        async fn ingest_from_template(&self, _row: &LearnedPlanTemplateRow) -> ArgoResult<()> {
            self.called.store(true, Ordering::SeqCst);
            Err(ArgoError::Provider("sink boom".into()))
        }
        async fn demote_from_template(&self, _row: &LearnedPlanTemplateRow) -> ArgoResult<()> {
            Ok(())
        }
    }

    let pool = fresh_pool().await;
    let id = insert(&pool, &fixture("t", "g")).await.unwrap();
    let sink = FailingSink {
        called: AtomicBool::new(false),
    };
    let err = promote(&pool, &id, &sink).await.expect_err("sink failure");
    assert!(format!("{err}").contains("sink boom"));
    assert!(sink.called.load(Ordering::SeqCst));
    // Even though the sink failed, the lifecycle column should
    // have flipped — sink errors don't roll back the grade write
    // (caller can retry the sink via re-promote).
    let row = get(&pool, &id).await.unwrap().unwrap();
    assert_eq!(row.trust_grade, TrustGrade::Promoted);
}
