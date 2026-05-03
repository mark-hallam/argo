//! Actuals-file support for scored evals (#694). Sibling of
//! `eval_score.rs` so that file stays under the 500-LOC budget.
//!
//! ## Why a separate file
//!
//! The canonical eval suite (`eval/<name>.jsonl`) carries
//! `{id, input, expected}` per row — the contract the agent is
//! graded against. Captured agent transcripts (or hand-curated
//! known-good reference responses) live in a parallel JSONL
//! that pairs `{id, actual|response}` per row. This file owns:
//!
//! 1. parsing the actuals JSONL into a typed map
//! 2. joining by `id` so the scorer sees each fixture's actual
//!    text without mutating the canonical fixtures
//!
//! Both steps return clear, line-numbered errors so the operator
//! can fix the actuals file from a single CI failure message.
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;

use crate::eval_score::{
    score_fixture, FixtureScore, FixtureStatus, SuiteScore,
};

/// Parsed actuals map: fixture id → actual text. Built by
/// [`parse_actuals_jsonl`]; consumed by
/// [`score_suite_jsonl_with_actuals`].
pub type ActualsMap = HashMap<String, String>;

/// Per-line failure detail. Caller surfaces these in the CLI's
/// scored-eval report so the operator sees exactly which actuals
/// row needs fixing without grepping the file.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ActualsParseError {
    pub line: usize,
    pub reason: String,
}

/// Parse an actuals JSONL body. Each row must:
/// - parse as JSON
/// - have a non-empty `id` string
/// - have either an `actual` or a `response` string field
///   (`actual` wins when both are present)
///
/// Returns `Err(Vec<…>)` listing every problem. Duplicate ids
/// are also reported here — actuals files are expected to be
/// 1:1 with fixtures, so a duplicate is almost always an
/// authoring mistake we want to surface loudly.
pub fn parse_actuals_jsonl(body: &str) -> Result<ActualsMap, Vec<ActualsParseError>> {
    let mut out: ActualsMap = HashMap::new();
    let mut errors: Vec<ActualsParseError> = Vec::new();
    for (i, raw) in body.lines().enumerate() {
        let line = i + 1;
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }
        let parsed: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(e) => {
                errors.push(ActualsParseError {
                    line,
                    reason: format!("invalid JSON: {e}"),
                });
                continue;
            }
        };
        let id = match parsed.get("id").and_then(Value::as_str) {
            Some(s) if !s.is_empty() => s.to_string(),
            Some(_) => {
                errors.push(ActualsParseError {
                    line,
                    reason: "`id` must be a non-empty string".into(),
                });
                continue;
            }
            None => {
                errors.push(ActualsParseError {
                    line,
                    reason: "missing `id` field".into(),
                });
                continue;
            }
        };
        let actual = parsed
            .get("actual")
            .and_then(Value::as_str)
            .or_else(|| parsed.get("response").and_then(Value::as_str));
        let Some(actual) = actual else {
            errors.push(ActualsParseError {
                line,
                reason: "missing both `actual` and `response` string fields".into(),
            });
            continue;
        };
        if out.contains_key(&id) {
            errors.push(ActualsParseError {
                line,
                reason: format!("duplicate id `{id}` (actuals must be 1:1 with fixtures)"),
            });
            continue;
        }
        out.insert(id, actual.to_string());
    }
    if errors.is_empty() {
        Ok(out)
    } else {
        Err(errors)
    }
}

/// Score a suite against an external actuals map. Behaviour:
///
/// - For each fixture row, the actuals map is consulted first by
///   `id`. When the map has an entry, the row's own `actual`
///   field (if any) is overridden — the external file is the
///   source of truth.
/// - When neither the map nor the row carries an actual, the
///   fixture surfaces as `FixtureStatus::NoActual` (same as the
///   no-actuals scoring path — coverage warning, not regression).
/// - Returns the suite rollup plus the `unknown_ids` warning
///   list (actuals rows that don't match any fixture id).
pub fn score_suite_jsonl_with_actuals(
    body: &str,
    actuals: &ActualsMap,
) -> (SuiteScore, Vec<String>) {
    // Build the set of fixture ids first so we can compute
    // unknown_ids without a second pass.
    let mut fixture_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut fixtures: Vec<FixtureScore> = Vec::new();
    for raw in body.lines() {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }
        let mut parsed: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(e) => {
                fixtures.push(FixtureScore {
                    id: "(unparseable)".into(),
                    status: FixtureStatus::Malformed,
                    checks: Vec::new(),
                    passed: false,
                    fail_reasons: vec![format!("invalid JSON: {e}")],
                });
                continue;
            }
        };
        if let Some(id) = parsed.get("id").and_then(Value::as_str) {
            fixture_ids.insert(id.to_string());
            // External actuals map wins — overlay onto the row
            // before scoring so the engine sees a normal-shaped
            // fixture without needing two code paths.
            if let Some(actual) = actuals.get(id) {
                if let Some(obj) = parsed.as_object_mut() {
                    obj.insert(
                        "actual".to_string(),
                        Value::String(actual.clone()),
                    );
                }
            }
        }
        fixtures.push(score_fixture(&parsed));
    }
    let unknown_ids: Vec<String> = actuals
        .keys()
        .filter(|id| !fixture_ids.contains(*id))
        .cloned()
        .collect();
    (SuiteScore::from_fixtures(fixtures), unknown_ids)
}

#[cfg(test)]
#[path = "eval_score_actuals_tests.rs"]
mod tests;
