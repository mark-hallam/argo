//! Skill IR — canonical structured representation of a skill.
//!
//! Every skill in Skycot — regardless of how it entered the system
//! (watch, demo, natural-language teaching, canvas editor, brief, or
//! Ghostwriter generation) — normalizes to a [`SkillGraph`]. The IR is
//! the substrate the canvas reads, the runtime executes, and the
//! diff/version-history surfaces compare against.
//!
//! This module is **pure types only** — no storage, no runtime, no UI.
//! Storage lives in `argo-skill::ir::store` (L2 brief); canvas
//! rendering and runtime execution come in later phases.
//!
//! Track L brief: `streams/L1-skill-ir-schema.md`.
//! Track L overview: `streams/L0-skill-canvas-track-overview.md`.

use serde::{Deserialize, Serialize};

/// Stable identifier for a [`Node`] within a [`SkillGraph`]. Opaque
/// string — typically a short slug like `"n1"` or a UUID; uniqueness
/// is per-graph, not global.
///
/// # Example
/// ```
/// use argo_skill::ir::NodeId;
/// let id: NodeId = "node-fetch-invoice".into();
/// assert_eq!(id.as_str(), "node-fetch-invoice");
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NodeId(String);

impl NodeId {
    /// Borrow the inner identifier.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for NodeId {
    fn from(s: &str) -> Self {
        Self(s.to_owned())
    }
}

impl From<String> for NodeId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

/// Stable identifier for an [`Edge`] within a [`SkillGraph`].
///
/// # Example
/// ```
/// use argo_skill::ir::EdgeId;
/// let id: EdgeId = "e1".into();
/// assert_eq!(id.as_str(), "e1");
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EdgeId(String);

impl EdgeId {
    /// Borrow the inner identifier.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for EdgeId {
    fn from(s: &str) -> Self {
        Self(s.to_owned())
    }
}

impl From<String> for EdgeId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

/// The nine canonical kinds of node a skill graph can contain, locked
/// at L1 — every consumer (canvas, runtime, gateway, diff engine)
/// switches on this enum. Variants are tagged via the `"type"` JSON
/// field for TS interop.
///
/// # Example
/// ```
/// use argo_skill::ir::NodeKind;
/// let action = NodeKind::Action { instruction: "Send email".into() };
/// let json = serde_json::to_string(&action).unwrap();
/// assert!(json.contains("\"type\":\"action\""));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum NodeKind {
    /// Single deterministic action — invoke a tool or run an
    /// instruction. The runtime workhorse variant.
    Action {
        /// Natural-language or template instruction the worker
        /// executes (resolved against the bound tool, if any).
        instruction: String,
    },
    /// Boolean predicate that gates a downstream branch.
    Condition {
        /// Predicate expression (engine-specific syntax; v1 is opaque
        /// string, evaluated by the runtime).
        expression: String,
    },
    /// Multi-way fan-out by labelled arms. Arms are evaluated in
    /// declared order; the first match wins.
    Branch {
        /// Ordered arms — each names a label and (optionally) a
        /// pattern the runtime matches against the upstream output.
        arms: Vec<BranchArm>,
    },
    /// Pauses the run until a human approver responds (or timeout).
    AwaitApproval {
        /// Logical role expected to approve (e.g. `"finance-lead"`).
        approver_role: String,
        /// Optional timeout in seconds; `None` = wait indefinitely.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        timeout_seconds: Option<u64>,
    },
    /// Read external data (HTTP, MCP resource, file). The runtime
    /// resolves the target against the bound tool.
    Fetch {
        /// Target URL or MCP resource URI.
        target: String,
        /// HTTP method (`"GET"`, `"POST"`, ...). For non-HTTP
        /// targets the runtime interprets per-tool.
        #[serde(default)]
        method: String,
    },
    /// Pure data transform — map / filter / template.
    Transform {
        /// Transform template (engine-specific; v1 opaque).
        template: String,
    },
    /// Calls another skill by reference. Enables composition.
    SubSkillCall {
        /// Skill reference (id + version).
        target: SubSkillRef,
    },
    /// Iterates over a collection variable, executing a sub-sequence
    /// of nodes per iteration. The body is referenced by node ids so
    /// loops compose cleanly with branches and try/catch.
    Loop {
        /// Variable name iterated over.
        iter_var: String,
        /// Node ids executed per iteration.
        body: Vec<NodeId>,
    },
    /// Wraps a sub-sequence in a try/catch; on failure, runs the
    /// catch sequence instead of propagating the error.
    TryCatch {
        /// Nodes executed in the try arm.
        try_body: Vec<NodeId>,
        /// Nodes executed on failure.
        catch_body: Vec<NodeId>,
    },
}

/// One arm of a [`NodeKind::Branch`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BranchArm {
    /// Human-readable arm label.
    pub label: String,
    /// Optional match pattern (regex / glob / equality — runtime
    /// decides). `None` = catch-all.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
}

/// Reference to another skill by stable id + version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubSkillRef {
    /// Hex-encoded skill id (matches `skill.id` in storage).
    pub skill_id: String,
    /// Skill version string, e.g. `"1.0.0"`.
    pub version: String,
}

