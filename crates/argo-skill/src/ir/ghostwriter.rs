//! Ghostwriter — `prompt → SkillGraph` synthesis (Track L brief: L6).
//!
//! The operator types a natural-language description of the skill they
//! want; this module turns that into a validated [`SkillGraph`] ready
//! for L2 storage and the L4 canvas. Closes the "blank-grid paralysis"
//! failure mode by ensuring a freshly-drafted skill always opens with
//! a populated canvas.
//!
//! Two collaborators drive the pipeline:
//! - The [`GhostwriterClient`] trait — narrow LLM abstraction. Real
//!   callers wire whichever model they have available (the brief asks
//!   for the strongest model regardless of the runtime provider); tests
//!   inject [`StubGhostwriterClient`] with canned responses.
//! - [`OfflineGhostwriterClient`] — deterministic, dependency-free
//!   default used by the gateway when no LLM client is wired in. It
//!   emits a 5-node template (fetch → transform → condition →
//!   await-approval → action) so the gateway always returns a valid IR
//!   for the v1 launch, well ahead of the model-routing wire-up.
//!
//! ## Pipeline
//!
//! 1. [`build_initial_prompt`](super::ghostwriter_prompts::build_initial_prompt)
//!    is sent to the client.
//! 2. The response is decoded as JSON and structurally validated
//!    against the L1 schema.
//! 3. On validation failure exactly one retry is issued with
//!    [`build_self_correction_prompt`](super::ghostwriter_prompts::build_self_correction_prompt).
//! 4. On second failure the call returns [`GhostwriterError::Validation`].
//!
//! The pipeline never persists; storing into L2 is the caller's
//! responsibility so the gateway can attach tenant / skill id / audit.

use crate::ir::ghostwriter_prompts::{build_initial_prompt, build_self_correction_prompt};
use crate::ir::types::{
    Edge, EdgeCond, EdgeId, Node, NodeAnnotations, NodeId, NodeKind, SkillGraph, SkillGraphMetadata,
};
use argo_types::{ArgoError, ArgoResult};
use async_trait::async_trait;
use std::collections::HashSet;

/// Source tag stamped on every IR produced by this module so downstream
/// surfaces (diff view, marketplace export) can tell a Ghostwriter
/// draft apart from a watched, taught, or backfilled skill.
pub const GHOSTWRITER_SOURCE: &str = "ghostwriter";

/// Narrow LLM abstraction. Production callers wrap whichever model
/// client they hold (per L0 §"Operator decisions" 3 the operator
/// expects the strongest available model to be used here regardless of
/// the worker's runtime provider). Tests inject a stub.
#[async_trait]
pub trait GhostwriterClient: Send + Sync {
    /// Send `prompt` to the model and return the raw response text
    /// (expected to be a JSON `SkillGraph`). Implementations MUST set
    /// temperature ≤ 0.2 so retries are deterministic-ish.
    async fn generate(&self, prompt: &str) -> ArgoResult<String>;
}

/// Errors specific to the Ghostwriter pipeline. Decode + validation
/// errors are surfaced separately so the gateway can pick the right
/// HTTP status; both fold into [`ArgoError::Provider`] when crossing
/// out of this module via [`ArgoResult`].
#[derive(Debug, thiserror::Error)]
pub enum GhostwriterError {
    /// Operator prompt was empty or whitespace-only.
    #[error("ghostwriter: operator prompt is empty")]
    EmptyPrompt,
    /// The model's JSON failed to deserialise as a `SkillGraph` on
    /// both attempts.
    #[error("ghostwriter: invalid JSON from model: {0}")]
    Decode(String),
    /// JSON parsed but the resulting `SkillGraph` failed L1 structural
    /// validation on both attempts.
    #[error("ghostwriter: graph failed validation: {0}")]
    Validation(String),
    /// The underlying LLM client failed (network, auth, timeout, …).
    /// Carries the inner provider message so the audit row can record
    /// the precise failure without re-rendering it.
    #[error("ghostwriter: LLM client failed: {0}")]
    Provider(String),
}

impl From<GhostwriterError> for ArgoError {
    fn from(e: GhostwriterError) -> Self {
        ArgoError::Provider(e.to_string())
    }
}

