//! Build Skill IR graphs from computer-use trajectories.
//!
//! Track L11 starts with the data the CU executor already records:
//! ordered action summaries plus an optional final observation. This
//! module keeps the conversion deterministic for v1 while preserving
//! confidence annotations so later LLM-backed abstraction can replace
//! the classifier without changing the graph shape.

use crate::ir::{
    Edge, EdgeCond, EdgeId, Node, NodeAnnotations, NodeId, NodeKind, NodePolicy, SkillGraph,
    SkillGraphMetadata,
};

/// Metadata source stamped on graphs synthesized from CU recordings.
pub const CU_RECORDING_SOURCE: &str = "cu-recording";

/// Nodes below this confidence should be surfaced as low-confidence
/// by consumers.
pub const LOW_CONFIDENCE_THRESHOLD: f64 = 0.6;

/// One CU trajectory ready to become a [`SkillGraph`].
///
/// # Example
/// ```
/// use argo_skill::ir::{graph_from_cu_trajectory, CuTrajectory};
/// let actions = vec!["navigate to https://example.com".to_string()];
/// let graph = graph_from_cu_trajectory(CuTrajectory {
///     name: "open-site",
///     version: "0.1.0",
///     description: Some("Open a site"),
///     actions: &actions,
///     final_text: None,
/// });
/// assert_eq!(graph.nodes.len(), 1);
/// ```
#[derive(Debug, Clone, Copy)]
pub struct CuTrajectory<'a> {
    /// Graph/skill name.
    pub name: &'a str,
    /// Graph/skill version.
    pub version: &'a str,
    /// Human-readable description.
    pub description: Option<&'a str>,
    /// Ordered CU action summaries.
    pub actions: &'a [String],
    /// Optional final observation emitted by the executor.
    pub final_text: Option<&'a str>,
}

/// Convert a CU trajectory into a linear Skill IR graph.
///
/// The current classifier is intentionally conservative:
/// navigation/open steps become [`NodeKind::Fetch`], approval/request
/// steps become [`NodeKind::AwaitApproval`], read/extract/summarize
/// steps become [`NodeKind::Transform`], and all remaining steps
/// become [`NodeKind::Action`]. Edges connect each node in order via
/// [`EdgeCond::OnSuccess`].
#[must_use]
pub fn graph_from_cu_trajectory(input: CuTrajectory<'_>) -> SkillGraph {
    let actions = if input.actions.is_empty() {
        vec![input
            .final_text
            .unwrap_or("no recorded computer-use action")
            .to_string()]
    } else {
        input.actions.to_vec()
    };

    let nodes: Vec<Node> = actions
        .iter()
        .enumerate()
        .map(|(idx, action)| build_node(idx, action))
        .collect();
    let edges = build_edges(nodes.len());
    SkillGraph {
        entry: nodes
            .first()
            .map(|node| node.id.clone())
            .unwrap_or_else(|| NodeId::from("n1")),
        nodes,
        edges,
        variables: vec![],
        metadata: SkillGraphMetadata {
            name: Some(input.name.to_owned()),
            version: Some(input.version.to_owned()),
            description: input.description.map(str::to_owned),
            source: Some(CU_RECORDING_SOURCE.into()),
        },
    }
}

fn build_node(idx: usize, action: &str) -> Node {
    let node_num = idx + 1;
    let kind = classify_action(action);
    let confidence = confidence_for(action, &kind);
    Node {
        id: NodeId::from(format!("n{node_num}")),
        label: label_for(action, node_num),
        kind,
        annotations: NodeAnnotations {
            confidence: Some(confidence),
            sample_count: Some(1),
            policy: Some(policy_for(action)),
            ..NodeAnnotations::default()
        },
    }
}

fn build_edges(node_count: usize) -> Vec<Edge> {
    (1..node_count)
        .map(|idx| Edge {
            id: EdgeId::from(format!("e{idx}")),
            from: NodeId::from(format!("n{idx}")),
            to: NodeId::from(format!("n{}", idx + 1)),
            cond: EdgeCond::OnSuccess,
        })
        .collect()
}

fn classify_action(action: &str) -> NodeKind {
    let lower = action.to_ascii_lowercase();
    if lower.contains("approval") || lower.contains("ask operator") || lower.contains("request") {
        return NodeKind::AwaitApproval {
            approver_role: "operator".into(),
            timeout_seconds: None,
        };
    }
    if lower.contains("navigate") || lower.contains("open ") || lower.contains("http") {
        return NodeKind::Fetch {
            target: extract_target(action).unwrap_or_else(|| action.to_owned()),
            method: "GET".into(),
        };
    }
    if lower.contains("extract")
        || lower.contains("summarize")
        || lower.contains("read ")
        || lower.contains("parse")
    {
        return NodeKind::Transform {
            template: action.to_owned(),
        };
    }
    NodeKind::Action {
        instruction: action.to_owned(),
    }
}

fn extract_target(action: &str) -> Option<String> {
    action
        .split_whitespace()
        .find(|part| part.starts_with("http://") || part.starts_with("https://"))
        .map(|part| part.trim_end_matches([',', '.']).to_owned())
}

fn confidence_for(action: &str, kind: &NodeKind) -> f64 {
    let lower = action.to_ascii_lowercase();
    if lower.contains("maybe") || lower.contains("uncertain") || lower.contains("unknown") {
        return 0.45;
    }
    match kind {
        NodeKind::Fetch { .. } if extract_target(action).is_some() => 0.92,
        NodeKind::Fetch { .. } => 0.7,
        NodeKind::AwaitApproval { .. } => 0.82,
        NodeKind::Transform { .. } => 0.76,
        NodeKind::Action { .. } => 0.68,
        _ => 0.6,
    }
}

fn policy_for(action: &str) -> NodePolicy {
    let lower = action.to_ascii_lowercase();
    if lower.contains("approval") || lower.contains("ask operator") {
        NodePolicy::Ask
    } else {
        NodePolicy::Auto
    }
}

fn label_for(action: &str, node_num: usize) -> String {
    let trimmed = action.trim();
    if trimmed.is_empty() {
        return format!("Step {node_num}");
    }
    let mut label: String = trimmed.chars().take(48).collect();
    if trimmed.chars().count() > 48 {
        label.push_str("...");
    }
    label
}
