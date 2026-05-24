//! Smoke tests for the `argo-skill-sdk` authoring flow.
//!
//! These are intentionally user-shaped examples: an echo skill, a
//! tool/capability-declared skill, and a metadata-bearing approval
//! checkpoint skill. The surface-contract test pins API names; this
//! file proves the public builder flow authors useful graphs.
use argo_skill_sdk::{
    validate_skill_graph, BuilderNodeOptions, EdgeCond, NodeKind, NodePolicy, SkillBuilder,
    SkillBuilderInput, ToolBinding,
};

fn input(name: &str, version: &str, description: &str, source: &str) -> SkillBuilderInput {
    SkillBuilderInput::default()
        .name(name)
        .version(version)
        .description(description)
        .source(source)
}

#[test]
fn authors_echo_skill_with_versioned_metadata() {
    let graph = SkillBuilder::new(input(
        "echo-skill",
        "1.0.0",
        "Return the caller input unchanged.",
        "hand-authored",
    ))
    .action(
        "echo",
        "Echo input",
        "Return `{{ input.text }}` without modification.",
    )
    .variable("input.text", "string")
    .build()
    .expect("echo skill builds");

    let graph = validate_skill_graph(graph).expect("echo skill validates");
    assert_eq!(graph.entry.as_str(), "echo");
    assert_eq!(graph.metadata.name.as_deref(), Some("echo-skill"));
    assert_eq!(graph.metadata.version.as_deref(), Some("1.0.0"));
    assert_eq!(graph.metadata.source.as_deref(), Some("hand-authored"));
    assert_eq!(graph.variables[0].name, "input.text");
    assert!(matches!(graph.nodes[0].kind, NodeKind::Action { .. }));
}

#[test]
fn authors_capability_declared_skill_with_tool_binding_and_policy() {
    let mut options = BuilderNodeOptions::default().tool_binding(ToolBinding {
        mcp_server: "slack".into(),
        tool_name: "chat.postMessage".into(),
    });
    options.annotations.policy = Some(NodePolicy::Ask);

    let graph = SkillBuilder::new(input(
        "approval-gated-slack-post",
        "1.0.0",
        "Draft a Slack post and require operator approval before sending.",
        "hand-authored",
    ))
    .action_with(
        "post",
        "Post approved message",
        "Send the approved message to the configured Slack channel.",
        options,
    )
    .build()
    .expect("capability-declared skill builds");

    let graph = validate_skill_graph(graph).expect("capability-declared skill validates");
    let annotations = &graph.nodes[0].annotations;
    let binding = annotations
        .tool_binding
        .as_ref()
        .expect("tool binding metadata is preserved");
    assert_eq!(binding.mcp_server, "slack");
    assert_eq!(binding.tool_name, "chat.postMessage");
    assert_eq!(annotations.policy, Some(NodePolicy::Ask));
}

#[test]
fn authors_metadata_bearing_skill_with_human_checkpoint() {
    let graph = SkillBuilder::new(input(
        "invoice-exception-review",
        "0.1.0",
        "Route invoice exceptions through a human checkpoint.",
        "sdk-smoke",
    ))
    .fetch("invoice", "Fetch invoice packet", "document.invoice")
    .condition(
        "exception",
        "Detect exception",
        "invoice.total_mismatch || invoice.missing_po",
    )
    .approval_timeout("review", "Ask AP lead", "ap-lead", Some(3_600))
    .action(
        "record",
        "Record review packet",
        "Persist the reviewed exception packet.",
    )
    .edge("e1", "invoice", "exception")
    .edge_when("e2", "exception", "review", EdgeCond::OnSuccess)
    .edge("e3", "review", "record")
    .build()
    .expect("metadata-bearing skill builds");

    let graph = validate_skill_graph(graph).expect("metadata-bearing skill validates");
    assert_eq!(
        graph.metadata.description.as_deref(),
        Some("Route invoice exceptions through a human checkpoint.")
    );
    assert_eq!(graph.metadata.source.as_deref(), Some("sdk-smoke"));
    assert!(graph.nodes.iter().any(|node| matches!(
        node.kind,
        NodeKind::AwaitApproval {
            timeout_seconds: Some(3_600),
            ..
        }
    )));

    let json = serde_json::to_value(&graph).expect("graph serializes");
    assert_eq!(json["metadata"]["name"], "invoice-exception-review");
    assert_eq!(json["metadata"]["version"], "0.1.0");
    assert_eq!(json["metadata"]["source"], "sdk-smoke");
}