/// Validate structural invariants the L1 schema implies but
/// `serde_json` deserialization does not catch by itself. Surfaced as
/// `Err(String)` so the message can flow straight into the
/// self-correction retry prompt.
pub fn validate_graph(graph: &SkillGraph) -> Result<(), String> {
    if graph.nodes.is_empty() {
        return Err("graph has zero nodes".to_string());
    }
    let ids: HashSet<&str> = graph.nodes.iter().map(|n| n.id.as_str()).collect();
    if ids.len() != graph.nodes.len() {
        return Err("graph has duplicate node ids".to_string());
    }
    let entry = graph.entry.as_str();
    if entry.is_empty() {
        return Err("graph.entry is empty".to_string());
    }
    if !ids.contains(entry) {
        return Err(format!("graph.entry '{entry}' does not match any node id"));
    }
    let mut edge_ids: HashSet<&str> = HashSet::new();
    for edge in &graph.edges {
        if !edge_ids.insert(edge.id.as_str()) {
            return Err(format!("duplicate edge id '{}'", edge.id.as_str()));
        }
        if !ids.contains(edge.from.as_str()) {
            return Err(format!(
                "edge '{}' references unknown source node '{}'",
                edge.id.as_str(),
                edge.from.as_str()
            ));
        }
        if !ids.contains(edge.to.as_str()) {
            return Err(format!(
                "edge '{}' references unknown destination node '{}'",
                edge.id.as_str(),
                edge.to.as_str()
            ));
        }
    }
    Ok(())
}

fn decode_graph(raw: &str) -> Result<SkillGraph, String> {
    let trimmed = strip_code_fence(raw.trim());
    serde_json::from_str::<SkillGraph>(trimmed).map_err(|e| format!("decode SkillGraph: {e}"))
}

/// Models often wrap their JSON in a ```json fence; strip it before
/// decoding so the parser sees just the object.
fn strip_code_fence(raw: &str) -> &str {
    let trimmed = raw.trim();
    let without_open = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```"))
        .unwrap_or(trimmed)
        .trim_start_matches('\n');
    without_open
        .trim_end_matches('\n')
        .trim_end_matches("```")
        .trim()
}

/// Run the full Ghostwriter pipeline. Returns the validated graph on
/// success or a [`GhostwriterError`] on second-attempt failure.
pub async fn generate_from_prompt(
    operator_prompt: &str,
    llm: &dyn GhostwriterClient,
) -> Result<SkillGraph, GhostwriterError> {
    if operator_prompt.trim().is_empty() {
        return Err(GhostwriterError::EmptyPrompt);
    }
    let initial = build_initial_prompt(operator_prompt);
    let first = llm
        .generate(&initial)
        .await
        .map_err(|e| GhostwriterError::Provider(e.to_string()))?;
    match parse_and_validate(&first) {
        Ok(graph) => Ok(stamp_source(graph)),
        Err(err) => {
            let retry_prompt = build_self_correction_prompt(operator_prompt, &first, &err);
            let second = llm
                .generate(&retry_prompt)
                .await
                .map_err(|e| GhostwriterError::Provider(e.to_string()))?;
            match parse_and_validate(&second) {
                Ok(graph) => Ok(stamp_source(graph)),
                Err(retry_err) => Err(classify_failure(&retry_err, &second)),
            }
        }
    }
}

fn parse_and_validate(raw: &str) -> Result<SkillGraph, String> {
    let graph = decode_graph(raw)?;
    validate_graph(&graph)?;
    Ok(graph)
}

fn classify_failure(err: &str, raw: &str) -> GhostwriterError {
    if err.starts_with("decode SkillGraph:") {
        GhostwriterError::Decode(format!("{err} (raw bytes: {} chars)", raw.len()))
    } else {
        GhostwriterError::Validation(err.to_string())
    }
}

fn stamp_source(mut graph: SkillGraph) -> SkillGraph {
    if graph.metadata.source.is_none() {
        graph.metadata.source = Some(GHOSTWRITER_SOURCE.to_string());
    }
    graph
}

/// Deterministic offline default for the gateway. Renders the same
/// 5-node "fetch → transform → decide → approve → act" template with
/// the operator's description threaded into the labels. Used until a
/// real LLM client is wired in — guarantees the gateway never returns
/// a blank canvas, satisfying the L0 "canvas is never blank" invariant.
pub struct OfflineGhostwriterClient;

