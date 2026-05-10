//! Tests for [`crate::dag`] — DAG topology validator + topo-sort.
//!
//! Extracted from `dag.rs` to keep that file under the 500-LOC
//! ceiling. Lives as a sibling `#[cfg(test)]` module rather than a
//! child of `dag` so the parent file stays focused on production
//! code.

#![cfg(test)]

use crate::dag::{Edge, TaskNode, Trigger, ValidationError, Workflow};
use std::collections::BTreeMap;

fn task(id: &str, deps: &[&str]) -> TaskNode {
    TaskNode {
        id: id.to_string(),
        worker: "w".into(),
        persona: None,
        inputs: BTreeMap::new(),
        outputs: BTreeMap::new(),
        depends_on: deps.iter().map(|s| s.to_string()).collect(),
        cache_key: None,
    }
}

#[test]
fn validate_empty_workflow_errors() {
    let w = Workflow {
        id: "x".into(),
        description: String::new(),
        tasks: vec![],
        edges: vec![],
    };
    assert_eq!(w.validate().err(), Some(ValidationError::Empty));
}

#[test]
fn validate_duplicate_task_id_errors() {
    let w = Workflow {
        id: "x".into(),
        description: String::new(),
        tasks: vec![task("a", &[]), task("a", &[])],
        edges: vec![],
    };
    match w.validate() {
        Err(ValidationError::DuplicateTaskId(s)) => assert_eq!(s, "a"),
        other => panic!("expected duplicate, got {other:?}"),
    }
}

#[test]
fn validate_unknown_depends_on_errors() {
    let w = Workflow {
        id: "x".into(),
        description: String::new(),
        tasks: vec![task("a", &["ghost"])],
        edges: vec![],
    };
    assert!(matches!(
        w.validate(),
        Err(ValidationError::UnknownTaskInDependsOn { .. })
    ));
}

#[test]
fn validate_unknown_edge_endpoint_errors() {
    let w = Workflow {
        id: "x".into(),
        description: String::new(),
        tasks: vec![task("a", &[])],
        edges: vec![Edge {
            from_task: "a".into(),
            to_task: "ghost".into(),
            trigger: Trigger::Success,
        }],
    };
    assert!(matches!(
        w.validate(),
        Err(ValidationError::UnknownTaskInEdge(_))
    ));
}

#[test]
fn validate_detects_simple_cycle() {
    let w = Workflow {
        id: "x".into(),
        description: String::new(),
        tasks: vec![task("a", &["b"]), task("b", &["a"])],
        edges: vec![],
    };
    assert!(matches!(w.validate(), Err(ValidationError::Cycle(_))));
}

#[test]
fn validate_detects_self_loop() {
    let w = Workflow {
        id: "x".into(),
        description: String::new(),
        tasks: vec![task("a", &["a"])],
        edges: vec![],
    };
    assert!(matches!(w.validate(), Err(ValidationError::Cycle(_))));
}

#[test]
fn validate_accepts_linear_chain() {
    let w = Workflow {
        id: "x".into(),
        description: String::new(),
        tasks: vec![task("a", &[]), task("b", &["a"]), task("c", &["b"])],
        edges: vec![],
    };
    let edges = w.validate().expect("valid");
    assert_eq!(edges.len(), 2);
    let order = w.topo_order().expect("topo");
    let ids: Vec<&str> = order.iter().map(|t| t.id.as_str()).collect();
    assert_eq!(ids, vec!["a", "b", "c"]);
}

#[test]
fn validate_accepts_diamond() {
    // a → b
    // a → c
    // b → d
    // c → d
    let w = Workflow {
        id: "x".into(),
        description: String::new(),
        tasks: vec![
            task("a", &[]),
            task("b", &["a"]),
            task("c", &["a"]),
            task("d", &["b", "c"]),
        ],
        edges: vec![],
    };
    let order = w.topo_order().expect("topo");
    let ids: Vec<&str> = order.iter().map(|t| t.id.as_str()).collect();
    assert_eq!(ids[0], "a");
    assert_eq!(ids[3], "d");
    // b and c are interchangeable but both before d.
    assert!(ids.iter().position(|i| *i == "b").unwrap() < 3);
    assert!(ids.iter().position(|i| *i == "c").unwrap() < 3);
}

#[test]
fn topo_order_handles_explicit_edges_too() {
    let w = Workflow {
        id: "x".into(),
        description: String::new(),
        tasks: vec![task("a", &[]), task("b", &[])],
        edges: vec![Edge {
            from_task: "a".into(),
            to_task: "b".into(),
            trigger: Trigger::Success,
        }],
    };
    let order = w.topo_order().unwrap();
    assert_eq!(
        order.iter().map(|t| t.id.as_str()).collect::<Vec<_>>(),
        vec!["a", "b"]
    );
}

#[test]
fn task_lookup_returns_node() {
    let w = Workflow {
        id: "x".into(),
        description: String::new(),
        tasks: vec![task("a", &[])],
        edges: vec![],
    };
    assert!(w.task("a").is_some());
    assert!(w.task("ghost").is_none());
}
