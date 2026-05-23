//! Contract pins for the `argo-skill-sdk` public surface.
//!
//! Authored by `eval-argo-skill-sdk-surface` (claim-4 audit, 2026-05-23).
//!
//! Purpose: lock the pub-API surface so a rename, removal, or
//! signature change in `crates/argo-skill-sdk/src/` fails this test
//! at PR time instead of silently shipping a breaking change to
//! external skill-plugin authors (the crate has zero workspace
//! callers per its `Cargo.toml` annotation — these tests are the
//! only fence on it).
//!
//! Each section pins one slice of the surface inventoried in the
//! crate-level rustdoc:
//!
//! - L1 IR re-exports (15 types via `argo_skill::ir`)
//! - Builder API: `SkillBuilder` + free fns
//! - Builder inputs: `SkillBuilderInput`, `BuilderNodeOptions`
//! - Error enum: `SkillBuildError` (9 variants — exhaustive match)
//! - `sample_skills` shape

#![allow(dead_code, clippy::extra_unused_lifetimes)]

use argo_skill_sdk::{
    branch_arm, branch_arm_pattern, create_skill, sample_skills, validate_skill_graph,
    BranchArm, BuilderNodeOptions, Edge, EdgeCond, EdgeId, Node, NodeAnnotations, NodeId,
    NodeKind, NodePolicy, SkillBuildError, SkillBuilder, SkillBuilderInput, SkillGraph,
    SkillGraphMetadata, SubSkillRef, ToolBinding, Variable, VariableScope,
};

// ---------------------------------------------------------------------------
// L1 IR re-exports — compile-fence: importing each name above is the proof
// that all 15 re-exports remain pub. If any disappear, this file fails to
// build.
// ---------------------------------------------------------------------------

fn _ir_reexports_present() {
    fn _branch_arm(_: BranchArm) {}
    fn _edge(_: Edge) {}
    fn _edge_cond(_: EdgeCond) {}
    fn _edge_id(_: EdgeId) {}
    fn _node(_: Node) {}
    fn _node_annotations(_: NodeAnnotations) {}
    fn _node_id(_: NodeId) {}
    fn _node_kind(_: NodeKind) {}
    fn _node_policy(_: NodePolicy) {}
    fn _skill_graph(_: SkillGraph) {}
    fn _skill_graph_metadata(_: SkillGraphMetadata) {}
    fn _sub_skill_ref(_: SubSkillRef) {}
    fn _tool_binding(_: ToolBinding) {}
    fn _variable(_: Variable) {}
    fn _variable_scope(_: VariableScope) {}
}

// ---------------------------------------------------------------------------
// Free-function signatures
// ---------------------------------------------------------------------------

#[test]
fn create_skill_returns_skill_builder() {
    let builder: SkillBuilder = create_skill();
    let _: SkillBuilder = builder; // type-pin
}

#[test]
fn branch_arm_helpers_return_branch_arm() {
    let plain: BranchArm = branch_arm("billing");
    assert_eq!(plain.label, "billing");
    assert!(plain.pattern.is_none());

    let patterned: BranchArm = branch_arm_pattern("billing", "invoice|bill");
    assert_eq!(patterned.label, "billing");
    assert_eq!(patterned.pattern.as_deref(), Some("invoice|bill"));
}

#[test]
fn validate_skill_graph_returns_result_skillgraph_skillbuilderror() {
    let graph = create_skill().action("a", "A", "go").build().unwrap();
    let validated: Result<SkillGraph, SkillBuildError> = validate_skill_graph(graph);
    let _ = validated.unwrap();
}

// ---------------------------------------------------------------------------
// SkillBuilder method surface — every fluent method called once, with the
// `Self` return type checked. Any rename, removed method, or method whose
// chain return changes from `Self` breaks compilation.
// ---------------------------------------------------------------------------

#[test]
fn skill_builder_method_surface_compiles_as_documented() {
    let _: SkillBuilder = SkillBuilder::new(SkillBuilderInput::default());
    let _: SkillBuilder = SkillBuilder::default();

    let builder = create_skill();

    let builder = builder
        .action("a1", "label", "instr")
        .action_with("a2", "label", "instr", BuilderNodeOptions::default())
        .fetch("f1", "label", "target")
        .fetch_method("f2", "label", "target", "POST")
        .transform("t1", "label", "{{template}}")
        .condition("c1", "label", "expr")
        .approval("ap1", "label", "approver-role")
        .approval_timeout("ap2", "label", "approver-role", Some(60))
        .branch("b1", "label", vec![branch_arm("only")])
        .sub_skill(
            "s1",
            "label",
            SubSkillRef {
                skill_id: "x".into(),
                version: "1.0.0".into(),
            },
        )
        .loop_over("l1", "label", "item", vec![NodeId::from("a1")])
        .try_catch(
            "tc1",
            "label",
            vec![NodeId::from("a1")],
            vec![NodeId::from("a2")],
        )
        .edge("e1", "a1", "a2")
        .edge_when("e2", "a2", "f1", EdgeCond::OnSuccess)
        .variable("v1", "string")
        .variable_scoped("v2", "string", VariableScope::Graph)
        .start("a1");

    let _: SkillBuilder = builder.clone(); // SkillBuilder: Clone
    let graph: SkillGraph = builder.build().unwrap();
    assert_eq!(graph.entry.as_str(), "a1");
}

