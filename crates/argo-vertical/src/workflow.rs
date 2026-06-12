//! Workflow TOML schema for the vertical-template platform
//! (#658 — Phase 5, foundation).
//!
//! A **workflow** is a named, ordered sequence of steps that a
//! vertical can declare in its `[workflows].files = [...]` list.
//! Each step says "do this thing"; the runtime (lands in #659)
//! walks the steps in order and threads results between them.
//!
//! This module ships ONLY the schema + parser + validator. The
//! runner is intentionally out of scope so this PR stays
//! independently revertable: parse a `.workflow.toml`, return
//! a strongly-typed [`Workflow`], reject malformed shapes. No
//! execution side-effects.
//!
//! ## Example workflow file
//!
//! ```toml
//! schema_version = "1"
//!
//! [workflow]
//! name        = "check_invariants"
//! description = "Run the invariant scan over a PR diff and post the summary."
//!
//! [[step]]
//! name = "scan"
//! kind = "skill_invoke"
//! skill = "identify_invariant_violations"
//!
//! [[step]]
//! name = "summarize"
//! kind = "skill_invoke"
//! skill = "post_review_summary"
//! depends_on = ["scan"]
//! ```
//!
//! ## Supported step kinds (v1)
//!
//! - `skill_invoke` — instructs the runtime to add the named
//!   skill to the active set for one turn.
//! - `prompt` — a literal user-message template injected as the
//!   next user turn.
//! - `tool_call` — a direct tool invocation with literal JSON
//!   arguments. Cap-checked + deny-list-checked like any other
//!   tool dispatch.
//!
//! Unknown kinds are rejected at parse time so the runner never
//! has to handle "what does that mean?".
use argo_types::{ArgoError, ArgoResult};
use serde::{Deserialize, Serialize};

/// Workflow schema version this argo build supports. Future
/// schema bumps land additively or via `argo vertical migrate`.
pub const WORKFLOW_SCHEMA_VERSION: &str = "1";

/// Top-level shape of a `.workflow.toml`. Two required sections:
/// `schema_version` + `[workflow]`. At least one `[[step]]` is
/// required by the validator.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Workflow {
    /// Must equal [`WORKFLOW_SCHEMA_VERSION`].
    pub schema_version: String,
    /// Identity + metadata.
    pub workflow: WorkflowHeader,
    /// Ordered sequence of steps. The runner walks these in
    /// order; `depends_on` is informational today, an
    /// ordering-graph upgrade in Phase 5+.
    #[serde(default, rename = "step")]
    pub steps: Vec<WorkflowStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(deny_unknown_fields)]
pub struct WorkflowHeader {
    /// Workflow name. Must be unique across the vertical's
    /// workflows.
    pub name: String,
    /// Optional one-paragraph description.
    #[serde(default)]
    pub description: String,
}

/// One step in a workflow. The `kind` field discriminates the
/// payload — see the `kind`-specific fields below.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WorkflowStep {
    /// Step name. Must be unique within the workflow.
    pub name: String,
    /// One of: `skill_invoke`, `prompt`, `tool_call`.
    pub kind: String,
    /// Names of earlier steps this step depends on. Today this
    /// is metadata only; the runner walks steps in declaration
    /// order. A future Phase 5+ upgrade can use this to build a
    /// DAG.
    #[serde(default)]
    pub depends_on: Vec<String>,

    // kind = "skill_invoke"
    /// The skill name to attach for this step's turn.
    #[serde(default)]
    pub skill: Option<String>,

    // kind = "prompt"
    /// The literal user message text to inject as the next turn.
    #[serde(default)]
    pub prompt: Option<String>,

    // kind = "tool_call"
    /// The tool name to invoke directly.
    #[serde(default)]
    pub tool: Option<String>,
    /// Literal JSON arguments to pass to the tool.
    #[serde(default)]
    pub args: Option<serde_json::Value>,
}

/// Parse a workflow TOML string.
pub fn parse_workflow(src: &str) -> ArgoResult<Workflow> {
    let wf: Workflow =
        toml::from_str(src).map_err(|e| ArgoError::Schema(format!("workflow: {e}")))?;
    if wf.schema_version != WORKFLOW_SCHEMA_VERSION {
        return Err(ArgoError::Schema(format!(
            "workflow schema_version '{}' is not supported by this argo build (expected '{}')",
            wf.schema_version, WORKFLOW_SCHEMA_VERSION
        )));
    }
    validate_workflow(&wf)?;
    Ok(wf)
}

