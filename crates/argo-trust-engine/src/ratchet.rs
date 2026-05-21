//! Score → grade mapping with hysteresis.
//!
//! There are two threshold tables: one for "promote up from the
//! current grade" (the harder line to cross) and one for "demote
//! down from the current grade" (the lower line). The gap between
//! them is the hysteresis band — a score oscillating just under the
//! promote line never demotes the grade it earned, and a score that
//! crests promotes only when it's CONFIDENTLY above.
//!
//! The engine ratchets at most one rung per call so the audit chain
//! shows the journey rather than a mystery jump.

use argo_skill::TrustGrade;
use serde::{Deserialize, Serialize};

/// `(target_grade, score_required_to_promote_into_it)`.
const PROMOTE_THRESHOLDS: [(TrustGrade, f32); 4] = [
    (TrustGrade::PublishedCommunity, 2.0),
    (TrustGrade::Tenant, 5.0),
    (TrustGrade::PublishedVerified, 10.0),
    (TrustGrade::Builtin, 20.0),
];

/// `(current_grade, score_below_which_to_demote)`.
const DEMOTE_THRESHOLDS: [(TrustGrade, f32); 4] = [
    (TrustGrade::PublishedCommunity, 1.0),
    (TrustGrade::Tenant, 3.5),
    (TrustGrade::PublishedVerified, 8.0),
    (TrustGrade::Builtin, 17.0),
];

/// Outcome of running [`ratchet_grade`] for one worker.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RatchetDecision {
    /// Score is within tolerance for the current grade — no change.
    Hold,
    /// Move up one rung.
    Promote {
        /// Grade the worker was at before this tick.
        from: TrustGrade,
        /// Grade the worker advances into.
        to: TrustGrade,
    },
    /// Move down one rung.
    Demote {
        /// Grade the worker was at before this tick.
        from: TrustGrade,
        /// Grade the worker drops into.
        to: TrustGrade,
    },
}

/// Given the current grade and the freshly-computed score, return
/// the single-step ratchet decision. Always at most one rung per
/// call.
///
/// ```
/// use argo_trust_engine::{ratchet_grade, RatchetDecision};
/// use argo_skill::TrustGrade;
///
/// // Score just above the published-community promote line (2.0).
/// let decision = ratchet_grade(TrustGrade::LearnedUnreviewed, 2.5);
/// assert!(matches!(decision, RatchetDecision::Promote { .. }));
///
/// // Score well above grade-2 → still only one rung per call.
/// let big_jump = ratchet_grade(TrustGrade::LearnedUnreviewed, 50.0);
/// assert_eq!(
///     big_jump,
///     RatchetDecision::Promote {
///         from: TrustGrade::LearnedUnreviewed,
///         to: TrustGrade::PublishedCommunity,
///     }
/// );
/// ```
#[must_use]
pub fn ratchet_grade(current: TrustGrade, score: f32) -> RatchetDecision {
    let current_u8 = grade_to_u8(current);

    if let Some(next) = u8_to_grade(current_u8.saturating_add(1)) {
        if let Some((_, threshold)) = PROMOTE_THRESHOLDS.iter().find(|(g, _)| *g == next) {
            if score >= *threshold {
                return RatchetDecision::Promote { from: current, to: next };
            }
        }
    }

    if let Some((_, demote_threshold)) =
        DEMOTE_THRESHOLDS.iter().find(|(g, _)| *g == current)
    {
        if score < *demote_threshold && current_u8 > 0 {
            if let Some(prev) = u8_to_grade(current_u8 - 1) {
                return RatchetDecision::Demote { from: current, to: prev };
            }
        }
    }
    RatchetDecision::Hold
}

/// Map [`TrustGrade`] to the inverted u8 the `worker.trust_grade`
/// column uses. Mirrors
/// `routes_worker_trust_grade::grade_for_u8` (the inverse direction)
/// so reads and writes agree.
#[must_use]
pub fn grade_to_u8(g: TrustGrade) -> u8 {
    match g {
        TrustGrade::LearnedUnreviewed => 0,
        TrustGrade::PublishedCommunity => 1,
        TrustGrade::Tenant => 2,
        TrustGrade::PublishedVerified => 3,
        TrustGrade::Builtin => 4,
    }
}

