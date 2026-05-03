//! Unit tests for the JSON-in-body convention parser (#697).
#![cfg(test)]

use super::*;

#[test]
fn parses_minimal_valid_candidate() {
    let body = r#"{"schema_key":"learned_skill_candidates","skill_name":"my_skill"}"#;
    let c = parse_skill_candidate(body).expect("must parse");
    assert_eq!(c.skill_name, "my_skill");
    assert!(c.problem.is_none());
    assert!(c.evidence.is_none());
}

#[test]
fn parses_full_candidate_with_all_optional_fields() {
    let body = r#"{
        "schema_key":"learned_skill_candidates",
        "skill_name":"technical_founder_outreach",
        "problem":"generic copy fails on technical founders",
        "reusable_pattern":"lead with auditability angle",
        "evidence":"3 campaigns reinforced the pattern",
        "expected_reuse":"every founder-segment campaign",
        "risk":"may not generalise to non-technical founders"
    }"#;
    let c = parse_skill_candidate(body).expect("must parse");
    assert_eq!(c.skill_name, "technical_founder_outreach");
    assert_eq!(c.problem.as_deref(), Some("generic copy fails on technical founders"));
    assert_eq!(c.reusable_pattern.as_deref(), Some("lead with auditability angle"));
    assert!(c.evidence.is_some());
    assert!(c.expected_reuse.is_some());
    assert!(c.risk.is_some());
}

#[test]
fn rejects_non_json_body() {
    assert!(parse_skill_candidate("not JSON").is_none());
    assert!(parse_skill_candidate("").is_none());
}

#[test]
fn rejects_json_array() {
    // Body must be an object — an array is the wrong shape even
    // if it contains valid candidate objects (unambiguous schema).
    let body = r#"[{"schema_key":"learned_skill_candidates","skill_name":"x"}]"#;
    assert!(parse_skill_candidate(body).is_none());
}

#[test]
fn rejects_missing_schema_key() {
    let body = r#"{"skill_name":"x"}"#;
    assert!(parse_skill_candidate(body).is_none());
}

#[test]
fn rejects_wrong_schema_key() {
    // Generic memory rows MUST NOT be picked up as candidates.
    let body = r#"{"schema_key":"icp","skill_name":"x"}"#;
    assert!(parse_skill_candidate(body).is_none());
}

#[test]
fn rejects_missing_skill_name() {
    let body = r#"{"schema_key":"learned_skill_candidates"}"#;
    assert!(parse_skill_candidate(body).is_none());
}

#[test]
fn rejects_empty_skill_name() {
    let body = r#"{"schema_key":"learned_skill_candidates","skill_name":""}"#;
    assert!(parse_skill_candidate(body).is_none());
    let body = r#"{"schema_key":"learned_skill_candidates","skill_name":"   "}"#;
    assert!(parse_skill_candidate(body).is_none());
}

#[test]
fn ignores_unknown_extra_fields_forward_compat() {
    // Verticals may evolve the candidate shape (add fields like
    // `counterexamples` later). The parser must tolerate
    // unknowns silently — old readers + new writers coexist.
    let body = r#"{
        "schema_key":"learned_skill_candidates",
        "skill_name":"x",
        "counterexamples":["future field"],
        "lifecycle_state":"pending_review",
        "scope":{"vertical":"gtm-engineer"}
    }"#;
    let c = parse_skill_candidate(body).expect("must parse despite extras");
    assert_eq!(c.skill_name, "x");
}

#[test]
fn schema_key_constant_is_stable() {
    // Lock the wire constant — verticals + tests reference this
    // string literal across the codebase. Changing it would
    // silently orphan every existing memory row.
    assert_eq!(SCHEMA_KEY_LEARNED_SKILL_CANDIDATE, "learned_skill_candidates");
}
