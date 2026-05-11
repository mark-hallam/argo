//! Cross-tenant trust-graph primitive (Stream
//! `mission-cross-tenant-trust-graph`, MISSION_BACKLOG Section 3
//! Claim 4 #81).
//!
//! Cross-tenant skill sharing already ships (`skill.shared = 1` is
//! visible to viewers in the same cluster) and per-skill trust grades
//! ship via [`crate::trust::TrustGrade`]. What was missing is a
//! GRAPH primitive that composes trust signals across tenants:
//!
//! - Tenant A trusts Tenant B's skills heavily.
//! - Tenant B trusts Tenant C's skills lightly.
//! - Tenant A inherits a moderate trust signal for C's skills via
//!   the composed path A→B→C.
//!
//! `TrustGraph` builds in-memory from a Vec of [`TrustObservation`]
//! values. Each observation is one quality score reported by a
//! consumer tenant for a skill shipped by a source tenant. Multiple
//! observations on the same `(source, consumer)` edge are averaged
//! (mean) and clamped to `0.0..=1.0` with a linear time-decay over
//! the configured window.
//!
//! ## Composition semantics
//!
//! - **Hop 1** = direct edge weight (`direct_trust`).
//! - **Hop 2+** = product of edge weights along the path. BFS
//!   explores all paths to `max_hops`; `composed_trust` returns the
//!   `max` across alternative paths.
//! - Default cap: 3 hops. Signal at 4+ hops is too lossy to be
//!   meaningful (operator-facing knob; brief calls out a follow-up
//!   if 5+ hops becomes desirable).
//! - Cycles are detected via a per-BFS `visited` set keyed on the
//!   path frontier, so a triangle (A→B→C→A) does not deadlock.
//!
//! ## Why max(product) instead of sum or average
//!
//! "I trust C because the *best* path through someone I trust gives
//! me a good signal", not "I trust C because many noisy paths exist".
//! Aggregating with sum would let a swarm of weak paths overpower
//! one strong one — that is precisely the social-engineering attack
//! vector trust graphs need to resist.
//!
//! ## Pure-Rust by construction
//!
//! No DB IO inside `TrustGraph` so the type is fast to test against
//! synthesized observation Vecs. The CLI layer is responsible for
//! reading `trust_observation` rows from `argo_store` and passing the
//! Vec to [`TrustGraph::from_observations`].
//!
//! # Example
//!
//! ```
//! use argo_skill::trust_graph::{TrustGraph, TrustObservation};
//! use argo_types::{SkillId, TenantId};
//! use chrono::Utc;
//!
//! let a = TenantId::new();
//! let b = TenantId::new();
//! let c = TenantId::new();
//! let now = Utc::now();
//!
//! let obs = vec![
//!     TrustObservation {
//!         source_tenant: b,
//!         consumer_tenant: a,
//!         skill_id: SkillId::new(),
//!         quality_score: 0.9,
//!         observed_at: now,
//!     },
//!     TrustObservation {
//!         source_tenant: c,
//!         consumer_tenant: b,
//!         skill_id: SkillId::new(),
//!         quality_score: 0.6,
//!         observed_at: now,
//!     },
//! ];
//! let graph = TrustGraph::from_observations(&obs);
//! assert!(graph.direct_trust(&a, &b).is_some());
//! assert!(graph.composed_trust(&a, &c, 3).is_some());
//! ```

use argo_types::{SkillId, TenantId};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

/// Default time window over which observations contribute to an
/// edge's weight. Observations older than this are dropped at
/// construction time; observations within the window are linearly
/// decayed by their age (full weight at now, zero at `now -
/// DECAY_WINDOW`).
pub const DECAY_WINDOW_DAYS: i64 = 90;

/// Default maximum hop count for `composed_trust`. Signal at 4+
/// hops is too lossy to be meaningful as a v1 default.
pub const DEFAULT_MAX_HOPS: usize = 3;

/// One persisted observation. The graph layer is pure — there is
/// no DB IO here. `argo_store::trust_observation` is the persistence
/// layer; CLI / orchestrator code reads rows and constructs this
/// Vec.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrustObservation {
    /// Tenant that shipped the skill.
    pub source_tenant: TenantId,
    /// Tenant that adopted the skill and reported a quality score.
    pub consumer_tenant: TenantId,
    /// Skill the observation references. Kept on the observation so
    /// future tuning can filter by skill class / risk tier without
    /// rebuilding the graph; today the graph aggregates across all
    /// skills.
    pub skill_id: SkillId,
    /// Observed quality on the consumer side. Stored verbatim;
    /// edge construction clamps to `0.0..=1.0`.
    pub quality_score: f32,
    /// Wall-clock time of the observation.
    pub observed_at: DateTime<Utc>,
}

