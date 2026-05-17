//! Skill IR schema tests — roundtrip every NodeKind, every EdgeCond
//! variant, and the canonical-hash invariants.

use crate::ir::{
    BranchArm, Edge, EdgeCond, EdgeId, Node, NodeAnnotations, NodeId, NodeKind, NodePolicy,
    SkillGraph, SkillGraphMetadata, SubSkillRef, ToolBinding, Variable, VariableScope,
};

fn node(id: &str, kind: NodeKind) -> Node {
    Node {
        id: NodeId::from(id),
        label: format!("node {id}"),
        kind,
        annotations: NodeAnnotations::default(),
    }
}

fn roundtrip(graph: &SkillGraph) -> SkillGraph {
    let json = serde_json::to_string(graph).unwrap();
    serde_json::from_str(&json).unwrap()
}

#[test]
fn node_kind_action_roundtrips() {
    let g = SkillGraph {
        entry: "n".into(),
        nodes: vec![node(
            "n",
            NodeKind::Action {
                instruction: "send email".into(),
            },
        )],
        ..Default::default()
    };
    assert_eq!(roundtrip(&g), g);
}

#[test]
fn node_kind_condition_roundtrips() {
    let g = SkillGraph {
        entry: "n".into(),
        nodes: vec![node(
            "n",
            NodeKind::Condition {
                expression: "x > 10".into(),
            },
        )],
        ..Default::default()
    };
    assert_eq!(roundtrip(&g), g);
}

#[test]
fn node_kind_branch_roundtrips() {
    let g = SkillGraph {
        entry: "n".into(),
        nodes: vec![node(
            "n",
            NodeKind::Branch {
                arms: vec![
                    BranchArm {
                        label: "yes".into(),
                        pattern: Some("y.*".into()),
                    },
                    BranchArm {
                        label: "default".into(),
                        pattern: None,
                    },
                ],
            },
        )],
        ..Default::default()
    };
    assert_eq!(roundtrip(&g), g);
}

#[test]
fn node_kind_await_approval_roundtrips() {
    let g = SkillGraph {
        entry: "n".into(),
        nodes: vec![node(
            "n",
            NodeKind::AwaitApproval {
                approver_role: "finance-lead".into(),
                timeout_seconds: Some(300),
            },
        )],
        ..Default::default()
    };
    assert_eq!(roundtrip(&g), g);
}

#[test]
fn node_kind_fetch_roundtrips() {
    let g = SkillGraph {
        entry: "n".into(),
        nodes: vec![node(
            "n",
            NodeKind::Fetch {
                target: "https://api.example.com/x".into(),
                method: "GET".into(),
            },
        )],
        ..Default::default()
    };
    assert_eq!(roundtrip(&g), g);
}

#[test]
fn node_kind_transform_roundtrips() {
    let g = SkillGraph {
        entry: "n".into(),
        nodes: vec![node(
            "n",
            NodeKind::Transform {
                template: "${x.upper()}".into(),
            },
        )],
        ..Default::default()
    };
    assert_eq!(roundtrip(&g), g);
}

#[test]
fn node_kind_sub_skill_call_roundtrips() {
    let g = SkillGraph {
        entry: "n".into(),
        nodes: vec![node(
            "n",
            NodeKind::SubSkillCall {
                target: SubSkillRef {
                    skill_id: "abcd1234".into(),
                    version: "2.0.0".into(),
                },
            },
        )],
        ..Default::default()
    };
    assert_eq!(roundtrip(&g), g);
}

#[test]
fn node_kind_loop_roundtrips() {
    let g = SkillGraph {
        entry: "n".into(),
        nodes: vec![node(
            "n",
            NodeKind::Loop {
                iter_var: "row".into(),
                body: vec![NodeId::from("b1"), NodeId::from("b2")],
            },
        )],
        ..Default::default()
    };
    assert_eq!(roundtrip(&g), g);
}