/// Per-node observability + policy metadata. Populated by telemetry
/// (L12) and editor (L7); the canvas reads these fields directly.
///
/// All fields are optional so a freshly authored graph can declare a
/// node without ever having executed it.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeAnnotations {
    /// Smoothed confidence ∈ \[0.0, 1.0\] — typically
    /// `success_count / sample_count` with a Bayesian prior.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
    /// Number of executions observed for this node.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sample_count: Option<u64>,
    /// RFC-3339 timestamp of the most recent execution.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_used_at: Option<String>,
    /// Fraction ∈ \[0.0, 1.0\] of executions that succeeded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub success_rate: Option<f64>,
    /// Execution policy — autonomous vs ask-operator.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy: Option<NodePolicy>,
    /// MCP tool bound to this node, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_binding: Option<ToolBinding>,
}

/// Whether the runtime may execute the node autonomously or must
/// prompt the operator first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NodePolicy {
    /// Run without prompting.
    Auto,
    /// Pause and ask the operator.
    Ask,
}

/// Identifies the MCP tool an [`NodeKind::Action`] or
/// [`NodeKind::Fetch`] node should invoke.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolBinding {
    /// MCP server name (matches manifest declaration).
    pub mcp_server: String,
    /// Tool name within that server.
    pub tool_name: String,
}

/// Edge transition condition — determines when the runtime should
/// follow this edge after the source node completes.
///
/// # Example
/// ```
/// use argo_skill::ir::EdgeCond;
/// assert_eq!(
///     serde_json::to_string(&EdgeCond::OnSuccess).unwrap(),
///     "{\"kind\":\"on-success\"}"
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum EdgeCond {
    /// Always follow this edge (the unconditional / default case).
    Always,
    /// Follow only when the source node completed successfully.
    OnSuccess,
    /// Follow only when the source node failed.
    OnFailure,
    /// Follow when the source's output matches the supplied pattern.
    OnPattern {
        /// Match pattern (runtime-defined — typically a regex or
        /// branch-arm label).
        pattern: String,
    },
}

/// A directed transition between two nodes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Edge {
    /// Stable id within the graph.
    pub id: EdgeId,
    /// Source node id.
    pub from: NodeId,
    /// Destination node id.
    pub to: NodeId,
    /// When to follow the edge.
    pub cond: EdgeCond,
}

/// A node — kind-specific config, plus shared id/label/annotations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    /// Stable id within the graph.
    pub id: NodeId,
    /// Human-readable label rendered on the canvas.
    pub label: String,
    /// Kind-specific configuration.
    pub kind: NodeKind,
    /// Observability + policy metadata.
    #[serde(default)]
    pub annotations: NodeAnnotations,
}

/// A scoped input or output variable that flows through edges.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Variable {
    /// Variable identifier (case-sensitive).
    pub name: String,
    /// Type tag (engine-specific; v1: `"string"`, `"number"`,
    /// `"bool"`, `"json"`).
    pub var_type: String,
    /// Visibility scope.
    pub scope: VariableScope,
}

/// Where a variable is visible.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum VariableScope {
    /// Visible across the whole graph.
    Graph,
    /// Visible only inside one node's body (e.g. a loop iterator).
    Node,
}

/// Graph-level metadata (skill identity + provenance hints). Distinct
/// from storage-layer metadata; this travels with the canonical
/// JSON and participates in the hash chain.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillGraphMetadata {
    /// Skill name (matches `skill.name` in storage).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Skill version (matches `skill.version`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Free-form description of the skill's purpose.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// How this graph was synthesized — `"hand-authored"`,
    /// `"ghostwriter"`, `"cu-recording"`, `"backfill"`, etc.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// The canonical structured representation of a skill — nodes, edges,
/// variables, and an entry point. Every consumer in Track L (canvas,
/// gateway, runtime, diff engine, Ghostwriter) reads or writes this
/// shape.
///
/// JSON serialization uses `camelCase` field names for TS interop;
/// the canonical-hash form is sorted-key + whitespace-stripped — see
/// [`crate::ir::canonical_json`] and [`crate::ir::canonical_hash_hex`].
///
/// # Example
/// ```
/// use argo_skill::ir::{SkillGraph, Node, NodeKind, NodeId, NodeAnnotations};
/// let g = SkillGraph {
///     entry: NodeId::from("n1"),
///     nodes: vec![Node {
///         id: NodeId::from("n1"),
///         label: "start".into(),
///         kind: NodeKind::Action { instruction: "say hi".into() },
///         annotations: NodeAnnotations::default(),
///     }],
///     edges: vec![],
///     variables: vec![],
///     metadata: Default::default(),
/// };
/// assert_eq!(g.nodes.len(), 1);
/// ```
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillGraph {
    /// Id of the entry node. The runtime starts here.
    #[serde(default)]
    pub entry: NodeId,
    /// All nodes in declaration order.
    pub nodes: Vec<Node>,
    /// All edges in declaration order.
    pub edges: Vec<Edge>,
    /// Graph-level variables.
    #[serde(default)]
    pub variables: Vec<Variable>,
    /// Graph-level metadata.
    #[serde(default)]
    pub metadata: SkillGraphMetadata,
}