/// One materialized edge in the graph. Direction is `from →
/// to`: the consumer tenant trusts the source tenant. (In the
/// observation, `consumer_tenant` reports a score for
/// `source_tenant`'s skill; that is the consumer expressing trust
/// in the source.)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrustEdge {
    /// Tenant doing the trusting (observation's `consumer_tenant`).
    pub from_tenant: TenantId,
    /// Tenant being trusted (observation's `source_tenant`).
    pub to_tenant: TenantId,
    /// Edge weight in `0.0..=1.0`. Composed from quality scores
    /// (mean) with linear time-decay over [`DECAY_WINDOW_DAYS`].
    pub weight: f32,
    /// Number of distinct observations that contributed to the
    /// weight. Drives operator-facing confidence reporting.
    pub observation_count: u32,
    /// Most recent observation timestamp that contributed.
    pub last_updated_at: DateTime<Utc>,
}

/// Materialized pairwise trust graph. Construct via
/// [`TrustGraph::from_observations`]; query via [`Self::direct_trust`]
/// / [`Self::composed_trust`] / [`Self::neighbors`].
#[derive(Debug, Clone, Default)]
pub struct TrustGraph {
    edges: HashMap<(TenantId, TenantId), TrustEdge>,
}

impl TrustGraph {
    /// Build a graph from a Vec of observations. Internally:
    ///
    /// 1. Bucket observations by `(consumer_tenant, source_tenant)`.
    /// 2. For each bucket, compute the time-decayed mean of clamped
    ///    quality scores. Decay factor for observation `o` is
    ///    `max(0, 1 - (now - o.observed_at) / DECAY_WINDOW)`.
    ///    Observations whose decay factor is 0 are dropped.
    /// 3. Final edge weight is clamped to `0.0..=1.0`.
    ///
    /// Empty Vec → empty graph (no panics, no edges).
    #[must_use]
    pub fn from_observations(observations: &[TrustObservation]) -> Self {
        Self::from_observations_at(observations, Utc::now())
    }

    /// Like [`Self::from_observations`] but with an injectable
    /// "now". Used by tests so decay math is deterministic.
    #[must_use]
    pub fn from_observations_at(
        observations: &[TrustObservation],
        now: DateTime<Utc>,
    ) -> Self {
        let decay_window = Duration::days(DECAY_WINDOW_DAYS);
        let window_secs = decay_window.num_seconds().max(1) as f32;

        let mut buckets: HashMap<(TenantId, TenantId), Bucket> = HashMap::new();
        for obs in observations {
            let age = now - obs.observed_at;
            if age > decay_window || age < Duration::zero() {
                continue;
            }
            let age_secs = age.num_seconds().max(0) as f32;
            let decay = (1.0 - age_secs / window_secs).clamp(0.0, 1.0);
            if decay == 0.0 {
                continue;
            }
            let clamped = obs.quality_score.clamp(0.0, 1.0);
            let key = (obs.consumer_tenant, obs.source_tenant);
            let entry = buckets.entry(key).or_default();
            entry.weighted_sum += clamped * decay;
            entry.decay_sum += decay;
            entry.count += 1;
            if entry.last_updated_at < obs.observed_at {
                entry.last_updated_at = obs.observed_at;
            }
        }

        let mut edges = HashMap::with_capacity(buckets.len());
        for ((from, to), b) in buckets {
            if b.decay_sum == 0.0 {
                continue;
            }
            let weight = (b.weighted_sum / b.decay_sum).clamp(0.0, 1.0);
            edges.insert(
                (from, to),
                TrustEdge {
                    from_tenant: from,
                    to_tenant: to,
                    weight,
                    observation_count: b.count,
                    last_updated_at: b.last_updated_at,
                },
            );
        }
        Self { edges }
    }

    /// Direct edge weight from `from` to `to`, or `None` if no edge
    /// exists.
    #[must_use]
    pub fn direct_trust(&self, from: &TenantId, to: &TenantId) -> Option<f32> {
        self.edges.get(&(*from, *to)).map(|e| e.weight)
    }