/// Cross-step validation. Called by [`parse_workflow`]; exposed
/// publicly so the manifest validator can re-check on demand.
pub fn validate_workflow(wf: &Workflow) -> ArgoResult<()> {
    if wf.workflow.name.trim().is_empty() {
        return Err(ArgoError::Schema(
            "[workflow].name must not be empty".to_string(),
        ));
    }
    if wf.steps.is_empty() {
        return Err(ArgoError::Schema(format!(
            "workflow '{}' has zero [[step]] entries — at least one is required",
            wf.workflow.name
        )));
    }
    let mut seen_names: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for step in &wf.steps {
        if !seen_names.insert(step.name.as_str()) {
            return Err(ArgoError::Schema(format!(
                "workflow '{}': duplicate step name '{}'",
                wf.workflow.name, step.name
            )));
        }
        validate_step_kind(&wf.workflow.name, step)?;
        for dep in &step.depends_on {
            if !wf.steps.iter().any(|s| s.name == *dep) {
                return Err(ArgoError::Schema(format!(
                    "workflow '{}': step '{}' depends on unknown step '{}'",
                    wf.workflow.name, step.name, dep
                )));
            }
        }
    }
    validate_no_cycles(wf)?;
    Ok(())
}

/// Reject `depends_on` graphs that contain a cycle.
///
/// Edges run step → dependency. A depth-first search marks nodes
/// white (unvisited) / gray (on the current path) / black (fully
/// explored); encountering a gray node means a back-edge, i.e. a
/// cycle. Both a self-dependency (`A → A`) and a multi-node cycle
/// (`A → B → A`) are rejected with the offending path named. This
/// is pure validation — it runs unconditionally at parse time and
/// has no effect on how a valid workflow loads.
fn validate_no_cycles(wf: &Workflow) -> ArgoResult<()> {
    use std::collections::HashMap;
    let adj: HashMap<&str, &[String]> = wf
        .steps
        .iter()
        .map(|s| (s.name.as_str(), s.depends_on.as_slice()))
        .collect();
    // 0 = white (unvisited), 1 = gray (on path), 2 = black (done).
    let mut state: HashMap<&str, u8> = HashMap::new();
    let mut path: Vec<&str> = Vec::new();
    for step in &wf.steps {
        if let Some(cycle) = dfs_find_cycle(step.name.as_str(), &adj, &mut state, &mut path) {
            return Err(ArgoError::Schema(format!(
                "workflow '{}': dependency cycle detected: {}",
                wf.workflow.name,
                cycle.join(" -> ")
            )));
        }
    }
    Ok(())
}

/// DFS helper for [`validate_no_cycles`]. Returns the cycle path
/// (from the repeated node back to itself) on a back-edge, else
/// `None`. Recursion depth is bounded by the step count.
fn dfs_find_cycle<'a>(
    node: &'a str,
    adj: &std::collections::HashMap<&'a str, &'a [String]>,
    state: &mut std::collections::HashMap<&'a str, u8>,
    path: &mut Vec<&'a str>,
) -> Option<Vec<&'a str>> {
    match state.get(node) {
        Some(2) => return None,
        Some(1) => {
            // Back-edge: slice the current path from the first
            // occurrence of `node` and close the loop on itself.
            let start = path.iter().position(|n| *n == node).unwrap_or(0);
            let mut cycle: Vec<&str> = path[start..].to_vec();
            cycle.push(node);
            return Some(cycle);
        }
        _ => {}
    }
    state.insert(node, 1);
    path.push(node);
    if let Some(deps) = adj.get(node) {
        for dep in deps.iter() {
            if let Some(cycle) = dfs_find_cycle(dep.as_str(), adj, state, path) {
                return Some(cycle);
            }
        }
    }
    path.pop();
    state.insert(node, 2);
    None
}

