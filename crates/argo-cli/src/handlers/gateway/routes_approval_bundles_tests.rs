//! Pure-function tests for [`routes_approval_bundles`] — covers the
//! JSON parsing, query-string parsing, and member-array validation
//! paths. The end-to-end "create → decide" round-trip needs a
//! workspace + memory store + gateway listener stack, exercised by
//! the broader gateway smoke suite (`tests/gateway_*`).
#![cfg(test)]
use serde_json::json;

use super::{
    bad_request_for_test, conflict_for_test, not_found_for_test, ok_decide_for_test,
    parse_limit_for_test, parse_members_for_test, parse_query_for_test,
};

#[test]
fn parse_query_extracts_known_key() {
    let tail = "/api/v1/approvals/bundles?status=pending&limit=10 HTTP/1.1";
    assert_eq!(
        parse_query_for_test(tail, "status"),
        Some("pending".to_string())
    );
    assert_eq!(parse_query_for_test(tail, "limit"), Some("10".to_string()));
    assert_eq!(parse_query_for_test(tail, "missing"), None);
}

#[test]
fn parse_limit_defaults_and_validates() {
    assert_eq!(
        parse_limit_for_test("/api/v1/approvals/bundles HTTP/1.1").unwrap(),
        50
    );
    assert_eq!(
        parse_limit_for_test("/api/v1/approvals/bundles?limit=7 HTTP/1.1").unwrap(),
        7
    );
    let err = parse_limit_for_test("/api/v1/approvals/bundles?limit=abc HTTP/1.1")
        .expect_err("non-number must reject");
    assert_eq!(err.0, "400 Bad Request");
    let err = parse_limit_for_test("/api/v1/approvals/bundles?limit=0 HTTP/1.1")
        .expect_err("0 must reject");
    assert!(err.1.contains("1..=100"));
    let err = parse_limit_for_test("/api/v1/approvals/bundles?limit=999 HTTP/1.1")
        .expect_err("over-cap must reject");
    assert!(err.1.contains("1..=100"));
}

#[test]
fn parse_members_accepts_valid_array() {
    let v = json!(["a", "b", "c"]);
    let out = parse_members_for_test(Some(&v)).unwrap();
    assert_eq!(out, vec!["a".to_string(), "b".to_string(), "c".to_string()]);
}

#[test]
fn parse_members_rejects_missing_or_empty() {
    let err = parse_members_for_test(None).expect_err("missing must reject");
    assert!(err.contains("array"));
    let empty = json!([]);
    let err = parse_members_for_test(Some(&empty)).expect_err("empty must reject");
    assert!(err.contains("at least one"));
    let mixed = json!(["a", 7]);
    let err = parse_members_for_test(Some(&mixed)).expect_err("non-string must reject");
    assert!(err.contains("strings"));
    let blank = json!(["a", "   "]);
    let err = parse_members_for_test(Some(&blank)).expect_err("blank must reject");
    assert!(err.contains("non-empty"));
}

#[test]
fn error_helpers_carry_envelope_shape() {
    let (status, body) = bad_request_for_test("nope");
    assert_eq!(status, "400 Bad Request");
    assert!(body.contains("\"bad_request\""));
    assert!(body.contains("nope"));

    let (status, body) = not_found_for_test("xyz");
    assert_eq!(status, "404 Not Found");
    assert!(body.contains("bundle_not_found"));
    assert!(body.contains("xyz"));

    let (status, body) = conflict_for_test("xyz", "approve");
    assert_eq!(status, "409 Conflict");
    assert!(body.contains("already_decided"));
    assert!(body.contains("approve"));
}

#[test]
fn ok_decide_envelope_has_counts() {
    let (status, body) = ok_decide_for_test("b1", "approve", 5, 2);
    assert_eq!(status, "200 OK");
    assert!(body.contains("\"bundle_id\":\"b1\""));
    assert!(body.contains("\"decision\":\"approve\""));
    assert!(body.contains("\"members_decided\":5"));
    assert!(body.contains("\"members_skipped\":2"));
}
