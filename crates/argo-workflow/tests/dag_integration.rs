//! Integration tests for `argo-workflow` (Stream
//! `claude-workflow-dag`). The two scenarios from the brief:
//! 3-task linear chain + 4-task diamond. Both wire YAML →
//! validate → executor with the `MockRunner`.
use argo_workflow::{parse_yaml_str, ExecutionRequest, Executor, MockRunner, TaskVerdict};

#[tokio::test]
async fn three_task_linear_chain_runs_in_order_with_output_binding() {
    let yaml = r#"
id: linear
tasks:
  - id: a
    worker: w
    inputs:
      greeting: hello
  - id: b
    worker: w
    inputs:
      upstream: "${tasks.a.out.echo}"
    depends_on: [a]
  - id: c
    worker: w
    inputs:
      upstream: "${tasks.b.out.echo}"
    depends_on: [b]
"#;
    let workflow = parse_yaml_str(yaml).expect("parses");
    let exec = Executor::new(MockRunner::echo());
    let report = exec
        .execute(ExecutionRequest::new(workflow))
        .await
        .expect("executes");
    assert!(report.ok, "linear chain should succeed: {report:?}");
    assert_eq!(report.task_verdicts.len(), 3);

    // The MockRunner echoes inputs; b's `upstream` field should
    // resolve to a's `echo` output (which is "a") and ride through
    // to c.
    match report.task_verdicts.get("b").unwrap() {
        TaskVerdict::Success { outputs } => {
            assert_eq!(
                outputs.get("upstream"),
                Some(&serde_json::Value::String("a".into()))
            );
        }
        other => panic!("expected b success, got {other:?}"),
    }
    match report.task_verdicts.get("c").unwrap() {
        TaskVerdict::Success { outputs } => {
            // c's upstream field references b's `echo`, which is "b".
            assert_eq!(
                outputs.get("upstream"),
                Some(&serde_json::Value::String("b".into()))
            );
        }
        other => panic!("expected c success, got {other:?}"),
    }
}

#[tokio::test]
async fn four_task_diamond_runs_with_fanin() {
    // a → b
    // a → c
    // b → d
    // c → d
    let yaml = r#"
id: diamond
tasks:
  - id: a
    worker: w
  - id: b
    worker: w
    depends_on: [a]
  - id: c
    worker: w
    depends_on: [a]
  - id: d
    worker: w
    depends_on: [b, c]
"#;
    let workflow = parse_yaml_str(yaml).expect("parses");
    let plan = Executor::new(MockRunner::echo()).dry_run(&workflow).unwrap();
    // a must come first; d must come last; b and c are in
    // between, in some order.
    assert_eq!(plan.task_order.first().map(String::as_str), Some("a"));
    assert_eq!(plan.task_order.last().map(String::as_str), Some("d"));
    assert!(plan.task_order.contains(&"b".to_string()));
    assert!(plan.task_order.contains(&"c".to_string()));

    let exec = Executor::new(MockRunner::echo());
    let report = exec
        .execute(ExecutionRequest::new(workflow))
        .await
        .expect("executes");
    assert!(report.ok);
    assert_eq!(report.task_verdicts.len(), 4);
    for id in ["a", "b", "c", "d"] {
        assert!(matches!(
            report.task_verdicts.get(id),
            Some(TaskVerdict::Success { .. })
        ));
    }
}