fn validate_step_kind(workflow_name: &str, step: &WorkflowStep) -> ArgoResult<()> {
    match step.kind.as_str() {
        "skill_invoke" => {
            if step.skill.as_deref().unwrap_or("").trim().is_empty() {
                return Err(ArgoError::Schema(format!(
                    "workflow '{workflow_name}': step '{}' has kind=skill_invoke but no `skill = \"…\"`",
                    step.name
                )));
            }
        }
        "prompt" => {
            if step.prompt.as_deref().unwrap_or("").trim().is_empty() {
                return Err(ArgoError::Schema(format!(
                    "workflow '{workflow_name}': step '{}' has kind=prompt but no `prompt = \"…\"`",
                    step.name
                )));
            }
        }
        "tool_call" => {
            if step.tool.as_deref().unwrap_or("").trim().is_empty() {
                return Err(ArgoError::Schema(format!(
                    "workflow '{workflow_name}': step '{}' has kind=tool_call but no `tool = \"…\"`",
                    step.name
                )));
            }
        }
        other => {
            return Err(ArgoError::Schema(format!(
                "workflow '{workflow_name}': step '{}' has unknown kind '{other}' (expected: skill_invoke | prompt | tool_call)",
                step.name
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "workflow_edge_tests.rs"]
mod edge_tests;

#[cfg(test)]
mod tests {
    use super::*;

    const MIN: &str = r#"
schema_version = "1"

[workflow]
name = "demo"

[[step]]
name = "one"
kind = "prompt"
prompt = "say hi"
"#;

    #[test]
    fn parses_minimal_workflow() {
        let wf = parse_workflow(MIN).unwrap();
        assert_eq!(wf.workflow.name, "demo");
        assert_eq!(wf.steps.len(), 1);
        assert_eq!(wf.steps[0].kind, "prompt");
    }

    #[test]
    fn parses_full_workflow_with_dependencies() {
        let src = r#"
schema_version = "1"

[workflow]
name = "check"
description = "scan + summarize"

[[step]]
name = "scan"
kind = "skill_invoke"
skill = "identify_invariant_violations"

[[step]]
name = "tool_check"
kind = "tool_call"
tool = "file_read"
args = { path = "README.md" }

[[step]]
name = "summarize"
kind = "skill_invoke"
skill = "post_review_summary"
depends_on = ["scan", "tool_check"]
"#;
        let wf = parse_workflow(src).unwrap();
        assert_eq!(wf.steps.len(), 3);
        assert_eq!(wf.steps[2].depends_on, vec!["scan", "tool_check"]);
    }

    #[test]
    fn rejects_wrong_schema_version() {
        let bad = MIN.replace("\"1\"", "\"99\"");
        let err = parse_workflow(&bad).unwrap_err();
        assert!(format!("{err}").contains("schema_version '99'"));
    }

    #[test]
    fn rejects_zero_steps() {
        let src = "schema_version = \"1\"\n[workflow]\nname = \"empty\"\n";
        let err = parse_workflow(src).unwrap_err();
        assert!(format!("{err}").contains("zero [[step]]"));
    }

    #[test]
    fn rejects_duplicate_step_names() {
        let src = "schema_version = \"1\"\n[workflow]\nname=\"d\"\n\
                   [[step]]\nname=\"x\"\nkind=\"prompt\"\nprompt=\"a\"\n\
                   [[step]]\nname=\"x\"\nkind=\"prompt\"\nprompt=\"b\"\n";
        let err = parse_workflow(src).unwrap_err();
        assert!(format!("{err}").contains("duplicate step name 'x'"));
    }

    #[test]
    fn rejects_unknown_step_kind() {
        let src = "schema_version = \"1\"\n[workflow]\nname=\"d\"\n\
                   [[step]]\nname=\"weird\"\nkind=\"telepathy\"\n";
        let err = parse_workflow(src).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("unknown kind 'telepathy'"), "got: {msg}");
    }

    #[test]
    fn rejects_skill_invoke_without_skill_field() {
        let src = "schema_version = \"1\"\n[workflow]\nname=\"d\"\n\
                   [[step]]\nname=\"x\"\nkind=\"skill_invoke\"\n";
        let err = parse_workflow(src).unwrap_err();
        assert!(format!("{err}").contains("no `skill"));
    }

    #[test]
    fn rejects_prompt_without_prompt_field() {
        let src = "schema_version = \"1\"\n[workflow]\nname=\"d\"\n\
                   [[step]]\nname=\"x\"\nkind=\"prompt\"\n";
        let err = parse_workflow(src).unwrap_err();
        assert!(format!("{err}").contains("no `prompt"));
    }

    #[test]
    fn rejects_tool_call_without_tool_field() {
        let src = "schema_version = \"1\"\n[workflow]\nname=\"d\"\n\
                   [[step]]\nname=\"x\"\nkind=\"tool_call\"\n";
        let err = parse_workflow(src).unwrap_err();
        assert!(format!("{err}").contains("no `tool"));
    }

    #[test]
    fn rejects_dependency_on_unknown_step() {
        let src = "schema_version = \"1\"\n[workflow]\nname=\"d\"\n\
                   [[step]]\nname=\"x\"\nkind=\"prompt\"\nprompt=\"hi\"\n\
                   depends_on=[\"ghost\"]\n";
        let err = parse_workflow(src).unwrap_err();
        assert!(format!("{err}").contains("unknown step 'ghost'"));
    }

    #[test]
    fn rejects_unknown_top_level_key() {
        let bad = format!("{MIN}\nbogus = 1\n");
        let err = parse_workflow(&bad).unwrap_err();
        assert!(format!("{err}").contains("workflow"));
    }
}
