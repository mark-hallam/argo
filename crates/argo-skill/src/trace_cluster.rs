//! Production-trace clustering — the cross-key half of the
//! feedback-mining loop (FEATURE_MATRIX "Self-improve" → "production-trace
//! clustering", Step 2).
//!
//! [`crate::dreams_distillation::group_into_cohorts`] groups CU
//! trajectories by their EXACT `(vertical_id, action_type)` key, then
//! tightens each group by similarity — its own doc flags semantic
//! similarity clustering as "a follow-up". This module is that follow-up:
//! it clusters traces purely by **action-sequence similarity, across the
//! `(vertical, action)` key boundary**, so the same underlying pattern
//! emitted under two different verticals/actions (e.g. a
//! "lookup → update → confirm" shape that recurs in both `support` and
//! `gtm`) lands in ONE cluster and becomes a single skill-distillation
//! candidate — which the keyed grouping can never surface.
//!
//! Reuses the proven [`crate::dreams_distillation::trajectory_similarity`]
//! (bag-of-tokens cosine over action names) as the distance metric — no
//! parallel similarity abstraction. Pure + deterministic (no model, no
//! store): the same input slice in the same order always yields the same
//! clusters, so it is safe for CI + offline analysis.
use crate::dreams_distillation::trajectory_similarity;
use serde::{Deserialize, Serialize};

/// Default similarity floor for two traces to land in the same cluster
/// (bag-of-tokens cosine over action names). Conservative — only
/// genuinely-similar shapes cluster.
pub const DEFAULT_MIN_SIMILARITY: f64 = 0.6;

/// Default minimum members before a cluster is a distillation candidate
/// (a one-off trace is not a reusable pattern).
pub const DEFAULT_MIN_CLUSTER_SIZE: usize = 3;

/// Default mean-success floor for a distillation candidate — only
/// cluster patterns that *worked* are worth turning into a skill.
pub const DEFAULT_MIN_MEAN_SUCCESS: f64 = 0.7;

/// Minimal owned fingerprint of one production trace — the only fields
/// trace-clustering needs. Decoupled from
/// [`crate::dreams_distillation::CuTrajectory`] so this module does not
/// depend on that type's full shape; [`TraceFingerprint::from_parts`]
/// builds one from the relevant CU-trajectory fields.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TraceFingerprint {
    /// Stable id of the trace (e.g. the session id that produced it).
    pub id: String,
    /// Ordered action names the trace emitted (the clustering signal).
    pub action_sequence: Vec<String>,
    /// Final success metric of the trace, 0.0–1.0.
    pub success_metric: f64,
}

impl TraceFingerprint {
    /// Build a fingerprint from the trace fields clustering cares about.
    #[must_use]
    pub fn from_parts(id: impl Into<String>, action_sequence: Vec<String>, success_metric: f64) -> Self {
        Self {
            id: id.into(),
            action_sequence,
            success_metric,
        }
    }
}

/// One cluster of similar traces — a candidate recurring pattern.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TraceCluster {
    /// Ids of the member traces, in the order they were assigned.
    pub member_ids: Vec<String>,
    /// The seed trace's action sequence (the cluster's reference shape).
    pub seed_sequence: Vec<String>,
    /// Id of the highest-success member (the best exemplar to distil
    /// from). Ties resolve to the earliest such member (deterministic).
    pub representative_id: String,
    /// Mean `success_metric` across members.
    pub mean_success: f64,
}

impl TraceCluster {
    /// Number of traces in the cluster.
    #[must_use]
    pub fn size(&self) -> usize {
        self.member_ids.len()
    }
}

