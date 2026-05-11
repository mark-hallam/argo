//! Unit tests for [`super::TrustGraph`].
//!
//! Each test names the brief's acceptance criterion it exercises so
//! a reader can map test→spec line-by-line.

use super::*;
use argo_types::{SkillId, TenantId};
use chrono::{Duration, Utc};

fn obs(
    source: TenantId,
    consumer: TenantId,
    quality: f32,
    observed_at: chrono::DateTime<Utc>,
) -> TrustObservation {
    TrustObservation {
        source_tenant: source,
        consumer_tenant: consumer,
        skill_id: SkillId::new(),
        quality_score: quality,
        observed_at,
    }
}

#[test]
fn direct_trust_returns_edge_weight() {
    let a = TenantId::new();
    let b = TenantId::new();
    let now = Utc::now();
    let graph = TrustGraph::from_observations_at(&[obs(b, a, 0.8, now)], now);
    let w = graph.direct_trust(&a, &b).expect("edge exists");
    assert!((w - 0.8).abs() < 1e-4, "expected ~0.8, got {w}");
}

#[test]
fn composed_2hop_multiplies_edge_weights() {
    let a = TenantId::new();
    let b = TenantId::new();
    let c = TenantId::new();
    let now = Utc::now();
    let graph = TrustGraph::from_observations_at(
        &[obs(b, a, 0.8, now), obs(c, b, 0.5, now)],
        now,
    );
    let composed = graph.composed_trust(&a, &c, 3).expect("path exists");
    assert!(
        (composed - 0.4).abs() < 1e-3,
        "expected 0.8 * 0.5 = 0.4, got {composed}"
    );
}

#[test]
fn composed_caps_at_max_hops_3() {
    // a→b→c→d→e: 4 hops. With max_hops=3, e should be unreachable.
    let a = TenantId::new();
    let b = TenantId::new();
    let c = TenantId::new();
    let d = TenantId::new();
    let e = TenantId::new();
    let now = Utc::now();
    let graph = TrustGraph::from_observations_at(
        &[
            obs(b, a, 0.9, now),
            obs(c, b, 0.9, now),
            obs(d, c, 0.9, now),
            obs(e, d, 0.9, now),
        ],
        now,
    );
    assert!(graph.composed_trust(&a, &e, 3).is_none(), "4-hop path should not compose");
    // With max_hops=4 it should resolve.
    let four_hop = graph.composed_trust(&a, &e, 4).expect("4-hop allowed");
    let expected = 0.9_f32.powi(4);
    assert!(
        (four_hop - expected).abs() < 1e-3,
        "expected {expected}, got {four_hop}"
    );
}

#[test]
fn composed_chooses_max_across_alternative_paths() {
    // a→b→c (weight 0.4) and a→d→c (weight 0.8). Composed should
    // pick the stronger path.
    let a = TenantId::new();
    let b = TenantId::new();
    let c = TenantId::new();
    let d = TenantId::new();
    let now = Utc::now();
    let graph = TrustGraph::from_observations_at(
        &[
            obs(b, a, 1.0, now),
            obs(c, b, 0.4, now),
            obs(d, a, 1.0, now),
            obs(c, d, 0.8, now),
        ],
        now,
    );
    let composed = graph.composed_trust(&a, &c, 3).expect("path exists");
    assert!(
        (composed - 0.8).abs() < 1e-3,
        "expected the max (0.8), got {composed}"
    );
}

#[test]
fn isolated_tenant_returns_none() {
    let a = TenantId::new();
    let unknown = TenantId::new();
    let graph = TrustGraph::from_observations_at(&[], Utc::now());
    assert!(graph.direct_trust(&a, &unknown).is_none());
    assert!(graph.composed_trust(&a, &unknown, 3).is_none());
}

