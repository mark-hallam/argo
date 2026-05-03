//! Unit tests for the actuals parser + joiner (#694). Covers
//! the contract every CLI consumer relies on: clean parses,
//! per-line errors with line numbers, duplicate detection,
//! `actual` vs `response` aliasing, and the join semantics
//! (external map wins over row-embedded actual; missing entries
//! preserve `no_actual` status; unknown ids produce a warning
//! but don't fail).
#![cfg(test)]

use super::*;

// ---- parse_actuals_jsonl ----------------------------------------

#[test]
fn parses_minimal_valid_actuals() {
    let body = "{\"id\":\"a\",\"actual\":\"hello\"}\n{\"id\":\"b\",\"actual\":\"world\"}\n";
    let map = parse_actuals_jsonl(body).expect("clean body must parse");
    assert_eq!(map.len(), 2);
    assert_eq!(map.get("a").map(String::as_str), Some("hello"));
    assert_eq!(map.get("b").map(String::as_str), Some("world"));
}

#[test]
fn skips_blank_lines() {
    let body = "\n{\"id\":\"a\",\"actual\":\"hi\"}\n\n";
    let map = parse_actuals_jsonl(body).unwrap();
    assert_eq!(map.len(), 1);
}

#[test]
fn accepts_response_as_alias_for_actual() {
    // The user spec explicitly allows either `actual` or
    // `response` so transcripts captured by an external pipeline
    // (which often uses `response`) can be ingested without
    // rewriting.
    let body = "{\"id\":\"a\",\"response\":\"from llm\"}\n";
    let map = parse_actuals_jsonl(body).unwrap();
    assert_eq!(map.get("a").map(String::as_str), Some("from llm"));
}

#[test]
fn actual_wins_when_both_actual_and_response_present() {
    let body = "{\"id\":\"a\",\"actual\":\"primary\",\"response\":\"alias\"}\n";
    let map = parse_actuals_jsonl(body).unwrap();
    assert_eq!(map.get("a").map(String::as_str), Some("primary"));
}

#[test]
fn flags_invalid_json_with_line_number() {
    let body = "{\"id\":\"a\",\"actual\":\"ok\"}\nNOT JSON\n";
    let errs = parse_actuals_jsonl(body).expect_err("must error");
    assert_eq!(errs.len(), 1);
    assert_eq!(errs[0].line, 2);
    assert!(errs[0].reason.contains("invalid JSON"));
}

#[test]
fn flags_missing_id() {
    let body = "{\"actual\":\"x\"}\n";
    let errs = parse_actuals_jsonl(body).expect_err("must error");
    assert_eq!(errs.len(), 1);
    assert!(errs[0].reason.contains("missing `id`"));
}

#[test]
fn flags_empty_id() {
    let body = "{\"id\":\"\",\"actual\":\"x\"}\n";
    let errs = parse_actuals_jsonl(body).expect_err("must error");
    assert_eq!(errs.len(), 1);
    assert!(errs[0].reason.contains("non-empty"));
}

#[test]
fn flags_missing_both_actual_and_response() {
    let body = "{\"id\":\"a\"}\n";
    let errs = parse_actuals_jsonl(body).expect_err("must error");
    assert_eq!(errs.len(), 1);
    assert!(
        errs[0].reason.contains("actual") && errs[0].reason.contains("response"),
        "reason={}",
        errs[0].reason
    );
}

#[test]
fn flags_duplicate_id_with_line_number() {
    let body =
        "{\"id\":\"a\",\"actual\":\"first\"}\n{\"id\":\"a\",\"actual\":\"dup\"}\n";
    let errs = parse_actuals_jsonl(body).expect_err("must error");
    assert_eq!(errs.len(), 1);
    assert_eq!(errs[0].line, 2);
    assert!(errs[0].reason.contains("duplicate"));
    assert!(errs[0].reason.contains("`a`"));
}

#[test]
fn collects_multiple_errors_in_one_pass() {
    let body =
        "NOT JSON\n{\"id\":\"a\",\"actual\":\"ok\"}\n{\"id\":\"a\",\"actual\":\"dup\"}\n";
    let errs = parse_actuals_jsonl(body).expect_err("must error");
    assert_eq!(errs.len(), 2);
    assert_eq!(errs[0].line, 1);
    assert_eq!(errs[1].line, 3);
}

// ---- score_suite_jsonl_with_actuals ------------------------------

