use super::*;
use crate::ir::{Node, NodeAnnotations, NodeId};
use std::collections::BTreeMap;

#[derive(Default)]
struct Catalog {
    graphs: BTreeMap<String, SkillGraph>,
}

impl Catalog {
    fn insert(&mut self, id: &str, version: &str, graph: SkillGraph) {
        self.graphs.insert(key(id, version), graph);
    }
}

impl SubSkillResolver for Catalog {
    fn resolve(&self, target: &SubSkillRef) -> Option<SkillGraph> {
        self.graphs
            .get(&key(&target.skill_id, &target.version))
            .cloned()
    }
}

fn key(id: &str, version: &str) -> String {
    format!("{id}@{version}")
}

fn reference(id: &str) -> SubSkillRef {
    SubSkillRef {
        skill_id: id.into(),
        version: "1.0.0".into(),
    }
}

fn action_graph(label: &str) -> SkillGraph {
    SkillGraph {
        entry: NodeId::from("start"),
        nodes: vec![Node {
            id: NodeId::from("start"),
            label: label.into(),
            kind: NodeKind::Action {
                instruction: format!("run {label}"),
            },
            annotations: NodeAnnotations::default(),
        }],
        ..SkillGraph::default()
    }
}

fn call_graph(calls: &[(&str, &str)]) -> SkillGraph {
    SkillGraph {
        entry: NodeId::from("start"),
        nodes: calls
            .iter()
            .map(|(node_id, target)| Node {
                id: NodeId::from(*node_id),
                label: format!("call {target}"),
                kind: NodeKind::SubSkillCall {
                    target: reference(target),
                },
                annotations: NodeAnnotations::default(),
            })
            .collect(),
        ..SkillGraph::default()
    }
}

#[test]
fn sub_skill_calls_lists_direct_targets() {
    let graph = call_graph(&[("n1", "child-a"), ("n2", "child-b")]);
    let calls = sub_skill_calls(&graph);
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].0, "n1");
    assert_eq!(calls[0].1.skill_id, "child-a");
    assert_eq!(calls[1].0, "n2");
    assert_eq!(calls[1].1.skill_id, "child-b");
}

#[test]
fn three_skill_composition_reports_edges_and_depth() {
    let mut catalog = Catalog::default();
    catalog.insert("root", "1.0.0", call_graph(&[("n1", "middle")]));
    catalog.insert("middle", "1.0.0", call_graph(&[("n2", "leaf")]));
    catalog.insert("leaf", "1.0.0", action_graph("leaf"));

    let report = compose_skill_graphs(&reference("root"), &catalog).unwrap();
    assert_eq!(
        report
            .skills
            .iter()
            .map(|s| s.skill_id.as_str())
            .collect::<Vec<_>>(),
        vec!["root", "middle", "leaf"]
    );
    assert_eq!(report.edges.len(), 2);
    assert_eq!(report.max_depth, 2);
}

#[test]
fn shared_child_is_visited_once_but_edges_are_preserved() {
    let mut catalog = Catalog::default();
    catalog.insert(
        "root",
        "1.0.0",
        call_graph(&[("left", "shared"), ("right", "shared")]),
    );
    catalog.insert("shared", "1.0.0", action_graph("shared"));

    let report = compose_skill_graphs(&reference("root"), &catalog).unwrap();
    assert_eq!(report.skills.len(), 2);
    assert_eq!(report.edges.len(), 2);
}

#[test]
fn missing_sub_skill_is_rejected() {
    let mut catalog = Catalog::default();
    catalog.insert("root", "1.0.0", call_graph(&[("n1", "missing")]));

    let err = compose_skill_graphs(&reference("root"), &catalog).unwrap_err();
    assert!(matches!(
        err,
        CompositionError::MissingSubSkill { skill_id, .. } if skill_id == "missing"
    ));
}

#[test]
fn cycle_is_rejected_with_path() {
    let mut catalog = Catalog::default();
    catalog.insert("a", "1.0.0", call_graph(&[("to-b", "b")]));
    catalog.insert("b", "1.0.0", call_graph(&[("to-a", "a")]));

    let err = compose_skill_graphs(&reference("a"), &catalog).unwrap_err();
    assert!(matches!(err, CompositionError::Cycle { .. }));
    assert!(err.to_string().contains("a@1.0.0 -> b@1.0.0 -> a@1.0.0"));
}

#[test]
fn missing_root_is_rejected() {
    let catalog = Catalog::default();
    let err = compose_skill_graphs(&reference("root"), &catalog).unwrap_err();
    assert!(matches!(
        err,
        CompositionError::MissingRoot { skill_id, .. } if skill_id == "root"
    ));
}