    /// Composed trust from `from` to `to` over BFS paths up to
    /// `max_hops`. Returns the *maximum* product of edge weights
    /// across alternative paths, or `None` if no path exists.
    ///
    /// `max_hops == 0` returns `None` (no edge counts at zero hops).
    /// `max_hops == 1` is equivalent to [`Self::direct_trust`].
    /// `max_hops > DEFAULT_MAX_HOPS` is honored but discouraged; the
    /// brief deliberately caps the default at 3.
    #[must_use]
    pub fn composed_trust(
        &self,
        from: &TenantId,
        to: &TenantId,
        max_hops: usize,
    ) -> Option<f32> {
        if max_hops == 0 || from == to {
            return None;
        }
        // BFS frontier: (current_node, accumulated_weight, depth, visited_set).
        // `visited` lives per-path so a cycle through one path doesn't
        // poison another. This is safe because max_hops is small
        // (default 3); branching factor times depth stays bounded.
        let mut best: Option<f32> = None;
        let mut frontier: VecDeque<(TenantId, f32, usize, HashSet<TenantId>)> = VecDeque::new();
        let mut start_visited = HashSet::new();
        start_visited.insert(*from);
        frontier.push_back((*from, 1.0, 0, start_visited));

        while let Some((node, acc, depth, visited)) = frontier.pop_front() {
            if depth >= max_hops {
                continue;
            }
            for ((edge_from, edge_to), edge) in &self.edges {
                if *edge_from != node {
                    continue;
                }
                if visited.contains(edge_to) {
                    continue;
                }
                let next_weight = acc * edge.weight;
                if *edge_to == *to {
                    best = Some(best.map_or(next_weight, |b| b.max(next_weight)));
                } else {
                    let mut next_visited = visited.clone();
                    next_visited.insert(*edge_to);
                    frontier.push_back((*edge_to, next_weight, depth + 1, next_visited));
                }
            }
        }
        best
    }

    /// Like [`Self::composed_trust`] but returns the winning path
    /// alongside the score. The path is `[from, hop1, …, to]` —
    /// length 2 for a direct edge, length 3 for a 2-hop composition,
    /// etc. Returns `None` if no path exists.
    ///
    /// Cost is bounded by `max_hops` × edge count; intended for
    /// operator-facing tooling (CLI / dashboards) that wants to
    /// surface the *why* behind a composed score.
    #[must_use]
    pub fn composed_trust_with_path(
        &self,
        from: &TenantId,
        to: &TenantId,
        max_hops: usize,
    ) -> Option<(f32, Vec<TenantId>)> {
        if max_hops == 0 || from == to {
            return None;
        }
        let mut best: Option<(f32, Vec<TenantId>)> = None;
        let mut visited = HashSet::new();
        visited.insert(*from);
        let mut frontier: VecDeque<PathFrame> = VecDeque::new();
        frontier.push_back(PathFrame {
            node: *from,
            acc: 1.0,
            depth: 0,
            visited,
            path: vec![*from],
        });
        while let Some(PathFrame { node, acc, depth, visited: vis, path }) = frontier.pop_front() {
            if depth >= max_hops {
                continue;
            }
            for ((edge_from, edge_to), edge) in &self.edges {
                if *edge_from != node || vis.contains(edge_to) {
                    continue;
                }
                let next_weight = acc * edge.weight;
                let mut next_path = path.clone();
                next_path.push(*edge_to);
                if *edge_to == *to {
                    if best.as_ref().is_none_or(|(s, _)| next_weight > *s) {
                        best = Some((next_weight, next_path));
                    }
                } else {
                    let mut next_vis = vis.clone();
                    next_vis.insert(*edge_to);
                    frontier.push_back(PathFrame {
                        node: *edge_to,
                        acc: next_weight,
                        depth: depth + 1,
                        visited: next_vis,
                        path: next_path,
                    });
                }
            }
        }
        best
    }

    /// All outgoing edges from `tenant`. Empty Vec if the tenant is
    /// isolated (no outgoing trust).
    #[must_use]
    pub fn neighbors(&self, tenant: &TenantId) -> Vec<&TrustEdge> {
        self.edges
            .iter()
            .filter_map(|((from, _), edge)| if from == tenant { Some(edge) } else { None })
            .collect()
    }

    /// Total edge count. Useful for operator dashboards and tests.
    #[must_use]
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Iterate every edge in the graph in unspecified order.
    pub fn edges(&self) -> impl Iterator<Item = &TrustEdge> {
        self.edges.values()
    }
}

#[derive(Debug, Default)]
struct Bucket {
    weighted_sum: f32,
    decay_sum: f32,
    count: u32,
    last_updated_at: DateTime<Utc>,
}

struct PathFrame {
    node: TenantId,
    acc: f32,
    depth: usize,
    visited: HashSet<TenantId>,
    path: Vec<TenantId>,
}

#[cfg(test)]
#[path = "trust_graph_tests.rs"]
mod tests;
