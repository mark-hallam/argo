//! Canonical-JSON + canonical-hash + TS-fixture interop tests.
//!
//! Extracted from `types_tests.rs` under the
//! `codex-split-files-450-500-wave` brief — the parent file was at
//! the 488-LOC border. Pulled verbatim, with the `node` helper
//! duplicated locally (the original was a private fn in the
//! sibling test module; copying it keeps both files independently
//! buildable without exposing test-only helpers).
use crate::ir::{
    canonical_hash_hex, canonical_json, Node, NodeAnnotations, NodeId, NodeKind, SkillGraph,
    SkillGraphMetadata, Variable, VariableScope,
};

fn node(id: &str, kind: NodeKind) -> Node {
    Node {
        id: NodeId::from(id),
        label: format!("node {id}"),
        kind,
        annotations: NodeAnnotations::default(),
    }
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
