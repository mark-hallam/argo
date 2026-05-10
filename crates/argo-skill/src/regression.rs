//! Skill quality regression detector (Mission Claim 4 §3 #82).
//!
//! Compares the latest [`QualityScore`] for a skill against its
//! prior baseline (the highest score in the trailing window,
//! excluding the latest sample) and emits a [`RegressionDecision`]
//! when the drop exceeds both an absolute and a percentage
//! threshold.
//!
//! The detector is pure: it returns a [`RegressionDecision`] and
//! emits no side effects. The cron tick that calls it and the
//! demote action it eventually drives live in separate briefs
//! (`mission-skill-quality-cron`, future demoter).

use crate::quality::{QualityScore, SkillQualityStore};
use argo_types::{ArgoError, ArgoResult, SkillId};
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};

/// Tunable regression-detection thresholds.
///
/// The defaults were chosen to surface unambiguous regressions
/// while suppressing noise:
///
/// - `min_score_drop = 20` — a 20-point absolute drop is large
///   enough to escape ordinary scoring jitter regardless of the
///   baseline.
/// - `min_pct_drop = 25` — a 25% relative drop signals a
///   meaningful shift even at higher baselines.
/// - `trailing_window_days = 30` — covers roughly a month of
///   usage; aligns with the quality recompute cadence.
/// - `min_history_count = 3` — require ≥ 3 in-window scores so a
///   brand-new skill (1–2 readings) cannot register as regressed
///   before it has stabilised.
///
/// Operators tune these per call site by constructing
/// `RegressionConfig` directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegressionConfig {
    /// Minimum absolute drop (in score points) required for a
    /// regression. Defaults to `20`.
    pub min_score_drop: u8,
    /// Minimum percentage drop (rounded) required for a regression.
    /// Defaults to `25` (i.e. 25 %).
    pub min_pct_drop: u8,
    /// Length of the trailing window the detector inspects when
    /// computing the baseline. Defaults to `30` days.
    pub trailing_window_days: u32,
    /// Minimum number of in-window scores needed before a
    /// regression decision can fire. Defaults to `3`.
    pub min_history_count: u8,
}

impl Default for RegressionConfig {
    fn default() -> Self {
        Self {
            min_score_drop: 20,
            min_pct_drop: 25,
            trailing_window_days: 30,
            min_history_count: 3,
        }
    }
}

/// Outcome of a regression check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegressionDecision {
    /// Fewer than `min_history_count` scores exist in the trailing
    /// window — the skill has not stabilised enough to evaluate.
    NoBaseline,
    /// Latest score is healthy relative to the baseline.
    Stable {
        /// Most recent score in the window.
        current: u8,
        /// Highest score in the trailing window (excluding `current`).
        baseline: u8,
    },
    /// Latest score has dropped past both thresholds.
    Regressed {
        /// Most recent score in the window.
        current: u8,
        /// Highest score in the trailing window (excluding `current`).
        baseline: u8,
        /// Signed delta `current - baseline` (always negative for
        /// `Regressed`; signed for clarity at call sites).
        drop_abs: i16,
        /// Rounded percentage drop relative to the baseline.
        drop_pct: u8,
    },
}

/// Narrow read surface the detector depends on. Implementors return
/// scores for `skill_id` whose `computed_at` is at or after `since`.
#[async_trait]
pub trait QualityScoreReader: Send + Sync {
    /// Scores for `skill_id` whose `computed_at >= since`, in any
    /// order.
    ///
    /// # Errors
    /// Implementation-specific I/O or decode failures.
    async fn recent_scores_for_skill(
        &self,
        skill_id: &SkillId,
        since: DateTime<Utc>,
    ) -> ArgoResult<Vec<QualityScore>>;
}

#[async_trait]
impl QualityScoreReader for SkillQualityStore {
    async fn recent_scores_for_skill(
        &self,
        skill_id: &SkillId,
        since: DateTime<Utc>,
    ) -> ArgoResult<Vec<QualityScore>> {
        let mut history = self.history(*skill_id).await?;
        history.retain(|s| s.computed_at >= since);
        Ok(history)
    }
}

/// Inspect the recent quality timeline for one skill and decide
/// whether it has regressed.
///
/// Algorithm:
/// 1. Pull all scores in the trailing window from `store`.
/// 2. If fewer than `cfg.min_history_count` rows are returned,
///    yield [`RegressionDecision::NoBaseline`].
/// 3. Pick the latest sample (max `computed_at`) as `current`.
///    The baseline is the maximum score across the remaining rows
///    — using the window-max rather than the second-most-recent
///    means a single recovered score cannot mask a regression
///    from an earlier peak.
/// 4. The decision is [`RegressionDecision::Regressed`] only when
///    the drop exceeds BOTH thresholds (AND-logic). This guards
///    against:
///    - Tiny absolute drops at high baselines (e.g. 100 → 99 is
///      a 1-point, 1 % drop — clearly noise).
///    - Tiny absolute drops at low baselines that look big in
///      percentage terms (e.g. 4 → 3 is 25 % but only 1 point).
///
/// # Errors
/// Propagates errors from the [`QualityScoreReader`].
pub async fn detect_regression(
    skill_id: SkillId,
    store: &dyn QualityScoreReader,
    cfg: &RegressionConfig,
) -> ArgoResult<RegressionDecision> {
    let since = Utc::now() - Duration::days(i64::from(cfg.trailing_window_days));
    let scores = store.recent_scores_for_skill(&skill_id, since).await?;
    if scores.len() < usize::from(cfg.min_history_count) {
        return Ok(RegressionDecision::NoBaseline);
    }
    let latest_idx = scores
        .iter()
        .enumerate()
        .max_by_key(|(_, s)| s.computed_at)
        .map(|(i, _)| i)
        .ok_or(ArgoError::Invariant(
            "regression: empty scores after length guard",
        ))?;
    let current = scores[latest_idx].score;
    let baseline = scores
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != latest_idx)
        .map(|(_, s)| s.score)
        .max()
        .ok_or(ArgoError::Invariant(
            "regression: no prior scores after length guard",
        ))?;
    if baseline <= current {
        return Ok(RegressionDecision::Stable { current, baseline });
    }
    let drop_abs: i16 = i16::from(current) - i16::from(baseline);
    let drop_pct_raw =
        f64::from(baseline - current) * 100.0 / f64::from(baseline);
    let drop_pct = drop_pct_raw.round().clamp(0.0, 100.0) as u8;
    let abs_hit = drop_abs.unsigned_abs() >= u16::from(cfg.min_score_drop);
    let pct_hit = drop_pct >= cfg.min_pct_drop;
    if abs_hit && pct_hit {
        Ok(RegressionDecision::Regressed {
            current,
            baseline,
            drop_abs,
            drop_pct,
        })
    } else {
        Ok(RegressionDecision::Stable { current, baseline })
    }
}

#[cfg(test)]
#[path = "regression_tests.rs"]
mod tests;
