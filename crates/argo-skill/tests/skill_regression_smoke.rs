//! Integration smoke for `detect_regression` end-to-end against
//! a real `SessionStore`-backed `skill_quality_score` table
//! (Mission Claim 4 §3 #82).
//!
//! Seeds 3 perfect baselines (score 100) plus 1 recent low score
//! (score 40), then asserts `detect_regression` returns
//! `Regressed` with the expected delta.

use argo_skill::{
    detect_regression,
    quality::{QualityInputs, SkillQualityStore},
    RegressionConfig, RegressionDecision,
};
use argo_store::SessionStore;
use argo_types::SkillId;
use chrono::{Duration, Utc};
use std::sync::Arc;

#[tokio::test]
async fn detects_regression_over_seeded_timeline() {
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    let qstore = SkillQualityStore::new(Arc::clone(&store));
    qstore.ensure_schema().await.unwrap();

    let skill_id = SkillId::new();
    let now = Utc::now();

    let perfect = QualityInputs::default();
    qstore
        .record(skill_id, &perfect, now - Duration::days(20))
        .await
        .unwrap();
    qstore
        .record(skill_id, &perfect, now - Duration::days(15))
        .await
        .unwrap();
    qstore
        .record(skill_id, &perfect, now - Duration::days(10))
        .await
        .unwrap();

    // success_rate 0.4 → base 40, no other penalties → score 40.
    let bad = QualityInputs {
        success_rate: 0.4,
        ..QualityInputs::default()
    };
    qstore
        .record(skill_id, &bad, now - Duration::days(1))
        .await
        .unwrap();

    let cfg = RegressionConfig::default();
    let decision = detect_regression(skill_id, &qstore, &cfg).await.unwrap();
    match decision {
        RegressionDecision::Regressed {
            current,
            baseline,
            drop_abs,
            drop_pct,
        } => {
            assert_eq!(current, 40);
            assert_eq!(baseline, 100);
            assert_eq!(drop_abs, -60);
            assert_eq!(drop_pct, 60);
        }
        other => panic!("expected Regressed, got {other:?}"),
    }
}
