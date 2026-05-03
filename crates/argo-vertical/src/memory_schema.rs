//! Memory-schema TOML parser for vertical templates (#663).
//!
//! Companion to the manifest's `[memory].schema = "…"` field.
//! When the manifest declares a schema file, this module parses
//! it into a strongly-typed [`MemorySchema`] so:
//!
//! - the validator can fail at vertical-load time on malformed
//!   schemas (instead of at first memory write), and
//! - the CLI's `argo vertical show` can surface the typed
//!   entries the operator + the agent are working with.
//!
//! Runtime write-time enforcement (rejecting a row that doesn't
//! match its declared `type`) is a follow-up — this PR ships the
//! schema *parser + validator hookup* slice so the manifest
//! field stops being a documentation-only string.
//!
//! ## Example schema file
//!
//! ```toml
//! schema_version = "1"
//!
//! [[entry]]
//! key  = "recurring_visitors"
//! type = "list-of-objects"
//! purpose = "People we've talked to before."
//!
//! [[entry]]
//! key  = "out_of_scope_patterns"
//! type = "list-of-strings"
//! purpose = "Topics that always escalate."
//! ```
//!
//! ## Supported entry types (v1)
//!
//! - `list-of-objects` — a JSON array whose items are objects
//!   (free-form keys today; v2 will add per-entry sub-schemas).
//! - `list-of-strings` — a JSON array of strings.
//! - `string` — a single string scalar.
//! - `object` — a single JSON object.
//! - `int`, `float`, `bool` — single primitive values.
use argo_types::{ArgoError, ArgoResult};
use serde::{Deserialize, Serialize};

/// Schema version this argo build understands. Future bumps
/// land additively or via `argo vertical migrate` — same policy
/// as `vertical.toml` and `*.workflow.toml`.
pub const MEMORY_SCHEMA_VERSION: &str = "1";

/// One memory entry's typed declaration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(deny_unknown_fields)]
pub struct MemoryEntry {
    /// Unique key. Memory writes target this by name.
    pub key: String,
    /// Type discriminator — one of:
    /// `list-of-objects`, `list-of-strings`, `string`, `object`,
    /// `int`, `float`, `bool`.
    #[serde(rename = "type")]
    pub type_: String,
    /// Optional human description. Surfaced by `argo vertical
    /// show`. Keep short; this is the operator's hint, not the
    /// agent's prompt fragment.
    #[serde(default)]
    pub purpose: String,
}

/// The whole `memory.schema.toml`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(deny_unknown_fields)]
pub struct MemorySchema {
    /// Must equal [`MEMORY_SCHEMA_VERSION`].
    pub schema_version: String,
    /// Ordered list of typed entries. The validator rejects
    /// duplicates by `key` so the CLI can resolve unambiguously.
    #[serde(default, rename = "entry")]
    pub entries: Vec<MemoryEntry>,
}

/// Parse a memory-schema TOML body.
pub fn parse_memory_schema(src: &str) -> ArgoResult<MemorySchema> {
    let s: MemorySchema =
        toml::from_str(src).map_err(|e| ArgoError::Schema(format!("memory.schema: {e}")))?;
    if s.schema_version != MEMORY_SCHEMA_VERSION {
        return Err(ArgoError::Schema(format!(
            "memory.schema schema_version '{}' is not supported by this argo build (expected '{}')",
            s.schema_version, MEMORY_SCHEMA_VERSION
        )));
    }
    validate_memory_schema(&s)?;
    Ok(s)
}

/// Cross-entry validation. Called by [`parse_memory_schema`];
/// exposed publicly so the manifest validator can re-check on
/// demand.
pub fn validate_memory_schema(s: &MemorySchema) -> ArgoResult<()> {
    let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for e in &s.entries {
        if e.key.trim().is_empty() {
            return Err(ArgoError::Schema(
                "memory.schema: entry has empty `key`".into(),
            ));
        }
        if !seen.insert(e.key.as_str()) {
            return Err(ArgoError::Schema(format!(
                "memory.schema: duplicate entry key '{}'",
                e.key
            )));
        }
        if !is_supported_type(&e.type_) {
            return Err(ArgoError::Schema(format!(
                "memory.schema: entry '{}' has unknown type '{}' (expected one of: list-of-objects | list-of-strings | string | object | int | float | bool)",
                e.key, e.type_
            )));
        }
    }
    Ok(())
}

fn is_supported_type(t: &str) -> bool {
    matches!(
        t,
        "list-of-objects"
            | "list-of-strings"
            | "string"
            | "object"
            | "int"
            | "float"
            | "bool"
    )
}

#[cfg(test)]
#[path = "memory_schema_tests.rs"]
mod tests;
