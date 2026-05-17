//! Fluent Rust SDK for authoring Skycot Skill IR graphs.
//!
//! The SDK consumes and emits the canonical L1 IR types from
//! `argo_skill::ir`; it does not define a parallel graph schema.
//!
//! # Example
//! ```
//! use argo_skill_sdk::{create_skill, validate_skill_graph};
//!
//! let graph = create_skill()
//!     .action("draft", "Draft reply", "Write a concise response")
//!     .build()
//!     .unwrap();
//!
//! validate_skill_graph(graph).unwrap();
//! ```

mod builders;
mod builders_inputs;
mod error;

pub use argo_skill::ir::{
    BranchArm, Edge, EdgeCond, EdgeId, Node, NodeAnnotations, NodeId, NodeKind, NodePolicy,
    SkillGraph, SkillGraphMetadata, SubSkillRef, ToolBinding, Variable, VariableScope,
};
pub use builders::{branch_arm, branch_arm_pattern, create_skill, validate_skill_graph, SkillBuilder};
pub use builders_inputs::{BuilderNodeOptions, SkillBuilderInput};
pub use error::SkillBuildError;

/// Five sample graphs matching the TypeScript SDK package.
///
/// # Example
/// ```
/// use argo_skill_sdk::sample_skills;
/// let samples = sample_skills().unwrap();
/// assert_eq!(samples.len(), 5);
/// ```
pub fn sample_skills() -> Result<Vec<SkillGraph>, SkillBuildError> {
    Ok(vec![
        SkillBuilder::new(
            SkillBuilderInput::default()
                .name("recruiter-screen")
                .version("1.0.0"),
        )
        .fetch(
            "inbox",
            "Read inbound candidate mail",
            "gmail.messages.unread",
        )
        .condition(
            "qualified",
            "Candidate matches role",
            "candidate.years >= 4",
        )
        .approval_timeout(
            "approve",
            "Ask recruiter lead",
            "recruiter-lead",
            Some(3600),
        )
        .edge("e1", "inbox", "qualified")
        .edge_when("e2", "qualified", "approve", EdgeCond::OnSuccess)
        .variable("candidate", "Candidate")
        .build()?,
        SkillBuilder::new(
            SkillBuilderInput::default()
                .name("ap-invoice-check")
                .version("1.0.0"),
        )
        .fetch("invoice", "Load invoice", "document.invoice")
        .transform("normalize", "Normalize totals", "{{invoice.total}}")
        .approval("approve", "Route exception", "ap-manager")
        .edge("e1", "invoice", "normalize")
        .edge_when(
            "e2",
            "normalize",
            "approve",
            EdgeCond::OnPattern {
                pattern: "exception".into(),
            },
        )
        .build()?,
        SkillBuilder::new(
            SkillBuilderInput::default()
                .name("support-tier1-triage")
                .version("1.0.0"),
        )
        .fetch("ticket", "Read ticket", "zendesk.ticket")
        .branch(
            "route",
            "Classify priority",
            vec![
                branch_arm_pattern("billing", "billing|invoice"),
                branch_arm_pattern("technical", "error|bug"),
            ],
        )
        .action("reply", "Draft response", "Write a concise first response")
        .edge("e1", "ticket", "route")
        .edge("e2", "route", "reply")
        .build()?,
        SkillBuilder::new(
            SkillBuilderInput::default()
                .name("vendor-renewal")
                .version("1.0.0"),
        )
        .action("score", "Score vendor", "Compare price, SLA, and incidents")
        .sub_skill(
            "negotiate",
            "Draft negotiation",
            SubSkillRef {
                skill_id: "canonical.negotiate-renewal".into(),
                version: "1.0.0".into(),
            },
        )
        .edge("e1", "score", "negotiate")
        .build()?,
        SkillBuilder::new(
            SkillBuilderInput::default()
                .name("deal-desk-validation")
                .version("1.0.0"),
        )
        .action("validate", "Validate quote", "Check discount policy")
        .try_catch(
            "guard",
            "Handle policy failure",
            vec![NodeId::from("validate")],
            vec![NodeId::from("escalate")],
        )
        .action(
            "escalate",
            "Escalate non-standard terms",
            "Notify deal desk",
        )
        .edge_when("e1", "guard", "escalate", EdgeCond::OnFailure)
        .start("guard")
        .build()?,
    ])
}
