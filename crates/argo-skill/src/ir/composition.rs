//! Composition helpers for [`NodeKind::SubSkillCall`].
//!
//! The IR schema already has a `SubSkillCall` node. This module adds
//! the backend primitive that validates a composed skill graph before a
//! runtime or canvas caller executes it: resolve each referenced graph,
//! report the parent→child call graph, and reject missing targets or
//! cycles.

use crate::ir::{NodeKind, SkillGraph, SubSkillRef};
use std::collections::BTreeSet;

/// Resolves a sub-skill reference to its current IR graph.
pub trait SubSkillResolver {
    /// Return the graph for `target`, or `None` when the skill/version
    /// is not installed or is not visible to the caller.
    fn resolve(&self, target: &SubSkillRef) -> Option<SkillGraph>;
}

/// One directed parent→child composition edge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompositionEdge {
    /// Parent skill id.
    pub from_skill_id: String,
    /// Parent skill version.
    pub from_version: String,
    /// Node id in the parent graph that made the sub-skill call.
    pub via_node_id: String,
    /// Child skill id.
    pub to_skill_id: String,
    /// Child skill version.
    pub to_version: String,
}

/// Validated composition call graph.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CompositionReport {
    /// Skills reached during validation, including the root. Order is
    /// deterministic depth-first discovery order.
    pub skills: Vec<SubSkillRef>,
    /// Parent→child call edges.
    pub edges: Vec<CompositionEdge>,
    /// Maximum sub-skill nesting depth. A root-only graph has depth 0.
    pub max_depth: usize,
}

/// Composition validation failure.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CompositionError {
    /// The root graph cannot be resolved.
    #[error("missing root skill {skill_id}@{version}")]
    MissingRoot {
        /// Skill id.
        skill_id: String,
        /// Version.
        version: String,
    },
    /// A sub-skill call points at an unavailable graph.
    #[error("missing sub-skill {skill_id}@{version} from {from_skill_id}@{from_version}")]
    MissingSubSkill {
        /// Parent skill id.
        from_skill_id: String,
        /// Parent version.
        from_version: String,
        /// Missing skill id.
        skill_id: String,
        /// Missing version.
        version: String,
    },
    /// The composition contains a cycle.
    #[error("sub-skill cycle detected: {path}")]
    Cycle {
        /// Human-readable cycle path.
        path: String,
    },
}

/// Return every sub-skill call directly declared by `graph`.
#[must_use]
pub fn sub_skill_calls(graph: &SkillGraph) -> Vec<(&str, &SubSkillRef)> {
    graph
        .nodes
        .iter()
        .filter_map(|node| match &node.kind {
            NodeKind::SubSkillCall { target } => Some((node.id.as_str(), target)),
            _ => None,
        })
        .collect()
}

/// Resolve and validate a root skill's full composition tree.
pub fn compose_skill_graphs(
    root: &SubSkillRef,
    resolver: &dyn SubSkillResolver,
) -> Result<CompositionReport, CompositionError> {
    let root_graph = resolver
        .resolve(root)
        .ok_or_else(|| CompositionError::MissingRoot {
            skill_id: root.skill_id.clone(),
            version: root.version.clone(),
        })?;
    let mut report = CompositionReport::default();
    let mut stack = Vec::new();
    let mut visited = BTreeSet::new();
    walk(
        root,
        &root_graph,
        resolver,
        &mut stack,
        &mut visited,
        &mut report,
        0,
    )?;
    Ok(report)
}

fn walk(
    current: &SubSkillRef,
    graph: &SkillGraph,
    resolver: &dyn SubSkillResolver,
    stack: &mut Vec<SubSkillRef>,
    visited: &mut BTreeSet<String>,
    report: &mut CompositionReport,
    depth: usize,
) -> Result<(), CompositionError> {
    let current_key = ref_key(current);
    if let Some(pos) = stack.iter().position(|seen| ref_key(seen) == current_key) {
        let mut path: Vec<String> = stack[pos..].iter().map(ref_key).collect();
        path.push(current_key);
        return Err(CompositionError::Cycle {
            path: path.join(" -> "),
        });
    }
    if !visited.insert(current_key) {
        return Ok(());
    }
    stack.push(current.clone());
    report.skills.push(current.clone());
    report.max_depth = report.max_depth.max(depth);

    for (node_id, target) in sub_skill_calls(graph) {
        report.edges.push(CompositionEdge {
            from_skill_id: current.skill_id.clone(),
            from_version: current.version.clone(),
            via_node_id: node_id.to_string(),
            to_skill_id: target.skill_id.clone(),
            to_version: target.version.clone(),
        });
        let target_graph =
            resolver
                .resolve(target)
                .ok_or_else(|| CompositionError::MissingSubSkill {
                    from_skill_id: current.skill_id.clone(),
                    from_version: current.version.clone(),
                    skill_id: target.skill_id.clone(),
                    version: target.version.clone(),
                })?;
        walk(
            target,
            &target_graph,
            resolver,
            stack,
            visited,
            report,
            depth + 1,
        )?;
    }
    stack.pop();
    Ok(())
}

fn ref_key(target: &SubSkillRef) -> String {
    format!("{}@{}", target.skill_id, target.version)
}

#[cfg(test)]
#[path = "composition_tests.rs"]
mod tests;
