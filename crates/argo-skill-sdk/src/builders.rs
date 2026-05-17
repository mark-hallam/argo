use std::collections::HashSet;

use argo_skill::ir::{
    BranchArm, Edge, EdgeCond, EdgeId, Node, NodeId, NodeKind, SkillGraph, SkillGraphMetadata,
    SubSkillRef, Variable, VariableScope,
};

use crate::builders_inputs::{BuilderNodeOptions, SkillBuilderInput};
use crate::SkillBuildError;

/// Create a fluent builder with empty metadata.
///
/// # Example
/// ```
/// use argo_skill_sdk::create_skill;
/// let graph = create_skill().action("a", "A", "go").build().unwrap();
/// assert_eq!(graph.entry.as_str(), "a");
/// ```
pub fn create_skill() -> SkillBuilder {
    SkillBuilder::default()
}

/// Create a branch arm without a match pattern.
pub fn branch_arm(label: impl Into<String>) -> BranchArm {
    BranchArm {
        label: label.into(),
        pattern: None,
    }
}

/// Create a branch arm with a match pattern.
pub fn branch_arm_pattern(label: impl Into<String>, pattern: impl Into<String>) -> BranchArm {
    BranchArm {
        label: label.into(),
        pattern: Some(pattern.into()),
    }
}

/// Fluent builder for canonical [`SkillGraph`] values.
#[derive(Debug, Clone, Default)]
pub struct SkillBuilder {
    nodes: Vec<Node>,
    edges: Vec<Edge>,
    variables: Vec<Variable>,
    entry_id: Option<NodeId>,
    metadata: SkillGraphMetadata,
    errors: Vec<SkillBuildError>,
}

impl SkillBuilder {
    /// Start a builder with explicit metadata.
    pub fn new(input: SkillBuilderInput) -> Self {
        Self {
            metadata: input.into(),
            ..Self::default()
        }
    }

    /// Add an action node.
    pub fn action(
        self,
        id: impl Into<NodeId>,
        label: impl Into<String>,
        instruction: impl Into<String>,
    ) -> Self {
        self.action_with(id, label, instruction, BuilderNodeOptions::default())
    }

    /// Add an action node with options.
    pub fn action_with(
        self,
        id: impl Into<NodeId>,
        label: impl Into<String>,
        instruction: impl Into<String>,
        options: BuilderNodeOptions,
    ) -> Self {
        self.add_node(
            id,
            label,
            NodeKind::Action {
                instruction: instruction.into(),
            },
            options,
        )
    }

    /// Add a fetch node with the default method.
    pub fn fetch(
        self,
        id: impl Into<NodeId>,
        label: impl Into<String>,
        target: impl Into<String>,
    ) -> Self {
        self.fetch_method(id, label, target, "")
    }

    /// Add a fetch node with an explicit method.
    pub fn fetch_method(
        self,
        id: impl Into<NodeId>,
        label: impl Into<String>,
        target: impl Into<String>,
        method: impl Into<String>,
    ) -> Self {
        self.add_node(
            id,
            label,
            NodeKind::Fetch {
                target: target.into(),
                method: method.into(),
            },
            BuilderNodeOptions::default(),
        )
    }

    /// Add a transform node.
    pub fn transform(
        self,
        id: impl Into<NodeId>,
        label: impl Into<String>,
        template: impl Into<String>,
    ) -> Self {
        self.add_node(
            id,
            label,
            NodeKind::Transform {
                template: template.into(),
            },
            BuilderNodeOptions::default(),
        )
    }

    /// Add a condition node.
    pub fn condition(
        self,
        id: impl Into<NodeId>,
        label: impl Into<String>,
        expression: impl Into<String>,
    ) -> Self {
        self.add_node(
            id,
            label,
            NodeKind::Condition {
                expression: expression.into(),
            },
            BuilderNodeOptions::default(),
        )
    }

    /// Add an approval node without a timeout.
    pub fn approval(
        self,
        id: impl Into<NodeId>,
        label: impl Into<String>,
        approver_role: impl Into<String>,
    ) -> Self {
        self.approval_timeout(id, label, approver_role, None)
    }

    /// Add an approval node with an optional timeout in seconds.
    pub fn approval_timeout(
        self,
        id: impl Into<NodeId>,
        label: impl Into<String>,
        approver_role: impl Into<String>,
        timeout_seconds: Option<u64>,
    ) -> Self {
        self.add_node(
            id,
            label,
            NodeKind::AwaitApproval {
                approver_role: approver_role.into(),
                timeout_seconds,
            },
            BuilderNodeOptions::default(),
        )
    }

    /// Add a branch node.
    pub fn branch(
        self,
        id: impl Into<NodeId>,
        label: impl Into<String>,
        arms: Vec<BranchArm>,
    ) -> Self {
        self.add_node(
            id,
            label,
            NodeKind::Branch { arms },
            BuilderNodeOptions::default(),
        )
    }

    /// Add a sub-skill call node.
    pub fn sub_skill(
        self,
        id: impl Into<NodeId>,
        label: impl Into<String>,
        target: SubSkillRef,
    ) -> Self {
        self.add_node(
            id,
            label,
            NodeKind::SubSkillCall { target },
            BuilderNodeOptions::default(),
        )
    }

