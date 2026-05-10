//! Unit tests for `crate::regression`.

use super::*;
use crate::quality::{QualityInputs, QualityScore};
use async_trait::async_trait;
use chrono::TimeZone;

struct FakeReader(Vec<QualityScore>);

#[async_trait]
impl QualityScoreReader for FakeReader {
    async fn recent_scores_for_skill(
        &self,
        _skill_id: &SkillId,
        _since: DateTime<Utc>,
    ) -> ArgoResult<Vec<QualityScore>> {
        Ok(self.0.clone())
    }
}

fn at(day: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 5, day, 0, 0, 0).unwrap()
}

fn score(skill: SkillId, value: u8, day: u32) -> QualityScore {
    QualityScore {
        skill_id: skill,
        score: value,
        inputs: QualityInputs::default(),
        computed_at: at(day),
    }
}

#[tokio::test]
async fn no_baseline_when_below_min_history() {
    let skill = SkillId::new();
    let reader = FakeReader(vec![score(skill, 90, 1), score(skill, 60, 2)]);
    let cfg = RegressionConfig::default(); // min_history_count = 3
    let decision = detect_regression(skill, &reader, &cfg).await.unwrap();
    assert_eq!(decision, RegressionDecision::NoBaseline);
}

#[tokio::test]
async fn stable_when_no_drop() {
    let skill = SkillId::new();
    let reader = FakeReader(vec![
        score(skill, 80, 1),
        score(skill, 85, 2),
        score(skill, 90, 3),
    ]);
    let cfg = RegressionConfig::default();
    let decision = detect_regression(skill, &reader, &cfg).await.unwrap();
    assert_eq!(
        decision,
        RegressionDecision::Stable {
            current: 90,
            baseline: 85,
        }
    );
}

#[tokio::test]
async fn stable_when_drop_below_thresholds() {
    let skill = SkillId::new();
    // Baseline 80, current 75 → 5-pt drop, ~6% — neither threshold hit.
    let reader = FakeReader(vec![
        score(skill, 80, 1),
        score(skill, 78, 2),
        score(skill, 75, 3),
    ]);
    let cfg = RegressionConfig::default();
    let decision = detect_regression(skill, &reader, &cfg).await.unwrap();
    assert_eq!(
        decision,
        RegressionDecision::Stable {
            current: 75,
            baseline: 80,
        }
    );
}

#[tokio::test]
async fn regressed_when_absolute_drop_dominates() {
    let skill = SkillId::new();
    // Baseline 100, current 70 → 30 pts (≥20) and 30% (≥25). Both hit.
    let reader = FakeReader(vec![
        score(skill, 100, 1),
        score(skill, 100, 2),
        score(skill, 100, 3),
        score(skill, 70, 4),
    ]);
    let cfg = RegressionConfig::default();
    let decision = detect_regression(skill, &reader, &cfg).await.unwrap();
    assert_eq!(
        decision,
        RegressionDecision::Regressed {
            current: 70,
            baseline: 100,
            drop_abs: -30,
            drop_pct: 30,
        }
    );
}

#[tokio::test]
async fn regressed_when_percentage_drop_dominates() {
    let skill = SkillId::new();
    // Baseline 80, current 50 → 30 pts (≥20) and 38% rounded (≥25).
    let reader = FakeReader(vec![
        score(skill, 80, 1),
        score(skill, 78, 2),
        score(skill, 80, 3),
        score(skill, 50, 4),
    ]);
    let cfg = RegressionConfig::default();
    let decision = detect_regression(skill, &reader, &cfg).await.unwrap();
    assert_eq!(
        decision,
        RegressionDecision::Regressed {
            current: 50,
            baseline: 80,
            drop_abs: -30,
            drop_pct: 38, // 30/80 = 0.375 → 38 rounded
        }
    );
}

#[tokio::test]
async fn regressed_when_both_thresholds_clearly_hit() {
    let skill = SkillId::new();
    // Baseline 90, current 30 → 60 pts and 67% — both well above.
    let reader = FakeReader(vec![
        score(skill, 85, 1),
        score(skill, 90, 2),
        score(skill, 88, 3),
        score(skill, 30, 4),
    ]);
    let cfg = RegressionConfig::default();
    let decision = detect_regression(skill, &reader, &cfg).await.unwrap();
    assert_eq!(
        decision,
        RegressionDecision::Regressed {
            current: 30,
            baseline: 90,
            drop_abs: -60,
            drop_pct: 67,
        }
    );
}

#[tokio::test]
async fn baseline_uses_trailing_window_max() {
    let skill = SkillId::new();
    // Window-max is 95 (day 2), NOT the most recent prior 80 (day 3).
    // A naive "last-prior" baseline would compute 80→50 = 38% drop;
    // the window-max correctly anchors on 95 → drop_pct = 47%.
    let reader = FakeReader(vec![
        score(skill, 70, 1),
        score(skill, 95, 2),
        score(skill, 80, 3),
        score(skill, 50, 4),
    ]);
    let cfg = RegressionConfig::default();
    let decision = detect_regression(skill, &reader, &cfg).await.unwrap();
    assert_eq!(
        decision,
        RegressionDecision::Regressed {
            current: 50,
            baseline: 95,
            drop_abs: -45,
            drop_pct: 47, // 45/95 = 0.4736… → 47 rounded
        }
    );
}

#[tokio::test]
async fn respects_min_history_count() {
    let skill = SkillId::new();
    let scores = vec![score(skill, 100, 1), score(skill, 100, 2), score(skill, 50, 3)];
    let reader = FakeReader(scores);

    // Tighter min_history_count (4) suppresses the decision.
    let strict = RegressionConfig {
        min_history_count: 4,
        ..RegressionConfig::default()
    };
    assert_eq!(
        detect_regression(skill, &reader, &strict).await.unwrap(),
        RegressionDecision::NoBaseline,
    );

    // Looser min_history_count (2) lets the same data regress.
    let loose = RegressionConfig {
        min_history_count: 2,
        ..RegressionConfig::default()
    };
    assert_eq!(
        detect_regression(skill, &reader, &loose).await.unwrap(),
        RegressionDecision::Regressed {
            current: 50,
            baseline: 100,
            drop_abs: -50,
            drop_pct: 50,
        },
    );
}