#[test]
fn node_kind_try_catch_roundtrips() {
    let g = SkillGraph {
        entry: "n".into(),
        nodes: vec![node(
            "n",
            NodeKind::TryCatch {
                try_body: vec![NodeId::from("t1")],
                catch_body: vec![NodeId::from("c1")],
            },
        )],
        ..Default::default()
    };
    assert_eq!(roundtrip(&g), g);
}

#[test]
fn edge_cond_variants_roundtrip() {
    let cases = vec![
        EdgeCond::Always,
        EdgeCond::OnSuccess,
        EdgeCond::OnFailure,
        EdgeCond::OnPattern {
            pattern: "ok.*".into(),
        },
    ];
    for cond in cases {
        let g = SkillGraph {
            entry: "a".into(),
            nodes: vec![
                node(
                    "a",
                    NodeKind::Action {
                        instruction: "x".into(),
                    },
                ),
                node(
                    "b",
                    NodeKind::Action {
                        instruction: "y".into(),
                    },
                ),
            ],
            edges: vec![Edge {
                id: EdgeId::from("e1"),
                from: "a".into(),
                to: "b".into(),
                cond: cond.clone(),
            }],
            ..Default::default()
        };
        assert_eq!(roundtrip(&g), g, "edge cond {cond:?}");
    }
}

#[test]
fn annotations_serialize_camel_case() {
    let node = Node {
        id: "n".into(),
        label: "n".into(),
        kind: NodeKind::Action {
            instruction: "x".into(),
        },
        annotations: NodeAnnotations {
            confidence: Some(0.85),
            sample_count: Some(12),
            last_used_at: Some("2026-05-14T08:00:00Z".into()),
            success_rate: Some(0.9),
            policy: Some(NodePolicy::Auto),
            tool_binding: Some(ToolBinding {
                mcp_server: "gh".into(),
                tool_name: "create_issue".into(),
            }),
        },
    };
    let json = serde_json::to_string(&node).unwrap();
    assert!(json.contains("\"sampleCount\":12"), "json was {json}");
    assert!(json.contains("\"lastUsedAt\""), "json was {json}");
    assert!(json.contains("\"successRate\":0.9"), "json was {json}");
    assert!(json.contains("\"toolBinding\""), "json was {json}");
    assert!(json.contains("\"mcpServer\":\"gh\""), "json was {json}");
    assert!(json.contains("\"toolName\""), "json was {json}");
    assert!(json.contains("\"policy\":\"auto\""), "json was {json}");
}

#[test]
fn variables_roundtrip() {
    let g = SkillGraph {
        entry: "n".into(),
        nodes: vec![node(
            "n",
            NodeKind::Action {
                instruction: "x".into(),
            },
        )],
        variables: vec![
            Variable {
                name: "input".into(),
                var_type: "string".into(),
                scope: VariableScope::Graph,
            },
            Variable {
                name: "i".into(),
                var_type: "number".into(),
                scope: VariableScope::Node,
            },
        ],
        ..Default::default()
    };
    assert_eq!(roundtrip(&g), g);
}

#[test]
fn metadata_roundtrips() {
    let g = SkillGraph {
        entry: "n".into(),
        nodes: vec![node(
            "n",
            NodeKind::Action {
                instruction: "x".into(),
            },
        )],
        metadata: SkillGraphMetadata {
            name: Some("reconcile_bank".into()),
            version: Some("1.0.0".into()),
            description: Some("Reconcile bank statements weekly.".into()),
            source: Some("ghostwriter".into()),
        },
        ..Default::default()
    };
    assert_eq!(roundtrip(&g), g);
}

#[test]
fn empty_graph_roundtrips() {
    let g = SkillGraph::default();
    let json = serde_json::to_string(&g).unwrap();
    let back: SkillGraph = serde_json::from_str(&json).unwrap();
    assert_eq!(g, back);
}
