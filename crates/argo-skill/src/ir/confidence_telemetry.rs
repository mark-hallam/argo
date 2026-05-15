//! Tenant-scoped confidence telemetry for Skill IR nodes.
//!
//! L5 emits node lifecycle events (`node.entered`, `node.completed`,
//! `node.failed`) and the canvas already reads confidence from
//! [`NodeAnnotations`]. This module bridges those two surfaces with a
//! pure aggregation helper: callers record per-node outcomes, query
//! tenant-scoped rows, and annotate a [`SkillGraph`] before returning it
//! to UI/runtime consumers.

use crate::ir::{NodeAnnotations, SkillGraph};
use chrono::Utc;
use std::collections::BTreeMap;

/// Per-node telemetry identity.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeTelemetryKey {
    /// Tenant that owns the run. Included in the key so rows are
    /// tenant-isolated even when skill ids collide across tenants.
    pub tenant_id: String,
    /// Skill id whose IR node emitted the sample.
    pub skill_id: String,
    /// Skill version associated with the executing graph.
    pub version: String,
    /// Node id inside the graph.
    pub node_id: String,
}

impl NodeTelemetryKey {
    /// Build a telemetry key from borrowed route/runtime parts.
    #[must_use]
    pub fn new(tenant_id: &str, skill_id: &str, version: &str, node_id: &str) -> Self {
        Self {
            tenant_id: tenant_id.to_string(),
            skill_id: skill_id.to_string(),
            version: version.to_string(),
            node_id: node_id.to_string(),
        }
    }
}

/// One observed node outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TelemetryOutcome {
    /// The node completed successfully.
    Success,
    /// The node failed.
    Failure,
}

/// One telemetry sample from a trace event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelemetrySample {
    /// Sample key.
    pub key: NodeTelemetryKey,
    /// Outcome to record.
    pub outcome: TelemetryOutcome,
    /// RFC-3339 timestamp. When absent, [`Utc::now`] is used.
    pub observed_at: Option<String>,
}

impl TelemetrySample {
    /// Build a successful sample.
    #[must_use]
    pub fn success(key: NodeTelemetryKey) -> Self {
        Self {
            key,
            outcome: TelemetryOutcome::Success,
            observed_at: None,
        }
    }

    /// Build a failed sample.
    #[must_use]
    pub fn failure(key: NodeTelemetryKey) -> Self {
        Self {
            key,
            outcome: TelemetryOutcome::Failure,
            observed_at: None,
        }
    }
}

/// Aggregated telemetry row for one node.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct NodeTelemetryRow {
    /// Number of samples observed for this node.
    pub sample_count: u64,
    /// Number of successful samples.
    pub success_count: u64,
    /// Number of failed samples.
    pub failure_count: u64,
    /// RFC-3339 timestamp of the most recent sample.
    pub last_used_at: Option<String>,
}

impl NodeTelemetryRow {
    /// Confidence for the node, or `None` when no samples exist.
    #[must_use]
    pub fn confidence(&self) -> Option<f64> {
        if self.sample_count == 0 {
            return None;
        }
        Some(self.success_count as f64 / self.sample_count as f64)
    }

    fn record(&mut self, outcome: TelemetryOutcome, observed_at: Option<String>) {
        self.sample_count = self.sample_count.saturating_add(1);
        match outcome {
            TelemetryOutcome::Success => {
                self.success_count = self.success_count.saturating_add(1);
            }
            TelemetryOutcome::Failure => {
                self.failure_count = self.failure_count.saturating_add(1);
            }
        }
        self.last_used_at = Some(observed_at.unwrap_or_else(|| Utc::now().to_rfc3339()));
    }

    fn to_annotations(&self) -> NodeAnnotations {
        NodeAnnotations {
            confidence: self.confidence(),
            sample_count: Some(self.sample_count),
            last_used_at: self.last_used_at.clone(),
            success_rate: self.confidence(),
            ..NodeAnnotations::default()
        }
    }
}

/// In-memory confidence telemetry aggregator.
#[derive(Debug, Clone, Default)]
pub struct ConfidenceTelemetry {
    rows: BTreeMap<NodeTelemetryKey, NodeTelemetryRow>,
}

impl ConfidenceTelemetry {
    /// Record a sample and return the updated row.
    pub fn record(&mut self, sample: TelemetrySample) -> &NodeTelemetryRow {
        let row = self.rows.entry(sample.key).or_default();
        row.record(sample.outcome, sample.observed_at);
        row
    }

    /// Return one row by exact key.
    #[must_use]
    pub fn get(&self, key: &NodeTelemetryKey) -> Option<&NodeTelemetryRow> {
        self.rows.get(key)
    }

