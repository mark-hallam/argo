//! Tests for the self-improvement loop closure (cluster→cohort bridge +
//! promotion-on-performance decision).

use super::*;
use crate::ab_eval::{SkillAbEvalReport, VariantEvalResult};
use crate::trace_cluster::{cluster_traces, TraceFingerprint};

// ---- cluster_to_cohort bridge --------------------------------------

fn member(id: &str, vertical: &str, action: &str, seq: &[&str], success: f64) -> TraceMember {
    TraceMember {
        id: id.into(),
        vertical_id: vertical.into(),
        action_type: action.into(),
        trajectory: seq.iter().map(|s| (*s).to_string()).collect(),
        final_text: None,
        success_metric: success,
    }
}

fn fp(id: &str, seq: &[&str], success: f64) -> TraceFingerprint {
    TraceFingerprint::from_parts(id, seq.iter().map(|s| (*s).to_string()).collect(), success)
}

#[test]
fn bridges_a_cross_key_cluster_into_a_cohort() {
    // Three traces with the SAME shape but DIFFERENT (vertical, action)
    // pairs — they cluster together in trace_cluster (cross-key) but
    // would never share a cohort key in group_into_cohorts.
    let members = vec![
        member("t1", "support", "resolve", &["lookup", "update", "confirm"], 0.9),
        member("t2", "gtm", "follow_up", &["lookup", "update", "confirm"], 0.85),
        member("t3", "billing", "reconcile", &["lookup", "update", "confirm"], 0.95),
    ];
    let fps: Vec<TraceFingerprint> = members
        .iter()
        .map(|m| fp(&m.id, &m.trajectory.iter().map(String::as_str).collect::<Vec<_>>(), m.success_metric))
        .collect();
    let clusters = cluster_traces(&fps, 0.6);
    assert_eq!(clusters.len(), 1, "same shape → one cross-key cluster");

    let cohort = cluster_to_cohort(&clusters[0], &members, "support", "resolve")
        .expect("bridge succeeds");
    assert_eq!(cohort.members.len(), 3);
    assert_eq!(cohort.vertical_id, "support");
    assert_eq!(cohort.action_type, "resolve");
    // members re-joined in cluster order, trajectories preserved
    assert_eq!(cohort.members[0].session_id, "t1");
    assert_eq!(cohort.members[0].trajectory, vec!["lookup", "update", "confirm"]);
}

#[test]
fn bridge_rejects_unknown_member_id() {
    let cluster = crate::trace_cluster::TraceCluster {
        member_ids: vec!["t1".into(), "ghost".into(), "t3".into()],
        seed_sequence: vec!["a".into()],
        representative_id: "t1".into(),
        mean_success: 0.9,
    };
    let members = vec![
        member("t1", "v", "a", &["a"], 0.9),
        member("t3", "v", "a", &["a"], 0.9),
    ];
    let err = cluster_to_cohort(&cluster, &members, "v", "a").unwrap_err();
    assert_eq!(err, BridgeError::UnknownMember("ghost".into()));
}

#[test]
fn bridge_rejects_too_small_a_cluster() {
    let cluster = crate::trace_cluster::TraceCluster {
        member_ids: vec!["t1".into(), "t2".into()], // 2 < MIN_COHORT_SIZE (3)
        seed_sequence: vec!["a".into()],
        representative_id: "t1".into(),
        mean_success: 0.9,
    };
    let members = vec![
        member("t1", "v", "a", &["a"], 0.9),
        member("t2", "v", "a", &["a"], 0.9),
    ];
    let err = cluster_to_cohort(&cluster, &members, "v", "a").unwrap_err();
    assert_eq!(err, BridgeError::TooSmall(2));
}

#[test]
fn bridged_cohort_is_accepted_by_min_cohort_size_invariant() {
    // The whole point: the bridged cohort must satisfy the same
    // MIN_COHORT_SIZE invariant group_into_cohorts guarantees, so the
    // downstream distill_cohort accepts it.
    let members: Vec<TraceMember> = (0..MIN_COHORT_SIZE)
        .map(|i| member(&format!("t{i}"), "v", "a", &["x", "y"], 0.9))
        .collect();
    let cluster = crate::trace_cluster::TraceCluster {
        member_ids: members.iter().map(|m| m.id.clone()).collect(),
        seed_sequence: vec!["x".into(), "y".into()],
        representative_id: "t0".into(),
        mean_success: 0.9,
    };
    let cohort = cluster_to_cohort(&cluster, &members, "v", "a").expect("bridge");
    assert!(cohort.members.len() >= MIN_COHORT_SIZE);
}

