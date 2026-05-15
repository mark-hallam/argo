//! Skill IR — canonical structured representation of a skill.
//!
//! See [`types`] for the schema and [`serde`] for canonical JSON +
//! hash. Track L brief: `streams/L1-skill-ir-schema.md`.

#![warn(missing_docs)]

pub mod backfill;
pub mod confidence_telemetry;
pub mod from_cu_trajectory;
pub mod ghostwriter;
pub mod ghostwriter_prompts;
pub mod portable;
pub mod save_pipeline;
mod serde;
pub mod store;
pub mod types;

#[cfg(test)]
mod types_tests;

pub use self::backfill::{
    synthesize_graph, BackfillEntry, BackfillReport, BackfillStatus, BACKFILL_ENTRY_NODE_ID,
    BACKFILL_SOURCE,
};
pub use self::confidence_telemetry::{
    ConfidenceTelemetry, NodeTelemetryKey, NodeTelemetryRow, TelemetryOutcome, TelemetrySample,
};
pub use self::from_cu_trajectory::{
    graph_from_cu_trajectory, CuTrajectory, CU_RECORDING_SOURCE, LOW_CONFIDENCE_THRESHOLD,
};
pub use self::ghostwriter::{
    generate_from_prompt, offline_template, validate_graph, GhostwriterClient, GhostwriterError,
    OfflineGhostwriterClient, GHOSTWRITER_SOURCE,
};
pub use self::ghostwriter_prompts::{build_initial_prompt, build_self_correction_prompt};
pub use self::portable::{
    export_skill_ir_bundle, graph_from_skill_ir_bundle, SkillIrBundle, SKILL_IR_BUNDLE_SCHEMA,
};
pub use self::save_pipeline::{save_edited_graph, validate_editable_topology, SavePipelineOutcome};
pub use self::serde::{canonical_hash, canonical_hash_hex, canonical_json};
pub use self::store::{ensure_schema, verify_chain, ChainRow, SkillIrStore, SqliteSkillIrStore};
pub use self::types::{
    BranchArm, Edge, EdgeCond, EdgeId, Node, NodeAnnotations, NodeId, NodeKind, NodePolicy,
    SkillGraph, SkillGraphMetadata, SubSkillRef, ToolBinding, Variable, VariableScope,
};