// ---------------------------------------------------------------------------
// Builder-input shapes — pub-field surface pinned. Adding a field is
// technically a breaking change at the struct-literal callsite below.
// ---------------------------------------------------------------------------

#[test]
fn skill_builder_input_pub_field_surface() {
    let input = SkillBuilderInput {
        name: Some("n".into()),
        version: Some("v".into()),
        description: Some("d".into()),
        source: Some("s".into()),
    };
    let chained = SkillBuilderInput::default()
        .name("n")
        .version("v")
        .description("d")
        .source("s");
    assert_eq!(input, chained);
    let _: SkillGraphMetadata = input.into();
}

#[test]
fn builder_node_options_pub_field_surface() {
    let opts = BuilderNodeOptions {
        annotations: NodeAnnotations::default(),
    };
    let chained = BuilderNodeOptions::default()
        .tool_binding(ToolBinding {
            mcp_server: "slack".into(),
            tool_name: "chat.postMessage".into(),
        })
        .confidence(0.5);
    assert_eq!(opts, BuilderNodeOptions::default());
    assert_eq!(chained.annotations.confidence, Some(0.5));
    assert_eq!(
        chained.annotations.tool_binding.as_ref().unwrap().tool_name,
        "chat.postMessage"
    );
}

// ---------------------------------------------------------------------------
// Error enum — exhaustive match pins all 9 variants. Adding or removing a
// variant fails compilation here. (`SkillBuildError` is intentionally not
// `#[non_exhaustive]` at v0.0.1 — see lib.rs rustdoc note 2 for the v1.0
// promotion plan.)
// ---------------------------------------------------------------------------

#[test]
fn skill_build_error_variants_pinned_exhaustively() {
    fn assert_all_variants(e: SkillBuildError) -> &'static str {
        match e {
            SkillBuildError::DuplicateNode(_) => "DuplicateNode",
            SkillBuildError::DuplicateEdge(_) => "DuplicateEdge",
            SkillBuildError::DuplicateVariable(_) => "DuplicateVariable",
            SkillBuildError::MissingEntry(_) => "MissingEntry",
            SkillBuildError::MissingEdgeNode(_) => "MissingEdgeNode",
            SkillBuildError::InvalidConfidence(_) => "InvalidConfidence",
            SkillBuildError::EmptyBranch(_) => "EmptyBranch",
            SkillBuildError::EmptyLoop(_) => "EmptyLoop",
            SkillBuildError::EmptyTryCatch(_) => "EmptyTryCatch",
        }
    }

    // Build one of each so the constructors are also pinned.
    let cases = [
        SkillBuildError::DuplicateNode("id".into()),
        SkillBuildError::DuplicateEdge("id".into()),
        SkillBuildError::DuplicateVariable("name".into()),
        SkillBuildError::MissingEntry("id".into()),
        SkillBuildError::MissingEdgeNode("id".into()),
        SkillBuildError::InvalidConfidence("id".into()),
        SkillBuildError::EmptyBranch("id".into()),
        SkillBuildError::EmptyLoop("id".into()),
        SkillBuildError::EmptyTryCatch("id".into()),
    ];
    assert_eq!(cases.len(), 9);

    // Each variant has a Display + std::error::Error impl.
    for case in cases {
        let _: String = case.to_string();
        let label = assert_all_variants(case.clone());
        assert!(!label.is_empty());
        let _: &dyn std::error::Error = &case;
    }
}

// ---------------------------------------------------------------------------
// sample_skills shape — count pinned at 5 (parity with TypeScript SDK
// package, per the rustdoc).
// ---------------------------------------------------------------------------

#[test]
fn sample_skills_returns_five_validated_graphs() {
    let samples: Vec<SkillGraph> = sample_skills().unwrap();
    assert_eq!(samples.len(), 5);
    for graph in samples {
        let _ = validate_skill_graph(graph).unwrap();
    }
}
