//! Edge-case coverage for [`crate::workflow`].
//!
//! Authored under brief `eval-argo-vertical-loader-coverage` to
//! deepen test surface beyond the cases already covered in
//! `workflow.rs`'s inline tests. The most important findings are
//! the `#[ignore]`'d cycle-detection tests at the bottom: today
//! the validator only checks that `depends_on` names point at
//! existing steps — it does NOT detect cycles. The runner walks
//! steps in declaration order, so a cycle is not an immediate
//! runtime hazard, but a future ordering-graph upgrade would
//! infinite-loop. See PR body for the fix-brief name.
use super::{parse_workflow, validate_workflow, Workflow, WorkflowHeader, WorkflowStep};

fn step(name: &str, kind: &str) -> WorkflowStep {
    WorkflowStep {
        name: name.to_string(),
        kind: kind.to_string(),
        depends_on: Vec::new(),
        skill: if kind == "skill_invoke" {
            Some("dummy_skill".to_string())
        } else {
            None
        },
        prompt: if kind == "prompt" {
            Some("hi".to_string())
        } else {
            None
        },
        tool: if kind == "tool_call" {
            Some("file_read".to_string())
        } else {
            None
        },
        args: None,
    }
}

fn workflow_with_steps(steps: Vec<WorkflowStep>) -> Workflow {
    Workflow {
        schema_version: "1".to_string(),
        workflow: WorkflowHeader {
            name: "test".to_string(),
            description: String::new(),
        },
        steps,
    }
}

/// `workflow.name` consisting entirely of whitespace is treated
/// the same as empty — caught at validate, not parse, because
/// TOML is happy to round-trip a quoted "   ".
#[test]
fn rejects_whitespace_only_workflow_name() {
    let src = "schema_version = \"1\"\n[workflow]\nname=\"   \"\n\
               [[step]]\nname=\"x\"\nkind=\"prompt\"\nprompt=\"hi\"\n";
    let err = parse_workflow(src).expect_err("whitespace-only name must fail");
    assert!(format!("{err}").contains("must not be empty"));
}

/// Missing `schema_version` is a parse error from the
/// `#[serde(deny_unknown_fields)]` shape.
#[test]
fn rejects_missing_schema_version() {
    let src = "[workflow]\nname=\"x\"\n\
               [[step]]\nname=\"a\"\nkind=\"prompt\"\nprompt=\"hi\"\n";
    let err = parse_workflow(src).expect_err("missing schema_version");
    assert!(format!("{err}").contains("workflow"));
}

/// Empty `schema_version = ""` is rejected by the version check
/// (not the deny_unknown_fields shape).
#[test]
fn rejects_empty_schema_version_string() {
    let src = "schema_version = \"\"\n[workflow]\nname=\"x\"\n\
               [[step]]\nname=\"a\"\nkind=\"prompt\"\nprompt=\"hi\"\n";
    let err = parse_workflow(src).expect_err("empty schema_version");
    let msg = format!("{err}");
    assert!(msg.contains("schema_version"), "got: {msg}");
}

/// `[workflow]` section entirely missing is a parse error
/// (required by the struct shape).
#[test]
fn rejects_missing_workflow_header() {
    let src = "schema_version = \"1\"\n";
    let err = parse_workflow(src).expect_err("missing [workflow]");
    assert!(format!("{err}").contains("workflow"));
}

/// A step that lists the same dependency twice today flows
/// through validation. Locks current behaviour so a future
/// dedup-check upgrade is a deliberate decision, not silent.
#[test]
fn duplicate_dependency_name_currently_passes() {
    let mut a = step("a", "prompt");
    let mut b = step("b", "prompt");
    b.depends_on = vec!["a".to_string(), "a".to_string()];
    a.depends_on = Vec::new();
    let wf = workflow_with_steps(vec![a, b]);
    validate_workflow(&wf).expect("duplicate dep names accepted today");
}

/// `[[step]]` whose `kind=skill_invoke` and a literally empty
/// `skill = ""` is caught by the validator just like a missing
/// field — covers the trim/empty branch in `validate_step_kind`.
#[test]
fn rejects_skill_invoke_with_empty_skill_string() {
    let src = "schema_version = \"1\"\n[workflow]\nname=\"x\"\n\
               [[step]]\nname=\"a\"\nkind=\"skill_invoke\"\nskill=\"\"\n";
    let err = parse_workflow(src).expect_err("empty skill must fail");
    assert!(format!("{err}").contains("no `skill"));
}

/// Same — empty `prompt = ""` for kind=prompt.
#[test]
fn rejects_prompt_with_whitespace_only_prompt_string() {
    let src = "schema_version = \"1\"\n[workflow]\nname=\"x\"\n\
               [[step]]\nname=\"a\"\nkind=\"prompt\"\nprompt=\"   \"\n";
    let err = parse_workflow(src).expect_err("whitespace prompt must fail");
    assert!(format!("{err}").contains("no `prompt"));
}

