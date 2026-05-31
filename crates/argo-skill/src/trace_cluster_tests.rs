//! Tests for production-trace clustering (cross-`(vertical,action)`-key
//! sequence-similarity clustering).

use super::*;

fn fp(id: &str, seq: &[&str], success: f64) -> TraceFingerprint {
    TraceFingerprint::from_parts(id, seq.iter().map(|s| (*s).to_string()).collect(), success)
}

#[test]
fn identical_shapes_cluster_across_the_key_boundary() {
    // a + b have the SAME action sequence (would live under different
    // (vertical,action) cohort keys in dreams_distillation, never grouped
    // there); c is a different shape.
    let traces = vec![
        fp("a", &["lookup", "update", "confirm"], 0.9),
        fp("b", &["lookup", "update", "confirm"], 0.8),
        fp("c", &["wipe", "reboot"], 0.5),
    ];
    let clusters = cluster_traces(&traces, 0.6);
    assert_eq!(clusters.len(), 2);
    assert_eq!(clusters[0].size(), 2);
    assert_eq!(clusters[0].member_ids, vec!["a".to_string(), "b".to_string()]);
    assert_eq!(clusters[1].size(), 1);
    assert_eq!(clusters[1].member_ids, vec!["c".to_string()]);
}

#[test]
fn dissimilar_shapes_each_seed_their_own_cluster() {
    let traces = vec![
        fp("a", &["x", "y"], 0.9),
        fp("b", &["p", "q"], 0.9),
        fp("c", &["m", "n"], 0.9),
    ];
    let clusters = cluster_traces(&traces, 0.6);
    assert_eq!(clusters.len(), 3);
    for c in &clusters {
        assert_eq!(c.size(), 1);
    }
}

#[test]
fn representative_is_the_highest_success_member() {
    let traces = vec![
        fp("low", &["lookup", "update"], 0.4),
        fp("high", &["lookup", "update"], 0.95),
        fp("mid", &["lookup", "update"], 0.7),
    ];
    let clusters = cluster_traces(&traces, 0.6);
    assert_eq!(clusters.len(), 1);
    assert_eq!(clusters[0].representative_id, "high");
    // mean = (0.4 + 0.95 + 0.7)/3
    assert!((clusters[0].mean_success - (0.4 + 0.95 + 0.7) / 3.0).abs() < 1e-9);
}

#[test]
fn representative_ties_resolve_to_earliest_member() {
    let traces = vec![
        fp("first", &["a", "b"], 0.8),
        fp("second", &["a", "b"], 0.8),
    ];
    let clusters = cluster_traces(&traces, 0.6);
    assert_eq!(clusters.len(), 1);
    assert_eq!(clusters[0].representative_id, "first");
}

#[test]
fn min_similarity_is_clamped() {
    let traces = vec![fp("a", &["x"], 0.9), fp("b", &["y"], 0.9)];
    // A wildly-out-of-range threshold must not panic; clamps to [0,1].
    // 5.0 → 1.0 (nothing but exact matches cluster); -5.0 → 0.0 (everything).
    assert_eq!(cluster_traces(&traces, 5.0).len(), 2);
    let lumped = cluster_traces(&traces, -5.0);
    assert_eq!(lumped.len(), 1);
    assert_eq!(lumped[0].size(), 2);
}

#[test]
fn empty_input_yields_no_clusters() {
    assert!(cluster_traces(&[], 0.6).is_empty());
}

#[test]
fn seed_sequence_is_the_first_assigned_member() {
    let traces = vec![
        fp("seed", &["alpha", "beta", "gamma"], 0.9),
        fp("joiner", &["alpha", "beta", "gamma"], 0.8),
    ];
    let clusters = cluster_traces(&traces, 0.6);
    assert_eq!(
        clusters[0].seed_sequence,
        vec!["alpha".to_string(), "beta".to_string(), "gamma".to_string()]
    );
}

// ---- distillation_candidates -----------------------------------------

#[test]
fn candidates_filter_by_size_and_mean_success() {
    let traces = vec![
        // big, successful cluster (3 members, mean 0.9) → candidate
        fp("p1", &["lookup", "update"], 0.9),
        fp("p2", &["lookup", "update"], 0.9),
        fp("p3", &["lookup", "update"], 0.9),
        // big but FAILING cluster (3 members, mean 0.2) → filtered out
        fp("f1", &["crash", "retry"], 0.2),
        fp("f2", &["crash", "retry"], 0.2),
        fp("f3", &["crash", "retry"], 0.2),
        // successful but TOO SMALL (1 member) → filtered out
        fp("s1", &["once"], 0.99),
    ];
    let clusters = cluster_traces(&traces, 0.6);
    let cands = distillation_candidates(&clusters, DEFAULT_MIN_CLUSTER_SIZE, DEFAULT_MIN_MEAN_SUCCESS);
    assert_eq!(cands.len(), 1);
    assert_eq!(cands[0].member_ids, vec!["p1", "p2", "p3"]);
}

#[test]
fn candidates_sorted_by_size_then_success_then_id() {
    // two qualifying clusters of different sizes → larger first.
    let traces = vec![
        fp("a1", &["x", "y"], 0.8),
        fp("a2", &["x", "y"], 0.8),
        fp("a3", &["x", "y"], 0.8),
        fp("b1", &["p", "q"], 0.95),
        fp("b2", &["p", "q"], 0.95),
        fp("b3", &["p", "q"], 0.95),
        fp("b4", &["p", "q"], 0.95),
    ];
    let clusters = cluster_traces(&traces, 0.6);
    let cands = distillation_candidates(&clusters, 3, 0.7);
    assert_eq!(cands.len(), 2);
    // b-cluster has 4 members → comes first (size desc).
    assert_eq!(cands[0].size(), 4);
    assert_eq!(cands[1].size(), 3);
}

#[test]
fn no_candidates_when_nothing_meets_floor() {
    let traces = vec![fp("a", &["x"], 0.9), fp("b", &["x"], 0.9)];
    let clusters = cluster_traces(&traces, 0.6);
    // require 5 members; only 2 exist → no candidates.
    assert!(distillation_candidates(&clusters, 5, 0.7).is_empty());
}

#[test]
fn cluster_and_fingerprint_serde_round_trip() {
    let traces = vec![fp("a", &["x", "y"], 0.9), fp("b", &["x", "y"], 0.8)];
    let clusters = cluster_traces(&traces, 0.6);
    let json = serde_json::to_string(&clusters[0]).expect("serialize cluster");
    let back: TraceCluster = serde_json::from_str(&json).expect("deserialize cluster");
    assert_eq!(clusters[0], back);

    let fjson = serde_json::to_string(&traces[0]).expect("serialize fp");
    let fback: TraceFingerprint = serde_json::from_str(&fjson).expect("deserialize fp");
    assert_eq!(traces[0], fback);
}

#[test]
fn defaults_are_sane() {
    assert!((DEFAULT_MIN_SIMILARITY - 0.6).abs() < f64::EPSILON);
    assert_eq!(DEFAULT_MIN_CLUSTER_SIZE, 3);
    assert!((DEFAULT_MIN_MEAN_SUCCESS - 0.7).abs() < f64::EPSILON);
}
