//! Quality-aware completion helpers for skill A/B tests.

use super::{get_run, significance, variant_counts, AbTestRun, Significance, VariantCounts};
use crate::quality::SkillQualityStore;
use crate::version::find_version;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, SkillId, TenantId};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Knobs that control whether [`maybe_complete_with_quality`] folds
/// the latest cached `QualityScore` into the winner-selection
/// formula. Defaults are conservative: success rate stays primary
/// (70%) and quality contributes the remaining 30%, with a 7-day
/// freshness cutoff so a stale quality snapshot can't bias the
/// pick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QualityWeightConfig {
    /// Fraction (0.0..=1.0) of the weighted score contributed by
    /// `quality_score / 100`. The remaining `1.0 - weight_quality`
    /// is success rate. `0.0` reverts to today's success-rate-only
    /// behavior.
    pub weight_quality: f32,
    /// Quality scores older than this are treated as missing — the
    /// variant falls through to success-rate-only logic. Prevents a
    /// stale snapshot from steering the pick.
    pub max_score_age_days: u32,
}

impl Default for QualityWeightConfig {
    fn default() -> Self {
        Self {
            weight_quality: 0.3,
            max_score_age_days: 7,
        }
    }
}

/// Why the winner was chosen — surfaced so `argo skill ab-promote`
/// can print "won by success rate" vs "won by weighted-quality
/// decision".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WinnerReason {
    /// Picked by raw success rate (today's logic). Used when
    /// quality weighting is disabled, no quality store was passed,
    /// or one/both variants lack a recent quality score.
    SuccessRate,
    /// Picked by `success_rate * (1 - weight) + (quality / 100) *
    /// weight`. Used only when both variants have a quality score
    /// recorded within `max_score_age_days`.
    WeightedQuality,
}

/// Like [`super::maybe_complete`] but also factors `QualityScore` into the
/// winner pick. When BOTH variants have a quality score recorded
/// within `config.max_score_age_days`, the winner is chosen by
/// `success_rate * (1 - weight_quality) + (quality_score / 100) *
/// weight_quality` per variant. Otherwise (quality store absent,
/// `weight_quality <= 0.0`, or one/both variants missing a recent
/// score) it falls back to today's success-rate-only logic.
///
/// Returns `(significance, winner_reason)`. `winner_reason` is
/// `Some` only when the run flipped to `completed` on this call.
pub async fn maybe_complete_with_quality(
    store: &SessionStore,
    tenant_id: &TenantId,
    run_id: &str,
    min_samples_per_variant: i64,
    quality_store: Option<&SkillQualityStore>,
    config: QualityWeightConfig,
) -> ArgoResult<(Option<Significance>, Option<WinnerReason>)> {
    let Some(run) = get_run(store, tenant_id, run_id).await? else {
        return Ok((None, None));
    };
    if run.status != "running" {
        return Ok((None, None));
    }
    let (a, b) = variant_counts(store, run_id).await?;
    let sig = significance(a, b, min_samples_per_variant);
    if !sig.significant {
        return Ok((Some(sig), None));
    }
    let rate_a = success_rate(a);
    let rate_b = success_rate(b);
    let (winner, reason) = pick_winner(store, &run, rate_a, rate_b, quality_store, config).await?;
    let now = Utc::now().timestamp_micros();
    sqlx::query(
        "UPDATE skill_ab_test_run SET status = 'completed', \
         completed_at = ?, winner = ? WHERE id = ? AND tenant_id = ?",
    )
    .bind(now)
    .bind(&winner)
    .bind(run_id)
    .bind(tenant_id.to_string())
    .execute(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("complete ab run: {e}")))?;
    Ok((Some(sig), Some(reason)))
}

fn success_rate(counts: VariantCounts) -> f64 {
    if counts.trials == 0 {
        0.0
    } else {
        counts.successes as f64 / counts.trials as f64
    }
}

async fn pick_winner(
    store: &SessionStore,
    run: &AbTestRun,
    rate_a: f64,
    rate_b: f64,
    quality_store: Option<&SkillQualityStore>,
    config: QualityWeightConfig,
) -> ArgoResult<(String, WinnerReason)> {
    let weight = f64::from(config.weight_quality.clamp(0.0, 1.0));
    if weight <= 0.0 {
        return Ok((
            success_winner(run, rate_a, rate_b),
            WinnerReason::SuccessRate,
        ));
    }
    let Some(qs) = quality_store else {
        return Ok((
            success_winner(run, rate_a, rate_b),
            WinnerReason::SuccessRate,
        ));
    };
    let q_a = recent_quality(
        store,
        qs,
        &run.skill_name,
        &run.variant_a,
        config.max_score_age_days,
    )
    .await?;
    let q_b = recent_quality(
        store,
        qs,
        &run.skill_name,
        &run.variant_b,
        config.max_score_age_days,
    )
    .await?;
    match (q_a, q_b) {
        (Some(qa), Some(qb)) => {
            let score_a = rate_a * (1.0 - weight) + (f64::from(qa) / 100.0) * weight;
            let score_b = rate_b * (1.0 - weight) + (f64::from(qb) / 100.0) * weight;
            let winner = if score_a >= score_b {
                run.variant_a.clone()
            } else {
                run.variant_b.clone()
            };
            Ok((winner, WinnerReason::WeightedQuality))
        }
        _ => Ok((
            success_winner(run, rate_a, rate_b),
            WinnerReason::SuccessRate,
        )),
    }
}

fn success_winner(run: &AbTestRun, rate_a: f64, rate_b: f64) -> String {
    if rate_a >= rate_b {
        run.variant_a.clone()
    } else {
        run.variant_b.clone()
    }
}

async fn recent_quality(
    store: &SessionStore,
    quality_store: &SkillQualityStore,
    skill_name: &str,
    version: &str,
    max_age_days: u32,
) -> ArgoResult<Option<u8>> {
    let Some(id_hex) = find_version(store, skill_name, version).await? else {
        return Ok(None);
    };
    let Some(skill_id) = skill_id_from_hex(&id_hex) else {
        return Ok(None);
    };
    let Some(latest) = quality_store.latest(skill_id).await? else {
        return Ok(None);
    };
    let age_days = Utc::now()
        .signed_duration_since(latest.computed_at)
        .num_days();
    if age_days > i64::from(max_age_days) {
        return Ok(None);
    }
    Ok(Some(latest.score))
}

fn skill_id_from_hex(hex: &str) -> Option<SkillId> {
    if hex.len() != 32 {
        return None;
    }
    let mut bytes = [0u8; 16];
    for (i, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(hex.get(i * 2..i * 2 + 2)?, 16).ok()?;
    }
    Some(SkillId(Uuid::from_bytes(bytes)))
}

#[cfg(test)]
#[path = "ab_test_quality_tests.rs"]
mod ab_test_quality_tests;