#[async_trait]
impl GhostwriterClient for OfflineGhostwriterClient {
    async fn generate(&self, prompt: &str) -> ArgoResult<String> {
        let description = extract_description(prompt);
        let graph = offline_template(&description);
        serde_json::to_string(&graph)
            .map_err(|e| ArgoError::Provider(format!("offline ghostwriter: {e}")))
    }
}

fn extract_description(prompt: &str) -> String {
    // Initial prompt is `Operator description:\n<<<\n{op}\n>>>\n…`.
    if let Some(rest) = prompt.split("<<<").nth(1) {
        if let Some(op) = rest.split(">>>").next() {
            return op.trim().to_string();
        }
    }
    prompt.trim().to_string()
}

/// Build the deterministic offline IR. Public so callers (and tests)
/// can render the same template without going through the trait.
#[must_use]
pub fn offline_template(description: &str) -> SkillGraph {
    let desc = if description.is_empty() {
        "Drafted skill"
    } else {
        description
    };
    let nodes = vec![
        Node {
            id: NodeId::from("n1-fetch"),
            label: "Fetch input".into(),
            kind: NodeKind::Fetch {
                target: format!("inputs://{}", slug(desc)),
                method: "GET".into(),
            },
            annotations: NodeAnnotations::default(),
        },
        Node {
            id: NodeId::from("n2-transform"),
            label: "Normalise records".into(),
            kind: NodeKind::Transform {
                template: format!("normalise({})", slug(desc)),
            },
            annotations: NodeAnnotations::default(),
        },
        Node {
            id: NodeId::from("n3-decide"),
            label: "Decide whether human review is needed".into(),
            kind: NodeKind::Condition {
                expression: "needs_review".into(),
            },
            annotations: NodeAnnotations::default(),
        },
        Node {
            id: NodeId::from("n4-approve"),
            label: "Await operator approval".into(),
            kind: NodeKind::AwaitApproval {
                approver_role: "operator".into(),
                timeout_seconds: Some(3600),
            },
            annotations: NodeAnnotations::default(),
        },
        Node {
            id: NodeId::from("n5-act"),
            label: "Carry out the action".into(),
            kind: NodeKind::Action {
                instruction: truncate(desc, 240),
            },
            annotations: NodeAnnotations::default(),
        },
    ];
    let edges = vec![
        Edge {
            id: EdgeId::from("e1"),
            from: NodeId::from("n1-fetch"),
            to: NodeId::from("n2-transform"),
            cond: EdgeCond::OnSuccess,
        },
        Edge {
            id: EdgeId::from("e2"),
            from: NodeId::from("n2-transform"),
            to: NodeId::from("n3-decide"),
            cond: EdgeCond::OnSuccess,
        },
        Edge {
            id: EdgeId::from("e3"),
            from: NodeId::from("n3-decide"),
            to: NodeId::from("n4-approve"),
            cond: EdgeCond::OnPattern {
                pattern: "needs_review".into(),
            },
        },
        Edge {
            id: EdgeId::from("e4"),
            from: NodeId::from("n4-approve"),
            to: NodeId::from("n5-act"),
            cond: EdgeCond::OnSuccess,
        },
        Edge {
            id: EdgeId::from("e5"),
            from: NodeId::from("n3-decide"),
            to: NodeId::from("n5-act"),
            cond: EdgeCond::OnFailure,
        },
    ];
    SkillGraph {
        entry: NodeId::from("n1-fetch"),
        nodes,
        edges,
        variables: vec![],
        metadata: SkillGraphMetadata {
            name: Some(short_title(desc)),
            version: Some("1.0.0".into()),
            description: Some(truncate(desc, 280)),
            source: Some(GHOSTWRITER_SOURCE.into()),
        },
    }
}

fn slug(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|p| !p.is_empty())
        .take(6)
        .collect::<Vec<_>>()
        .join("-")
}

fn short_title(s: &str) -> String {
    let truncated = truncate(s, 64);
    if truncated.is_empty() {
        "Drafted skill".to_string()
    } else {
        truncated
    }
}

fn truncate(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max_chars.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

#[cfg(test)]
#[path = "ghostwriter_tests.rs"]
mod tests;
