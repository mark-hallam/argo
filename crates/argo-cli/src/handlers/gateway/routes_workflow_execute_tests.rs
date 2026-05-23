//! Unit tests for `routes_workflow_execute`. Integration-level
//! contracts (anonymous → 401, reader → 403, owner-with-workflow →
//! 200 + audit row, cross-tenant → 404) live in
//! `crates/argo-cli/tests/routes_workflow_execute_test.rs` — those
//! spin up `argo gateway start` and exercise the full surface over
//! TCP. The tests here cover the pure helpers (path parse, body
//! parse, vertical → DAG conversion) so a regression on parsing
//! shape lands without booting the gateway.
use super::*;

#[test]
fn parses_execute_path_with_vertical_and_name() {
    let req = "POST /api/v1/verticals/pr_checker/workflows/standard_review/execute HTTP/1.1\r\nHost: localhost\r\n\r\n";
    let (vertical, name) = parse_execute_path(req).expect("parse");
    assert_eq!(vertical, "pr_checker");
    assert_eq!(name, "standard_review");
}

#[test]
fn parses_execute_path_with_query_string() {
    let req = "POST /api/v1/verticals/pr/workflows/wf/execute?trace=1 HTTP/1.1\r\n\r\n";
    let (vertical, name) = parse_execute_path(req).expect("parse with query");
    assert_eq!(vertical, "pr");
    assert_eq!(name, "wf");
}

#[test]
fn rejects_path_with_wrong_suffix() {
    let req = "POST /api/v1/verticals/pr/workflows/wf/test_runs HTTP/1.1\r\n\r\n";
    assert!(
        parse_execute_path(req).is_none(),
        "the /test_runs suffix must fall through to its own dispatcher"
    );
    let req = "POST /api/v1/verticals/pr/workflows/wf HTTP/1.1\r\n\r\n";
    assert!(parse_execute_path(req).is_none(), "no /execute suffix");
}

#[test]
fn rejects_non_post_verbs() {
    let req = "GET /api/v1/verticals/pr/workflows/wf/execute HTTP/1.1\r\n\r\n";
    assert!(parse_execute_path(req).is_none());
}

#[test]
fn body_parser_accepts_empty_body() {
    let (inputs, idem) = parse_request_body("").expect("empty body");
    assert!(inputs.is_object() && inputs.as_object().unwrap().is_empty());
    assert!(idem.is_none());
}

#[test]
fn body_parser_extracts_inputs_and_idempotency_key() {
    let body = r#"{"inputs":{"k":"v"},"idempotency_key":"req-42"}"#;
    let (inputs, idem) = parse_request_body(body).expect("parse");
    assert_eq!(inputs.get("k"), Some(&serde_json::Value::String("v".into())));
    assert_eq!(idem.as_deref(), Some("req-42"));
}

#[test]
fn body_parser_rejects_non_string_idempotency_key() {
    let body = r#"{"idempotency_key":42}"#;
    let err = parse_request_body(body).unwrap_err();
    assert!(err.contains("idempotency_key"), "got: {err}");
}

#[test]
fn body_parser_rejects_malformed_json() {
    let body = "{not json";
    let err = parse_request_body(body).unwrap_err();
    assert!(err.contains("parse json body"), "got: {err}");
}

#[test]
fn vertical_workflow_to_dag_carries_step_names_and_depends_on() {
    let toml = r#"schema_version = "1"
[workflow]
name = "wf"
description = "test"

[[step]]
name = "first"
kind = "tool_call"
tool = "echo"
args = {msg = "hi"}

[[step]]
name = "second"
kind = "skill_invoke"
skill = "summarize"
depends_on = ["first"]
"#;
    let parsed = argo_vertical::parse_workflow(toml).expect("parse");
    let dag = vertical_workflow_to_dag(&parsed);
    assert_eq!(dag.id, "wf");
    assert_eq!(dag.tasks.len(), 2);
    assert_eq!(dag.tasks[0].id, "first");
    assert_eq!(
        dag.tasks[0].inputs.get("tool"),
        Some(&serde_json::Value::String("echo".into()))
    );
    assert_eq!(dag.tasks[1].id, "second");
    assert_eq!(dag.tasks[1].worker, "summarize");
    assert_eq!(dag.tasks[1].depends_on, vec!["first".to_string()]);
}