/// Same — empty `tool = ""` for kind=tool_call.
#[test]
fn rejects_tool_call_with_empty_tool_string() {
    let src = "schema_version = \"1\"\n[workflow]\nname=\"x\"\n\
               [[step]]\nname=\"a\"\nkind=\"tool_call\"\ntool=\"\"\n";
    let err = parse_workflow(src).expect_err("empty tool must fail");
    assert!(format!("{err}").contains("no `tool"));
}

/// Step kind comparison is case-sensitive — `Prompt` ≠ `prompt`.
/// Locks the parser's behaviour against accidental case-folding.
#[test]
fn step_kind_is_case_sensitive() {
    let src = "schema_version = \"1\"\n[workflow]\nname=\"x\"\n\
               [[step]]\nname=\"a\"\nkind=\"Prompt\"\nprompt=\"hi\"\n";
    let err = parse_workflow(src).expect_err("mixed-case kind rejected");
    assert!(format!("{err}").contains("unknown kind 'Prompt'"));
}

/// Unknown subsection keys (e.g. typoed `prommpt` instead of
/// `prompt`) are caught by `#[serde(deny_unknown_fields)]` on
/// `WorkflowStep`.
#[test]
fn rejects_typoed_field_inside_step() {
    let src = "schema_version = \"1\"\n[workflow]\nname=\"x\"\n\
               [[step]]\nname=\"a\"\nkind=\"prompt\"\nprommpt=\"oops\"\n";
    let err = parse_workflow(src).expect_err("typoed field rejected");
    assert!(format!("{err}").contains("prommpt") || format!("{err}").contains("unknown"));
}

// ─────────────────────────────────────────────────────────────
// FINDINGS-ONLY: cycle detection is NOT implemented today.
// ─────────────────────────────────────────────────────────────
// `validate_workflow` only checks that every name in
// `depends_on` resolves to an existing step. It does not build
// the dependency DAG, so a graph with cycles passes validation.
// The runner today walks steps in declaration order — so a
// cycle is benign at runtime. But the workflow.rs docstring
// explicitly flags "An ordering-graph upgrade in Phase 5+" for
// `depends_on`; that upgrade would infinite-loop on these
// inputs unless cycle detection lands first.
//
// These tests are `#[ignore]`'d and will start passing as soon
// as cycle detection is added. Fix-brief: `claude-vertical-
// workflow-cycle-detection` (see PR body).

/// FINDINGS-ONLY: a step that lists ITSELF as a dependency is
/// not detected today. Should be rejected after the fix-brief
/// lands.
#[test]
#[ignore = "cycle detection not implemented yet — see claude-vertical-workflow-cycle-detection"]
fn findings_only_self_dependency_should_be_rejected() {
    let mut a = step("a", "prompt");
    a.depends_on = vec!["a".to_string()];
    let wf = workflow_with_steps(vec![a]);
    let err = validate_workflow(&wf).expect_err("self-cycle should fail");
    assert!(format!("{err}").to_lowercase().contains("cycle"));
}

/// FINDINGS-ONLY: a 2-node cycle A→B→A is not detected today.
#[test]
#[ignore = "cycle detection not implemented yet — see claude-vertical-workflow-cycle-detection"]
fn findings_only_two_node_cycle_should_be_rejected() {
    let mut a = step("a", "prompt");
    let mut b = step("b", "prompt");
    a.depends_on = vec!["b".to_string()];
    b.depends_on = vec!["a".to_string()];
    let wf = workflow_with_steps(vec![a, b]);
    let err = validate_workflow(&wf).expect_err("2-cycle should fail");
    assert!(format!("{err}").to_lowercase().contains("cycle"));
}

/// FINDINGS-ONLY: a 3-node cycle A→B→C→A is not detected today.
#[test]
#[ignore = "cycle detection not implemented yet — see claude-vertical-workflow-cycle-detection"]
fn findings_only_three_node_cycle_should_be_rejected() {
    let mut a = step("a", "prompt");
    let mut b = step("b", "prompt");
    let mut c = step("c", "prompt");
    a.depends_on = vec!["c".to_string()];
    b.depends_on = vec!["a".to_string()];
    c.depends_on = vec!["b".to_string()];
    let wf = workflow_with_steps(vec![a, b, c]);
    let err = validate_workflow(&wf).expect_err("3-cycle should fail");
    assert!(format!("{err}").to_lowercase().contains("cycle"));
}

/// LOCK-IN: current behaviour — a 2-node cycle parses cleanly.
/// This test fails when cycle detection lands, signalling that
/// the lock-in test above should be removed in the same PR.
#[test]
fn current_behaviour_two_node_cycle_passes_validation() {
    let mut a = step("a", "prompt");
    let mut b = step("b", "prompt");
    a.depends_on = vec!["b".to_string()];
    b.depends_on = vec!["a".to_string()];
    let wf = workflow_with_steps(vec![a, b]);
    validate_workflow(&wf)
        .expect("today a 2-cycle is accepted; remove this test once detection lands");
}