/// Greedy single-pass clustering of traces by action-sequence
/// similarity, **ignoring** the `(vertical, action)` key.
///
/// Each trace joins the first existing cluster whose SEED it is
/// `>= min_similarity` to (via [`trajectory_similarity`]); otherwise it
/// seeds a new cluster. Single-pass + seed-anchored keeps it
/// deterministic (output depends only on input order) and O(n·k) for
/// `k` clusters — the right cost for an offline mining pass. A trace
/// with an empty action sequence still clusters (cosine of two empty
/// sets is treated as 0 by the underlying metric, so it only joins
/// another empty-sequence seed).
///
/// `min_similarity` is clamped to `0.0..=1.0`.
///
/// # Examples
///
/// ```
/// use argo_skill::trace_cluster::{cluster_traces, TraceFingerprint};
///
/// let traces = vec![
///     TraceFingerprint::from_parts("a", vec!["lookup".into(), "update".into(), "confirm".into()], 0.9),
///     // same shape, DIFFERENT vertical/action would share no cohort key —
///     // but clusters here because the action sequence matches:
///     TraceFingerprint::from_parts("b", vec!["lookup".into(), "update".into(), "confirm".into()], 0.8),
///     TraceFingerprint::from_parts("c", vec!["wipe".into(), "reboot".into()], 0.5),
/// ];
/// let clusters = cluster_traces(&traces, 0.6);
/// assert_eq!(clusters.len(), 2);
/// assert_eq!(clusters[0].size(), 2); // a + b
/// ```
#[must_use]
pub fn cluster_traces(traces: &[TraceFingerprint], min_similarity: f64) -> Vec<TraceCluster> {
    let threshold = min_similarity.clamp(0.0, 1.0);
    // Accumulate members per cluster; finalize stats after assignment.
    let mut seeds: Vec<&TraceFingerprint> = Vec::new();
    let mut groups: Vec<Vec<&TraceFingerprint>> = Vec::new();
    for t in traces {
        let mut placed = false;
        for (i, seed) in seeds.iter().enumerate() {
            if trajectory_similarity(&seed.action_sequence, &t.action_sequence) >= threshold {
                groups[i].push(t);
                placed = true;
                break;
            }
        }
        if !placed {
            seeds.push(t);
            groups.push(vec![t]);
        }
    }
    seeds
        .iter()
        .zip(groups.iter())
        .map(|(seed, members)| finalize_cluster(seed, members))
        .collect()
}

/// Turn a seed + its assigned members into a [`TraceCluster`] with mean
/// success + the highest-success representative.
fn finalize_cluster(seed: &TraceFingerprint, members: &[&TraceFingerprint]) -> TraceCluster {
    let mean_success = if members.is_empty() {
        0.0
    } else {
        members.iter().map(|m| m.success_metric).sum::<f64>() / members.len() as f64
    };
    // Representative = highest success_metric; ties → earliest member.
    let representative_id = members
        .iter()
        .fold(None::<&&TraceFingerprint>, |best, m| match best {
            Some(b) if b.success_metric >= m.success_metric => Some(b),
            _ => Some(m),
        })
        .map(|m| m.id.clone())
        .unwrap_or_else(|| seed.id.clone());
    TraceCluster {
        member_ids: members.iter().map(|m| m.id.clone()).collect(),
        seed_sequence: seed.action_sequence.clone(),
        representative_id,
        mean_success,
    }
}

/// Filter clusters to the ones worth distilling into a skill: at least
/// `min_size` members AND `mean_success >= min_mean_success`. Returned
/// sorted by size (desc), then mean success (desc), then representative
/// id (asc) — deterministic, strongest recurring pattern first.
#[must_use]
pub fn distillation_candidates(
    clusters: &[TraceCluster],
    min_size: usize,
    min_mean_success: f64,
) -> Vec<TraceCluster> {
    let mut out: Vec<TraceCluster> = clusters
        .iter()
        .filter(|c| c.size() >= min_size && c.mean_success >= min_mean_success)
        .cloned()
        .collect();
    out.sort_by(|a, b| {
        b.size()
            .cmp(&a.size())
            .then(
                b.mean_success
                    .partial_cmp(&a.mean_success)
                    .unwrap_or(std::cmp::Ordering::Equal),
            )
            .then(a.representative_id.cmp(&b.representative_id))
    });
    out
}

#[cfg(test)]
#[path = "trace_cluster_tests.rs"]
mod tests;
