//! Skill IR schema tests — roundtrip every NodeKind, every EdgeCond
//! variant, and the canonical-hash invariants.

use crate::ir::{
    canonical_hash_hex, canonical_json, BranchArm, Edge, EdgeCond, EdgeId, Node, NodeAnnotations,
    NodeId, NodeKind, NodePolicy, SkillGraph, SkillGraphMetadata, SubSkillRef, ToolBinding,
    Variable, VariableScope,
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
fn canonical_json_is_deterministic_under_key_reorder() {
    // Build the same logical graph two different ways — the canonical
    // form should be byte-identical regardless of which struct field
    // was constructed first.
    let g1 = SkillGraph {
        entry: "a".into(),
        nodes: vec![node(
            "a",
            NodeKind::Action {
                instruction: "x".into(),
            },
        )],
        edges: vec![],
        variables: vec![Variable {
            name: "v".into(),
            var_type: "string".into(),
            scope: VariableScope::Graph,
        }],
        metadata: SkillGraphMetadata {
            name: Some("s".into()),
            version: Some("1".into()),
            description: None,
            source: None,
        },
    };

    // Round-trip through serde_json's object representation (which
    // does NOT preserve struct field order — sort happens in
    // canonical_json) and back.
    let value: serde_json::Value = serde_json::to_value(&g1).unwrap();
    let g2: SkillGraph = serde_json::from_value(value).unwrap();

    assert_eq!(canonical_json(&g1).unwrap(), canonical_json(&g2).unwrap());
}

#[test]
fn canonical_json_has_no_whitespace() {
    let g = SkillGraph {
        entry: "n".into(),
        nodes: vec![node(
            "n",
            NodeKind::Action {
                instruction: "do thing".into(),
            },
        )],
        ..Default::default()
    };
    let canon = canonical_json(&g).unwrap();
    // The string instruction "do thing" legitimately has a space;
    // strip those before checking for structural whitespace.
    let without_strings: String = {
        let mut out = String::new();
        let mut in_string = false;
        let mut escape = false;
        for c in canon.chars() {
            if escape {
                escape = false;
                continue;
            }
            if c == '\\' && in_string {
                escape = true;
                continue;
            }
            if c == '"' {
                in_string = !in_string;
                out.push(c);
                continue;
            }
            if !in_string {
                out.push(c);
            }
        }
        out
    };
    assert!(
        !without_strings.contains(' '),
        "structural whitespace found: {without_strings}"
    );
    assert!(!without_strings.contains('\n'));
    assert!(!without_strings.contains('\t'));
}

#[test]
fn canonical_hash_is_stable() {
    let g = SkillGraph {
        entry: "a".into(),
        nodes: vec![node(
            "a",
            NodeKind::Action {
                instruction: "x".into(),
            },
        )],
        ..Default::default()
    };
    let h1 = canonical_hash_hex(&g).unwrap();
    let h2 = canonical_hash_hex(&g).unwrap();
    assert_eq!(h1, h2);
    assert_eq!(h1.len(), 64);
    assert!(h1.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn canonical_hash_differs_on_node_change() {
    let g1 = SkillGraph {
        entry: "a".into(),
        nodes: vec![node(
            "a",
            NodeKind::Action {
                instruction: "x".into(),
            },
        )],
        ..Default::default()
    };
    let g2 = SkillGraph {
        entry: "a".into(),
        nodes: vec![node(
            "a",
            NodeKind::Action {
                instruction: "y".into(),
            },
        )],
        ..Default::default()
    };
    assert_ne!(
        canonical_hash_hex(&g1).unwrap(),
        canonical_hash_hex(&g2).unwrap()
    );
}

#[test]
fn empty_graph_roundtrips() {
    let g = SkillGraph::default();
    let json = serde_json::to_string(&g).unwrap();
    let back: SkillGraph = serde_json::from_str(&json).unwrap();
    assert_eq!(g, back);
}

#[test]
fn ts_fixture_roundtrip_rust_json_ts_json_rust() {
    // Hand-authored TS fixture — kept in sync with
    // web/lib/types/skill-ir.ts. Demonstrates the camelCase shape
    // a TS consumer would produce when emitting an IR. If this test
    // fails, the TS mirror and Rust schema have diverged.
    let ts_emitted_json = r#"{
        "entry": "n1",
        "nodes": [
            {
                "id": "n1",
                "label": "Fetch invoice",
                "kind": { "type": "action", "instruction": "Download today's invoice." },
                "annotations": {
                    "confidence": 0.92,
                    "sampleCount": 14,
                    "lastUsedAt": "2026-05-14T08:00:00Z",
                    "successRate": 0.86,
                    "policy": "auto",
                    "toolBinding": { "mcpServer": "stripe", "toolName": "list_invoices" }
                }
            },
            {
                "id": "n2",
                "label": "Approve over $500",
                "kind": { "type": "await-approval", "approverRole": "cfo", "timeoutSeconds": 3600 },
                "annotations": {}
            }
        ],
        "edges": [
            { "id": "e1", "from": "n1", "to": "n2", "cond": { "kind": "on-success" } }
        ],
        "variables": [
            { "name": "amount", "varType": "number", "scope": "graph" }
        ],
        "metadata": {
            "name": "reconcile_bank",
            "version": "1.0.0",
            "description": "Weekly reconciliation skill.",
            "source": "ghostwriter"
        }
    }"#;

    // Rust ← JSON (TS-shaped)
    let graph: SkillGraph = serde_json::from_str(ts_emitted_json).unwrap();
    assert_eq!(graph.nodes.len(), 2);
    assert_eq!(graph.edges.len(), 1);
    assert_eq!(graph.variables.len(), 1);

    // Rust → JSON → Rust (verifies serde shape is stable)
    let rust_emitted = serde_json::to_string(&graph).unwrap();
    let back: SkillGraph = serde_json::from_str(&rust_emitted).unwrap();
    assert_eq!(graph, back);

    // Canonical hash is stable across the roundtrip.
    let h1 = canonical_hash_hex(&graph).unwrap();
    let h2 = canonical_hash_hex(&back).unwrap();
    assert_eq!(h1, h2);
}