#[test]
fn observation_aggregation_averages_quality_scores() {
    // Two observations on the same edge with identical timestamps
    // → mean of 0.6 and 1.0 = 0.8.
    let a = TenantId::new();
    let b = TenantId::new();
    let now = Utc::now();
    let graph = TrustGraph::from_observations_at(
        &[obs(b, a, 0.6, now), obs(b, a, 1.0, now)],
        now,
    );
    let w = graph.direct_trust(&a, &b).expect("edge exists");
    assert!(
        (w - 0.8).abs() < 1e-3,
        "expected mean 0.8 (clamped), got {w}"
    );
    let neighbor = graph
        .neighbors(&a)
        .into_iter()
        .find(|e| e.to_tenant == b)
        .expect("neighbor present");
    assert_eq!(neighbor.observation_count, 2);
}

#[test]
fn cycle_does_not_deadlock_bfs() {
    // Triangle a→b→c→a — BFS over composed_trust must terminate.
    let a = TenantId::new();
    let b = TenantId::new();
    let c = TenantId::new();
    let now = Utc::now();
    let graph = TrustGraph::from_observations_at(
        &[obs(b, a, 0.5, now), obs(c, b, 0.5, now), obs(a, c, 0.5, now)],
        now,
    );
    // a→b (direct, 0.5) is the only 1-hop trust into b.
    assert_eq!(graph.composed_trust(&a, &b, 3), Some(0.5));
    // a→a should be None (no self-loop, BFS rejects start==to).
    assert!(graph.composed_trust(&a, &a, 3).is_none());
    // a→c via a→b→c = 0.25.
    let ac = graph.composed_trust(&a, &c, 3).expect("path exists");
    assert!((ac - 0.25).abs() < 1e-3, "expected 0.25, got {ac}");
}

#[test]
fn neighbors_returns_all_outgoing_edges() {
    let a = TenantId::new();
    let b = TenantId::new();
    let c = TenantId::new();
    let d = TenantId::new();
    let now = Utc::now();
    let graph = TrustGraph::from_observations_at(
        &[obs(b, a, 0.8, now), obs(c, a, 0.5, now), obs(d, b, 0.4, now)],
        now,
    );
    let mut targets: Vec<TenantId> = graph
        .neighbors(&a)
        .into_iter()
        .map(|e| e.to_tenant)
        .collect();
    targets.sort_by_key(|t| t.0);
    let mut expected = vec![b, c];
    expected.sort_by_key(|t| t.0);
    assert_eq!(targets, expected);
}

#[test]
fn weight_clamps_to_one_for_rogue_quality_score() {
    // Single observation with quality > 1.0 must NOT poison the
    // edge — defense-in-depth clamp at the graph layer.
    let a = TenantId::new();
    let b = TenantId::new();
    let now = Utc::now();
    let graph = TrustGraph::from_observations_at(&[obs(b, a, 5.0, now)], now);
    let w = graph.direct_trust(&a, &b).expect("edge exists");
    assert!(w <= 1.0, "weight must clamp to <= 1.0; got {w}");
    assert!((w - 1.0).abs() < 1e-3, "expected clamped 1.0, got {w}");
}

#[test]
fn empty_graph_does_not_panic_on_queries() {
    let graph = TrustGraph::default();
    let t = TenantId::new();
    let u = TenantId::new();
    assert!(graph.direct_trust(&t, &u).is_none());
    assert!(graph.composed_trust(&t, &u, 3).is_none());
    assert!(graph.neighbors(&t).is_empty());
    assert_eq!(graph.edge_count(), 0);
}

#[test]
fn linear_time_decay_drops_observations_outside_window() {
    // Two observations: one fresh, one >90 days old. The old one
    // should be dropped entirely, leaving only the fresh signal.
    let a = TenantId::new();
    let b = TenantId::new();
    let now = Utc::now();
    let old = now - Duration::days(120);
    let graph = TrustGraph::from_observations_at(
        &[obs(b, a, 0.1, old), obs(b, a, 0.9, now)],
        now,
    );
    let w = graph.direct_trust(&a, &b).expect("edge exists");
    assert!(
        (w - 0.9).abs() < 1e-3,
        "old observation must be dropped; expected 0.9, got {w}"
    );
}
