use argo_skill::ir::{canonical_json, EdgeCond, NodeId, NodeKind, ToolBinding, VariableScope};
use argo_skill_sdk::{
    branch_arm_pattern, create_skill, sample_skills, validate_skill_graph, BuilderNodeOptions,
    SkillBuildError, SkillBuilder, SkillBuilderInput,
};

#[test]
fn builds_action_graph_using_first_node_as_entry() {
    let graph = SkillBuilder::new(SkillBuilderInput::default().name("hello").version("1"))
        .action("draft", "Draft reply", "Write a reply")
        .build()
        .unwrap();

    assert_eq!(graph.entry.as_str(), "draft");
    assert!(matches!(graph.nodes[0].kind, NodeKind::Action { .. }));
    assert_eq!(graph.metadata.name.as_deref(), Some("hello"));
}

#[test]
fn supports_explicit_entry_selection() {
    let graph = create_skill()
        .action("a", "A", "first")
        .action("b", "B", "second")
        .start("b")
        .build()
        .unwrap();

    assert_eq!(graph.entry.as_str(), "b");
}

#[test]
fn builds_typed_edges_and_variables() {
    let graph = create_skill()
        .fetch("read", "Read", "gmail")
        .transform("clean", "Clean", "{{body}}")
        .edge_when("read-clean", "read", "clean", EdgeCond::OnSuccess)
        .variable("body", "string")
        .build()
        .unwrap();

    assert_eq!(graph.edges[0].cond, EdgeCond::OnSuccess);
    assert_eq!(graph.variables[0].scope, VariableScope::Graph);
}

#[test]
fn preserves_tool_bindings_and_confidence_annotations() {
    let graph = create_skill()
        .action_with(
            "post",
            "Post",
            "Publish",
            BuilderNodeOptions::default()
                .confidence(0.8)
                .tool_binding(ToolBinding {
                    mcp_server: "slack".into(),
                    tool_name: "chat.postMessage".into(),
                }),
        )
        .build()
        .unwrap();

    let annotations = &graph.nodes[0].annotations;
    assert_eq!(
        annotations.tool_binding.as_ref().unwrap().tool_name,
        "chat.postMessage"
    );
    assert_eq!(annotations.confidence, Some(0.8));
}

#[test]
fn rejects_duplicate_node_ids() {
    let err = create_skill()
        .action("same", "A", "one")
        .action("same", "B", "two")
        .build()
        .unwrap_err();

    assert_eq!(err, SkillBuildError::DuplicateNode("same".into()));
}

#[test]
fn rejects_duplicate_variable_names() {
    let err = create_skill()
        .action("a", "A", "go")
        .variable("x", "string")
        .variable("x", "number")
        .build()
        .unwrap_err();

    assert_eq!(err, SkillBuildError::DuplicateVariable("x".into()));
}

#[test]
fn rejects_edges_to_missing_nodes() {
    let err = create_skill()
        .action("a", "A", "go")
        .edge("bad", "a", "missing")
        .build()
        .unwrap_err();

    assert_eq!(err, SkillBuildError::MissingEdgeNode("bad".into()));
}

#[test]
fn rejects_invalid_confidence_scores() {
    let err = create_skill()
        .action_with(
            "a",
            "A",
            "go",
            BuilderNodeOptions::default().confidence(1.1),
        )
        .build()
        .unwrap_err();

    assert_eq!(err, SkillBuildError::InvalidConfidence("a".into()));
}

#[test]
fn rejects_empty_branch_arms() {
    let err = create_skill()
        .branch("route", "Route", vec![])
        .build()
        .unwrap_err();

    assert_eq!(err, SkillBuildError::EmptyBranch("route".into()));
}

#[test]
fn rejects_empty_loop_and_try_catch_bodies() {
    let loop_err = create_skill()
        .loop_over("loop", "Loop", "item", vec![])
        .build()
        .unwrap_err();
    assert_eq!(loop_err, SkillBuildError::EmptyLoop("loop".into()));

    let try_err = create_skill()
        .action("a", "A", "go")
        .try_catch("guard", "Guard", vec![NodeId::from("a")], vec![])
        .build()
        .unwrap_err();
    assert_eq!(try_err, SkillBuildError::EmptyTryCatch("guard".into()));
}

#[test]
fn builds_five_sample_skills_against_l1_shape() {
    let samples = sample_skills().unwrap();

    assert_eq!(samples.len(), 5);
    for graph in samples {
        let valid = validate_skill_graph(graph).unwrap();
        assert!(!valid.nodes.is_empty());
        assert!(valid.metadata.name.is_some());
        let json = canonical_json(&valid).unwrap();
        let roundtrip: argo_skill_sdk::SkillGraph = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip.entry, valid.entry);
    }
}

#[test]
fn branch_arm_helpers_map_to_l1_schema() {
    let graph = create_skill()
        .branch(
            "route",
            "Route",
            vec![branch_arm_pattern("billing", "invoice")],
        )
        .action("reply", "Reply", "draft")
        .edge("e1", "route", "reply")
        .build()
        .unwrap();

    match &graph.nodes[0].kind {
        NodeKind::Branch { arms } => {
            assert_eq!(arms[0].label, "billing");
            assert_eq!(arms[0].pattern.as_deref(), Some("invoice"));
        }
        other => panic!("expected branch, got {other:?}"),
    }
}