#[test]
fn trace_member_serde_round_trips() {
    let m = member("t1", "support", "resolve", &["a", "b"], 0.8);
    let json = serde_json::to_string(&m).expect("serialize");
    let back: TraceMember = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(m, back);
}

// ---- decide_promotion gate -----------------------------------------

fn report(base_rate: f32, cand_rate: f32) -> SkillAbEvalReport {
    SkillAbEvalReport {
        skill_name: "refund-helper".into(),
        case_count: 10,
        baseline: VariantEvalResult {
            version: "v1".into(),
            pass: (base_rate * 10.0).round() as u32,
            fail: 10 - (base_rate * 10.0).round() as u32,
            pass_rate: base_rate,
        },
        candidate: VariantEvalResult {
            version: "v2".into(),
            pass: (cand_rate * 10.0).round() as u32,
            fail: 10 - (cand_rate * 10.0).round() as u32,
            pass_rate: cand_rate,
        },
        pass_rate_delta: cand_rate - base_rate,
        improved: cand_rate > base_rate,
    }
}

#[test]
fn promotes_a_clear_improvement() {
    let d = decide_promotion(&report(0.5, 0.9), &PromotionPolicy::default());
    assert!(d.is_promote(), "0.5 -> 0.9 must promote: {d:?}");
}

#[test]
fn holds_an_improvement_that_is_too_small() {
    // +3pp delta, below the 5pp bar → Hold (not a win yet).
    let d = decide_promotion(&report(0.80, 0.83), &PromotionPolicy::default());
    assert!(matches!(d, PromotionDecision::Hold { .. }), "got {d:?}");
}

#[test]
fn holds_a_big_improvement_that_stays_below_the_absolute_floor() {
    // +30pp delta but candidate only reaches 0.4 (< 0.5 floor) → Hold:
    // never promote a still-bad skill just because the baseline was worse.
    let d = decide_promotion(&report(0.10, 0.40), &PromotionPolicy::default());
    assert!(matches!(d, PromotionDecision::Hold { .. }), "got {d:?}");
}

#[test]
fn rejects_a_regression() {
    // candidate 0.9 -> 0.7 is a 20pp regression → Reject.
    let d = decide_promotion(&report(0.9, 0.7), &PromotionPolicy::default());
    assert!(matches!(d, PromotionDecision::Reject { .. }), "got {d:?}");
}

#[test]
fn regression_precedence_beats_a_high_absolute_rate() {
    // candidate still high (0.92) but regressed from 0.99 (7pp >= 5pp
    // reject floor) → Reject wins over the absolute-rate consideration.
    let d = decide_promotion(&report(0.99, 0.92), &PromotionPolicy::default());
    assert!(matches!(d, PromotionDecision::Reject { .. }), "got {d:?}");
}

#[test]
fn flat_result_holds() {
    // No change at all → Hold (delta 0 < bar, no regression).
    let d = decide_promotion(&report(0.7, 0.7), &PromotionPolicy::default());
    assert!(matches!(d, PromotionDecision::Hold { .. }), "got {d:?}");
}

#[test]
fn custom_policy_is_honored() {
    // A stricter policy (need +20pp) holds a +10pp improvement that the
    // default would promote.
    let strict = PromotionPolicy {
        min_delta: 0.20,
        min_candidate_rate: 0.5,
        max_regression: 0.05,
    };
    let d = decide_promotion(&report(0.6, 0.7), &strict);
    assert!(matches!(d, PromotionDecision::Hold { .. }), "got {d:?}");
    // …but the default WOULD promote the same report.
    assert!(decide_promotion(&report(0.6, 0.7), &PromotionPolicy::default()).is_promote());
}

#[test]
fn decision_serde_round_trips() {
    let d = decide_promotion(&report(0.5, 0.9), &PromotionPolicy::default());
    let json = serde_json::to_string(&d).expect("serialize");
    let back: PromotionDecision = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(d, back);
}

#[test]
fn promotion_reason_carries_the_measured_numbers() {
    match decide_promotion(&report(0.5, 0.9), &PromotionPolicy::default()) {
        PromotionDecision::Promote { reason } => {
            assert!(reason.contains("v2"), "reason names the candidate: {reason}");
            assert!(reason.contains("40pp"), "reason states the delta: {reason}");
        }
        other => panic!("expected Promote, got {other:?}"),
    }
}

#[test]
fn default_policy_values() {
    let p = PromotionPolicy::default();
    assert!((p.min_delta - 0.05).abs() < f32::EPSILON);
    assert!((p.min_candidate_rate - 0.5).abs() < f32::EPSILON);
    assert!((p.max_regression - 0.05).abs() < f32::EPSILON);
}