/// Inverse of [`grade_to_u8`]. Returns `None` for u8s above the top
/// rung so the caller can clamp.
#[must_use]
pub fn u8_to_grade(u: u8) -> Option<TrustGrade> {
    Some(match u {
        0 => TrustGrade::LearnedUnreviewed,
        1 => TrustGrade::PublishedCommunity,
        2 => TrustGrade::Tenant,
        3 => TrustGrade::PublishedVerified,
        4 => TrustGrade::Builtin,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn u8_round_trip_every_rung() {
        for u in 0_u8..=4 {
            let grade = u8_to_grade(u).expect("known rung");
            assert_eq!(grade_to_u8(grade), u, "round-trip for u={u}");
        }
    }

    #[test]
    fn u8_above_top_rung_returns_none() {
        assert_eq!(u8_to_grade(5), None);
        assert_eq!(u8_to_grade(255), None);
    }

    #[test]
    fn hold_at_default_grade_with_zero_score() {
        let d = ratchet_grade(TrustGrade::LearnedUnreviewed, 0.0);
        assert_eq!(d, RatchetDecision::Hold);
    }

    #[test]
    fn does_not_demote_below_default_grade() {
        let d = ratchet_grade(TrustGrade::LearnedUnreviewed, -5.0);
        assert_eq!(d, RatchetDecision::Hold);
    }

    #[test]
    fn does_not_promote_above_top_grade() {
        let d = ratchet_grade(TrustGrade::Builtin, 1000.0);
        assert_eq!(d, RatchetDecision::Hold);
    }

    #[test]
    fn promote_at_most_one_rung_per_call_even_with_huge_score() {
        let d = ratchet_grade(TrustGrade::LearnedUnreviewed, 50.0);
        assert_eq!(
            d,
            RatchetDecision::Promote {
                from: TrustGrade::LearnedUnreviewed,
                to: TrustGrade::PublishedCommunity,
            }
        );
    }

    #[test]
    fn promote_at_exact_threshold() {
        let d = ratchet_grade(TrustGrade::LearnedUnreviewed, 2.0);
        assert_eq!(
            d,
            RatchetDecision::Promote {
                from: TrustGrade::LearnedUnreviewed,
                to: TrustGrade::PublishedCommunity,
            }
        );
    }

    #[test]
    fn hold_just_below_promote_threshold() {
        let d = ratchet_grade(TrustGrade::LearnedUnreviewed, 1.99);
        assert_eq!(d, RatchetDecision::Hold);
    }

    #[test]
    fn demote_on_low_score_at_higher_grade() {
        let d = ratchet_grade(TrustGrade::Tenant, 2.0);
        assert_eq!(
            d,
            RatchetDecision::Demote {
                from: TrustGrade::Tenant,
                to: TrustGrade::PublishedCommunity,
            }
        );
    }

    #[test]
    fn hold_above_demote_threshold() {
        let d = ratchet_grade(TrustGrade::Tenant, 4.0);
        assert_eq!(d, RatchetDecision::Hold);
    }

    #[test]
    fn hysteresis_prevents_flap_around_promote_line() {
        // Once at PublishedCommunity, score oscillating 1.0..=1.99
        // — between the demote_below=1.0 and promote_to_tenant=5.0
        // lines — must stay at PublishedCommunity.
        for score_x100 in 100..=199 {
            let score = score_x100 as f32 / 100.0;
            let d = ratchet_grade(TrustGrade::PublishedCommunity, score);
            assert_eq!(
                d,
                RatchetDecision::Hold,
                "flap at score={score} from PublishedCommunity"
            );
        }
    }

    #[test]
    fn demote_at_score_strictly_below_threshold() {
        let d = ratchet_grade(TrustGrade::PublishedCommunity, 0.5);
        assert_eq!(
            d,
            RatchetDecision::Demote {
                from: TrustGrade::PublishedCommunity,
                to: TrustGrade::LearnedUnreviewed,
            }
        );
    }

    #[test]
    fn promote_thresholds_table_matches_table_in_docs() {
        // The 5-grade ladder is part of the public contract; if
        // this test fails the ScoreBreakdown weights also need
        // re-tuning, so the assertions act as a tripwire.
        assert_eq!(PROMOTE_THRESHOLDS.len(), 4);
        assert_eq!(DEMOTE_THRESHOLDS.len(), 4);
        for (i, (_, promote)) in PROMOTE_THRESHOLDS.iter().enumerate() {
            let (_, demote) = DEMOTE_THRESHOLDS[i];
            assert!(
                *promote > demote,
                "promote ({promote}) must be > demote ({demote}) for rung {i}"
            );
        }
    }
}