    /// Return rows for one tenant/skill/version tuple.
    #[must_use]
    pub fn rows_for_skill(
        &self,
        tenant_id: &str,
        skill_id: &str,
        version: &str,
    ) -> Vec<(&NodeTelemetryKey, &NodeTelemetryRow)> {
        self.rows
            .iter()
            .filter(|(key, _)| {
                key.tenant_id == tenant_id && key.skill_id == skill_id && key.version == version
            })
            .collect()
    }

    /// Apply telemetry annotations to matching nodes in a graph.
    pub fn annotate_graph(
        &self,
        graph: &mut SkillGraph,
        tenant_id: &str,
        skill_id: &str,
        version: &str,
    ) {
        for node in &mut graph.nodes {
            let key = NodeTelemetryKey::new(tenant_id, skill_id, version, node.id.as_str());
            if let Some(row) = self.rows.get(&key) {
                let telemetry = row.to_annotations();
                node.annotations.confidence = telemetry.confidence;
                node.annotations.sample_count = telemetry.sample_count;
                node.annotations.success_rate = telemetry.success_rate;
                node.annotations.last_used_at = telemetry.last_used_at;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{Node, NodeId, NodeKind};

    fn key(tenant: &str, node: &str) -> NodeTelemetryKey {
        NodeTelemetryKey::new(tenant, "skill-1", "1.0.0", node)
    }

    fn graph() -> SkillGraph {
        SkillGraph {
            entry: NodeId::from("n1"),
            nodes: vec![
                Node {
                    id: NodeId::from("n1"),
                    label: "Fetch".into(),
                    kind: NodeKind::Fetch {
                        target: "https://example.com".into(),
                        method: "GET".into(),
                    },
                    annotations: NodeAnnotations::default(),
                },
                Node {
                    id: NodeId::from("n2"),
                    label: "Act".into(),
                    kind: NodeKind::Action {
                        instruction: "send summary".into(),
                    },
                    annotations: NodeAnnotations::default(),
                },
            ],
            ..SkillGraph::default()
        }
    }

    #[test]
    fn zero_sample_confidence_is_none() {
        let row = NodeTelemetryRow::default();
        assert_eq!(row.confidence(), None);
    }

    #[test]
    fn success_and_failure_samples_update_confidence() {
        let mut telemetry = ConfidenceTelemetry::default();
        telemetry.record(TelemetrySample::success(key("tenant-a", "n1")));
        telemetry.record(TelemetrySample::failure(key("tenant-a", "n1")));
        let row = telemetry.get(&key("tenant-a", "n1")).unwrap();
        assert_eq!(row.sample_count, 2);
        assert_eq!(row.success_count, 1);
        assert_eq!(row.failure_count, 1);
        assert_eq!(row.confidence(), Some(0.5));
    }

    #[test]
    fn rows_are_tenant_isolated() {
        let mut telemetry = ConfidenceTelemetry::default();
        telemetry.record(TelemetrySample::success(key("tenant-a", "n1")));
        telemetry.record(TelemetrySample::failure(key("tenant-b", "n1")));
        let a = telemetry.rows_for_skill("tenant-a", "skill-1", "1.0.0");
        let b = telemetry.rows_for_skill("tenant-b", "skill-1", "1.0.0");
        assert_eq!(a.len(), 1);
        assert_eq!(b.len(), 1);
        assert_eq!(a[0].1.confidence(), Some(1.0));
        assert_eq!(b[0].1.confidence(), Some(0.0));
    }

    #[test]
    fn annotate_graph_sets_confidence_fields_for_matching_nodes() {
        let mut telemetry = ConfidenceTelemetry::default();
        telemetry.record(TelemetrySample {
            key: key("tenant-a", "n1"),
            outcome: TelemetryOutcome::Success,
            observed_at: Some("2026-05-15T05:15:00Z".into()),
        });
        telemetry.record(TelemetrySample::failure(key("tenant-a", "n1")));
        let mut graph = graph();
        telemetry.annotate_graph(&mut graph, "tenant-a", "skill-1", "1.0.0");
        let annotations = &graph.nodes[0].annotations;
        assert_eq!(annotations.sample_count, Some(2));
        assert_eq!(annotations.confidence, Some(0.5));
        assert_eq!(annotations.success_rate, Some(0.5));
        assert!(annotations.last_used_at.is_some());
        assert_eq!(graph.nodes[1].annotations.confidence, None);
    }

    #[test]
    fn annotate_graph_does_not_cross_tenant_boundary() {
        let mut telemetry = ConfidenceTelemetry::default();
        telemetry.record(TelemetrySample::success(key("tenant-a", "n1")));
        let mut graph = graph();
        telemetry.annotate_graph(&mut graph, "tenant-b", "skill-1", "1.0.0");
        assert_eq!(graph.nodes[0].annotations.confidence, None);
    }
}
