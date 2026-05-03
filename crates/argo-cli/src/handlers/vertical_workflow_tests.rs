//! Unit tests for `handlers::vertical_workflow`. Extracted to a
//! sibling file (loaded via `#[path = "vertical_workflow_tests.rs"]
//! mod tests;`) so that `vertical_workflow.rs` stays under the
//! 500-LOC budget as the runner grows.
use super::*;
use argo_vertical::{Workflow, WorkflowHeader};

fn empty_set() -> std::collections::HashSet<String> {
    std::collections::HashSet::new()
}

fn local_test_principal() -> Principal {
    Principal::Local {
        os_user: "test".into(),
    }
}

#[tokio::test]
async fn step_outcome_for_unknown_kind_fails() {
    let step = WorkflowStep {
        name: "weird".into(),
        kind: "telepathy".into(),
        depends_on: vec![],
        skill: None,
        prompt: None,
        tool: None,
        args: None,
    };
    // The outer parse_workflow rejects unknown kinds, but
    // run_one_step is defensive in case future schema
    // additions land mid-flight.
    let dir = std::env::current_dir().unwrap();
    let registry = ToolRegistry::new();
    let engine = CapabilityEngine::with_defaults();
    let principal = local_test_principal();
    let outcome = run_one_step(
        &step, &registry, &engine, &principal, &dir, &empty_set(), &empty_set(),
    )
    .await;
    assert_eq!(outcome.status, "failed");
    assert!(outcome.message.contains("telepathy"));
}

#[tokio::test]
async fn skill_invoke_step_is_deferred() {
    let step = WorkflowStep {
        name: "scan".into(),
        kind: "skill_invoke".into(),
        depends_on: vec![],
        skill: Some("identify_invariant_violations".into()),
        prompt: None,
        tool: None,
        args: None,
    };
    let dir = std::env::current_dir().unwrap();
    let registry = ToolRegistry::new();
    let engine = CapabilityEngine::with_defaults();
    let principal = local_test_principal();
    let outcome = run_one_step(
        &step, &registry, &engine, &principal, &dir, &empty_set(), &empty_set(),
    )
    .await;
    assert_eq!(outcome.status, "deferred_no_llm");
    assert!(outcome.message.contains("identify_invariant_violations"));
}

#[tokio::test]
async fn prompt_step_is_deferred() {
    let step = WorkflowStep {
        name: "ask".into(),
        kind: "prompt".into(),
        depends_on: vec![],
        skill: None,
        prompt: Some("say hi".into()),
        tool: None,
        args: None,
    };
    let dir = std::env::current_dir().unwrap();
    let registry = ToolRegistry::new();
    let engine = CapabilityEngine::with_defaults();
    let principal = local_test_principal();
    let outcome = run_one_step(
        &step, &registry, &engine, &principal, &dir, &empty_set(), &empty_set(),
    )
    .await;
    assert_eq!(outcome.status, "deferred_no_llm");
}

#[tokio::test]
async fn tool_call_step_in_deny_list_is_blocked_before_dispatch() {
    // Even though the registry has no tools, the deny check
    // fires first — so we get `denied_by_vertical`, not
    // "no such tool". This locks in the deny-wins ordering.
    let step = WorkflowStep {
        name: "boom".into(),
        kind: "tool_call".into(),
        depends_on: vec![],
        skill: None,
        prompt: None,
        tool: Some("file_write".into()),
        args: None,
    };
    let dir = std::env::current_dir().unwrap();
    let registry = ToolRegistry::new();
    let engine = CapabilityEngine::with_defaults();
    let principal = local_test_principal();
    let mut denied = empty_set();
    denied.insert("file_write".into());
    let outcome = run_one_step(
        &step, &registry, &engine, &principal, &dir, &denied, &empty_set(),
    )
    .await;
    assert_eq!(outcome.status, "denied_by_vertical");
}

#[tokio::test]
async fn tool_call_step_in_approval_list_pauses_with_escalation_status() {
    // Approval gate fires before the registry is consulted —
    // the operator sees "needs human approval" with the tool
    // + args, NOT "no such tool".
    let step = WorkflowStep {
        name: "ship".into(),
        kind: "tool_call".into(),
        depends_on: vec![],
        skill: None,
        prompt: None,
        tool: Some("delegate_task".into()),
        args: None,
    };
    let dir = std::env::current_dir().unwrap();
    let registry = ToolRegistry::new();
    let engine = CapabilityEngine::with_defaults();
    let principal = local_test_principal();
    let mut needs_approval = empty_set();
    needs_approval.insert("delegate_task".into());
    let outcome = run_one_step(
        &step, &registry, &engine, &principal, &dir, &empty_set(), &needs_approval,
    )
    .await;
    assert_eq!(outcome.status, "escalation_required");
    assert!(
        outcome.message.contains("delegate_task"),
        "got: {}",
        outcome.message
    );
    assert!(
        outcome.message.contains("human approval"),
        "got: {}",
        outcome.message
    );
}

#[tokio::test]
async fn tool_call_step_denied_wins_over_approval_when_both_listed() {
    // A manifest declaring a tool as both denied AND
    // approval-required gets the harder answer (deny). This
    // locks in the precedence so an operator-mistake of
    // double-listing can never accidentally weaken the
    // platform's hard "never" boundary.
    let step = WorkflowStep {
        name: "ambiguous".into(),
        kind: "tool_call".into(),
        depends_on: vec![],
        skill: None,
        prompt: None,
        tool: Some("file_write".into()),
        args: None,
    };
    let dir = std::env::current_dir().unwrap();
    let registry = ToolRegistry::new();
    let engine = CapabilityEngine::with_defaults();
    let principal = local_test_principal();
    let mut denied = empty_set();
    denied.insert("file_write".into());
    let mut needs_approval = empty_set();
    needs_approval.insert("file_write".into());
    let outcome = run_one_step(
        &step, &registry, &engine, &principal, &dir, &denied, &needs_approval,
    )
    .await;
    assert_eq!(outcome.status, "denied_by_vertical");
}

#[tokio::test]
async fn tool_call_step_missing_tool_field_fails() {
    let step = WorkflowStep {
        name: "noop".into(),
        kind: "tool_call".into(),
        depends_on: vec![],
        skill: None,
        prompt: None,
        tool: None,
        args: None,
    };
    let dir = std::env::current_dir().unwrap();
    let registry = ToolRegistry::new();
    let engine = CapabilityEngine::with_defaults();
    let principal = local_test_principal();
    let outcome = run_one_step(
        &step, &registry, &engine, &principal, &dir, &empty_set(), &empty_set(),
    )
    .await;
    assert_eq!(outcome.status, "failed");
    assert!(outcome.message.contains("missing `tool`"));
}

#[test]
fn workflow_types_are_publicly_constructible() {
    // Lock the re-exports so the runner can construct headers
    // + workflows from outside the argo-vertical crate.
    let _wh = WorkflowHeader {
        name: "x".into(),
        description: String::new(),
    };
    let _wf = Workflow {
        schema_version: "1".into(),
        workflow: _wh,
        steps: vec![],
    };
}