#[test]
fn external_actual_overlays_onto_fixture_for_scoring() {
    // The fixture row carries no `actual`. The actuals map
    // provides one. Result: the fixture is scored as Scored,
    // not NoActual.
    let suite =
        "{\"id\":\"a\",\"input\":\"\",\"expected\":{\"must_include\":[\"hello\"]}}\n";
    let mut actuals = ActualsMap::new();
    actuals.insert("a".into(), "hello world".into());
    let (score, unknown) = score_suite_jsonl_with_actuals(suite, &actuals);
    assert_eq!(score.scored_count, 1);
    assert_eq!(score.passed, 1);
    assert_eq!(score.no_actual, 0);
    assert!(unknown.is_empty());
}

#[test]
fn external_actual_wins_over_row_embedded_actual() {
    // Setup: fixture row has actual "FROM_ROW" (which would
    // pass must_include=["FROM_ROW"]); external map provides
    // "FROM_FILE" instead. After the join, scoring against
    // must_include=["FROM_ROW"] FAILS — proving the external
    // actual won.
    let suite = "{\"id\":\"a\",\"input\":\"\",\"expected\":{\"must_include\":[\"FROM_ROW\"]},\"actual\":\"FROM_ROW\"}\n";
    let mut actuals = ActualsMap::new();
    actuals.insert("a".into(), "FROM_FILE".into());
    let (score, _) = score_suite_jsonl_with_actuals(suite, &actuals);
    assert_eq!(score.scored_count, 1);
    assert_eq!(
        score.failed, 1,
        "row's own FROM_ROW actual should have been overridden by external FROM_FILE → must_include[FROM_ROW] should fail",
    );
    // Symmetric proof: scoring the same suite against
    // must_include=["FROM_FILE"] PASSES because the external
    // actual is the one that gets seen.
    let suite2 = "{\"id\":\"a\",\"input\":\"\",\"expected\":{\"must_include\":[\"FROM_FILE\"]},\"actual\":\"FROM_ROW\"}\n";
    let (score2, _) = score_suite_jsonl_with_actuals(suite2, &actuals);
    assert_eq!(score2.passed, 1, "external actual should have scored");
}

#[test]
fn missing_actual_remains_no_actual() {
    let suite = "{\"id\":\"a\",\"input\":\"\",\"expected\":{\"must_include\":[\"x\"]}}\n";
    let actuals = ActualsMap::new(); // empty
    let (score, _) = score_suite_jsonl_with_actuals(suite, &actuals);
    assert_eq!(score.no_actual, 1);
    assert_eq!(score.scored_count, 0);
}

#[test]
fn unknown_actuals_ids_surface_as_warning() {
    // Operator may have captured more responses than fixtures
    // exist (e.g. retries). These don't fail the run; they're
    // a heads-up that the actuals file is out of sync.
    let suite = "{\"id\":\"a\",\"input\":\"\",\"expected\":{}}\n";
    let mut actuals = ActualsMap::new();
    actuals.insert("a".into(), "ok".into());
    actuals.insert("ghost".into(), "no fixture for me".into());
    actuals.insert("phantom".into(), "me neither".into());
    let (_, unknown) = score_suite_jsonl_with_actuals(suite, &actuals);
    assert_eq!(unknown.len(), 2);
    let mut sorted = unknown;
    sorted.sort();
    assert_eq!(sorted, vec!["ghost".to_string(), "phantom".to_string()]);
}

#[test]
fn malformed_fixture_row_still_surfaces_as_malformed() {
    // The actuals file shouldn't rescue a bad fixture row —
    // structural errors in the suite remain visible.
    let suite = "NOT JSON\n";
    let actuals = ActualsMap::new();
    let (score, _) = score_suite_jsonl_with_actuals(suite, &actuals);
    assert_eq!(score.malformed, 1);
    assert_eq!(score.scored_count, 0);
}

#[test]
fn empty_actuals_map_behaves_like_unscored_path() {
    // When the map is empty + no row carries actual, the result
    // is byte-identical to score_suite_jsonl — proves the
    // overlay is purely additive.
    use crate::eval_score::score_suite_jsonl;
    let suite = "{\"id\":\"a\",\"input\":\"\",\"expected\":{\"must_include\":[\"x\"]}}\n\
                 {\"id\":\"b\",\"input\":\"\",\"expected\":{},\"actual\":\"present\"}\n";
    let baseline = score_suite_jsonl(suite);
    let (overlay, _) = score_suite_jsonl_with_actuals(suite, &ActualsMap::new());
    assert_eq!(baseline.fixture_count, overlay.fixture_count);
    assert_eq!(baseline.scored_count, overlay.scored_count);
    assert_eq!(baseline.no_actual, overlay.no_actual);
    assert_eq!(baseline.passed, overlay.passed);
    assert_eq!(baseline.failed, overlay.failed);
}