    /// Add a loop node.
    pub fn loop_over(
        self,
        id: impl Into<NodeId>,
        label: impl Into<String>,
        iter_var: impl Into<String>,
        body: Vec<NodeId>,
    ) -> Self {
        self.add_node(
            id,
            label,
            NodeKind::Loop {
                iter_var: iter_var.into(),
                body,
            },
            BuilderNodeOptions::default(),
        )
    }

    /// Add a try/catch node.
    pub fn try_catch(
        self,
        id: impl Into<NodeId>,
        label: impl Into<String>,
        try_body: Vec<NodeId>,
        catch_body: Vec<NodeId>,
    ) -> Self {
        self.add_node(
            id,
            label,
            NodeKind::TryCatch {
                try_body,
                catch_body,
            },
            BuilderNodeOptions::default(),
        )
    }

    /// Add an unconditional edge.
    pub fn edge(
        self,
        id: impl Into<EdgeId>,
        from: impl Into<NodeId>,
        to: impl Into<NodeId>,
    ) -> Self {
        self.edge_when(id, from, to, EdgeCond::Always)
    }

    /// Add an edge with an explicit transition condition.
    pub fn edge_when(
        mut self,
        id: impl Into<EdgeId>,
        from: impl Into<NodeId>,
        to: impl Into<NodeId>,
        cond: EdgeCond,
    ) -> Self {
        let id = id.into();
        if self.edges.iter().any(|edge| edge.id == id) {
            self.errors
                .push(SkillBuildError::DuplicateEdge(id.as_str().into()));
        } else {
            self.edges.push(Edge {
                id,
                from: from.into(),
                to: to.into(),
                cond,
            });
        }
        self
    }

    /// Add a graph-scoped variable.
    pub fn variable(self, name: impl Into<String>, var_type: impl Into<String>) -> Self {
        self.variable_scoped(name, var_type, VariableScope::Graph)
    }

    /// Add a variable with an explicit scope.
    pub fn variable_scoped(
        mut self,
        name: impl Into<String>,
        var_type: impl Into<String>,
        scope: VariableScope,
    ) -> Self {
        let name = name.into();
        if self.variables.iter().any(|var| var.name == name) {
            self.errors.push(SkillBuildError::DuplicateVariable(name));
        } else {
            self.variables.push(Variable {
                name,
                var_type: var_type.into(),
                scope,
            });
        }
        self
    }

    /// Select the graph entry node.
    pub fn start(mut self, id: impl Into<NodeId>) -> Self {
        self.entry_id = Some(id.into());
        self
    }

    /// Build and validate the graph.
    pub fn build(mut self) -> Result<SkillGraph, SkillBuildError> {
        if let Some(error) = self.errors.pop() {
            return Err(error);
        }
        let entry = self
            .entry_id
            .or_else(|| self.nodes.first().map(|node| node.id.clone()))
            .unwrap_or_default();
        validate_skill_graph(SkillGraph {
            entry,
            nodes: self.nodes,
            edges: self.edges,
            variables: self.variables,
            metadata: self.metadata,
        })
    }

    fn add_node(
        mut self,
        id: impl Into<NodeId>,
        label: impl Into<String>,
        kind: NodeKind,
        options: BuilderNodeOptions,
    ) -> Self {
        let id = id.into();
        if self.nodes.iter().any(|node| node.id == id) {
            self.errors
                .push(SkillBuildError::DuplicateNode(id.as_str().into()));
        } else {
            if self.entry_id.is_none() {
                self.entry_id = Some(id.clone());
            }
            self.nodes.push(Node {
                id,
                label: label.into(),
                kind,
                annotations: options.annotations,
            });
        }
        self
    }
}

/// Validate a graph against the SDK's authoring invariants.
pub fn validate_skill_graph(graph: SkillGraph) -> Result<SkillGraph, SkillBuildError> {
    let node_ids: HashSet<NodeId> = graph.nodes.iter().map(|node| node.id.clone()).collect();
    if graph.entry.as_str().is_empty() || !node_ids.contains(&graph.entry) {
        let entry = if graph.entry.as_str().is_empty() {
            "(empty)".to_string()
        } else {
            graph.entry.as_str().to_string()
        };
        return Err(SkillBuildError::MissingEntry(entry));
    }
    for edge in &graph.edges {
        if !node_ids.contains(&edge.from) || !node_ids.contains(&edge.to) {
            return Err(SkillBuildError::MissingEdgeNode(edge.id.as_str().into()));
        }
    }
    for node in &graph.nodes {
        validate_node(node)?;
    }
    Ok(graph)
}

fn validate_node(node: &Node) -> Result<(), SkillBuildError> {
    if let Some(confidence) = node.annotations.confidence {
        if !(0.0..=1.0).contains(&confidence) {
            return Err(SkillBuildError::InvalidConfidence(node.id.as_str().into()));
        }
    }
    match &node.kind {
        NodeKind::Branch { arms } if arms.is_empty() => {
            Err(SkillBuildError::EmptyBranch(node.id.as_str().into()))
        }
        NodeKind::Loop { body, .. } if body.is_empty() => {
            Err(SkillBuildError::EmptyLoop(node.id.as_str().into()))
        }
        NodeKind::TryCatch {
            try_body,
            catch_body,
        } if try_body.is_empty() || catch_body.is_empty() => {
            Err(SkillBuildError::EmptyTryCatch(node.id.as_str().into()))
        }
        _ => Ok(()),
    }
}
