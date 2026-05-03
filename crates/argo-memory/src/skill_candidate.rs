//! Memory-backed learned skill candidate parsing (#697).
//!
//! Convention established by GTM Engineer's `propose_gtm_skill`
//! (and reusable for any future vertical that wants memory-
//! backed skill proposals): a `MemoryEntry` whose body is a
//! JSON object with `schema_key = "learned_skill_candidates"`
//! and at minimum a `skill_name` is treated as a candidate
//! that operators can review.
//!
//! Why JSON-in-body rather than a new column? The memory_entry
//! table is a generic key/value store — adding per-key columns
//! would balloon the schema for every new schema_key. The
//! JSON-in-body convention keeps the storage layer simple and
//! lets verticals iterate on candidate shape without migrations.
//!
//! Required fields enforced here: `schema_key`, `skill_name`.
//! Optional fields preserved verbatim: `problem`,
//! `reusable_pattern`, `evidence`, `expected_reuse`, `risk`.
//! Unknown fields are kept in `extra` so a future schema bump
//! is forward-compatible.
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The reserved `schema_key` value that identifies a candidate
/// row. Any memory entry whose JSON body carries this key (and
/// `skill_name`) is surfaced by `argo skill review list`.
pub const SCHEMA_KEY_LEARNED_SKILL_CANDIDATE: &str = "learned_skill_candidates";

/// Parsed view of a memory-backed skill candidate. Returned by
/// [`parse_skill_candidate`]. Pure data — no I/O.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillCandidate {
    /// Required. Snake-case skill name the operator will review.
    pub skill_name: String,
    /// Optional one-paragraph problem description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub problem: Option<String>,
    /// Optional pattern description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reusable_pattern: Option<String>,
    /// Optional evidence summary.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence: Option<String>,
    /// Optional expected-reuse description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_reuse: Option<String>,
    /// Optional risk note.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub risk: Option<String>,
}

/// Parse a memory entry's body as a skill candidate. Returns
/// `None` when the body isn't JSON, isn't an object, lacks
/// `schema_key == SCHEMA_KEY_LEARNED_SKILL_CANDIDATE`, or
/// lacks a non-empty `skill_name`.
///
/// Permissive on extra fields — verticals may evolve the
/// candidate shape over time; the read side ignores what it
/// doesn't recognise so old + new shapes coexist.
#[must_use]
pub fn parse_skill_candidate(body: &str) -> Option<SkillCandidate> {
    let v: Value = serde_json::from_str(body).ok()?;
    let obj = v.as_object()?;
    let schema_key = obj.get("schema_key")?.as_str()?;
    if schema_key != SCHEMA_KEY_LEARNED_SKILL_CANDIDATE {
        return None;
    }
    let skill_name = obj.get("skill_name")?.as_str()?;
    if skill_name.trim().is_empty() {
        return None;
    }
    Some(SkillCandidate {
        skill_name: skill_name.to_string(),
        problem: obj.get("problem").and_then(|v| v.as_str()).map(String::from),
        reusable_pattern: obj
            .get("reusable_pattern")
            .and_then(|v| v.as_str())
            .map(String::from),
        evidence: obj
            .get("evidence")
            .and_then(|v| v.as_str())
            .map(String::from),
        expected_reuse: obj
            .get("expected_reuse")
            .and_then(|v| v.as_str())
            .map(String::from),
        risk: obj.get("risk").and_then(|v| v.as_str()).map(String::from),
    })
}

#[cfg(test)]
#[path = "skill_candidate_tests.rs"]
mod tests;
