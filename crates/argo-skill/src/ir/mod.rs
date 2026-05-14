//! Skill IR — canonical structured representation of a skill.
//!
//! See [`types`] for the schema and [`serde`] for canonical JSON +
//! hash. Track L brief: `streams/L1-skill-ir-schema.md`.

#![warn(missing_docs)]

mod serde;
pub mod types;

#[cfg(test)]
mod types_tests;

pub use self::serde::{canonical_hash, canonical_hash_hex, canonical_json};
pub use self::types::{
    BranchArm, Edge, EdgeCond, EdgeId, Node, NodeAnnotations, NodeId, NodeKind, NodePolicy,
    SkillGraph, SkillGraphMetadata, SubSkillRef, ToolBinding, Variable, VariableScope,
};
