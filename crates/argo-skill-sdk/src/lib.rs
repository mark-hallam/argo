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
//!
//! # Public surface
//!
//! The crate is consumed by external skill-plugin authors (no
//! workspace callers — see the `zero-caller-allowed` annotation on
//! `Cargo.toml`). The complete public surface is:
//!
//! - **L1 IR re-exports** from `argo_skill::ir` (15 types):
//!   [`BranchArm`], [`Edge`], [`EdgeCond`], [`EdgeId`], [`Node`],
//!   [`NodeAnnotations`], [`NodeId`], [`NodeKind`], [`NodePolicy`],
//!   [`SkillGraph`], [`SkillGraphMetadata`], [`SubSkillRef`],
//!   [`ToolBinding`], [`Variable`], [`VariableScope`].
//! - **Builder API**: [`SkillBuilder`] + the [`create_skill`],
//!   [`branch_arm`], [`branch_arm_pattern`], [`validate_skill_graph`]
//!   free functions.
//! - **Builder inputs**: [`SkillBuilderInput`] (skill-level metadata)
//!   and [`BuilderNodeOptions`] (per-node options).
//! - **Errors**: [`SkillBuildError`] (9 variants).
//! - **Sample graphs**: [`sample_skills`].
//!
//! Every item above is pinned by the contract test at
//! `tests/surface_contract.rs`: renaming, removing, or changing the
//! signature of any pub item fails that test, surfacing the breaking
//! change at PR time.
//!
//! # Versioning + breaking-change discipline
//!
//! The crate inherits the workspace `version = "0.0.1"` — pre-1.0
//! per semver, so breaking changes between `0.0.x` bumps are allowed
//! by the spec. Until the workspace promotes to `0.x` or `1.0.0`,
//! external skill-plugin authors should pin to an exact `=0.0.1`
//! dependency, not a caret range.
//!
//! Three known breaking-change sources downstream consumers should
//! be aware of (none are bugs — they are the trade-offs of an SDK
//! that re-exports an evolving IR substrate):
//!
//! 1. **IR type churn propagates.** This crate re-exports 15 types
//!    from `argo_skill::ir` wholesale. Any field-add or variant-add
//!    in those types appears in the SDK surface without an SDK
//!    version bump. The contract test pins the re-export names but
//!    not the upstream shape; consumers should treat the IR types
//!    as authored by `argo-skill`, not by this crate.
//! 2. **`SkillBuildError` is not `#[non_exhaustive]`.** Adding a
//!    variant is technically breaking for downstream `match` arms.
//!    Pre-1.0 this is acceptable; at the 1.0 promotion the enum
//!    should either be marked `#[non_exhaustive]` or frozen.
//! 3. **`SkillBuilderInput` and `BuilderNodeOptions` expose pub
//!    fields.** Adding a field is breaking for struct-literal
//!    construction. Builder methods are the recommended construction
//!    path; pre-1.0 the pub-field surface is acceptable.

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
