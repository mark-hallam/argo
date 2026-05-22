//! Unit coverage for [`super::routes_workflows_test_runs`].
//! Path parsing + `limit=` clamping + body decoding are validated in
//! isolation; the cross-tenant 404 / empty-list guarantee is
//! exercised at the store layer in
//! `crates/argo-store/src/workflow_test_run.rs::tests::cross_tenant_lookup_returns_empty`.

use super::{decode_record, parse_limit, parse_path};
use serde_json::json;

#[test]
fn parse_path_recognises_test_runs_collection() {
    for verb in ["GET ", "POST "] {
        let req = format!(
            "{verb}/api/v1/verticals/pr_checker/workflows/standard_review/test_runs HTTP/1.1\r\nHost: x\r\n\r\n"
        );
        let parsed = parse_path(&req, verb).expect("must parse");
        assert_eq!(parsed.0, "pr_checker");
        assert_eq!(parsed.1, "standard_review");
        assert_eq!(parsed.2, "");
    }
}

#[test]
fn parse_path_carries_query_string() {
    let req = "GET /api/v1/verticals/pr_checker/workflows/standard_review/test_runs?limit=25 HTTP/1.1\r\nHost: x\r\n\r\n";
    let (vertical, name, query) = parse_path(req, "GET ").expect("must parse");
    assert_eq!(vertical, "pr_checker");
    assert_eq!(name, "standard_review");
    assert_eq!(query, "limit=25");
}

#[test]
fn parse_path_rejects_non_test_runs_suffix() {
    let req = "GET /api/v1/verticals/v/workflows/n/rollback HTTP/1.1\r\nHost: x\r\n\r\n";
    assert!(parse_path(req, "GET ").is_none());
    let req2 = "GET /api/v1/verticals/v/workflows/n HTTP/1.1\r\nHost: x\r\n\r\n";
    assert!(parse_path(req2, "GET ").is_none());
}

#[test]
fn parse_path_rejects_hyphenated_legacy_suffix() {
    // The original closed-as-stale PR #2021 cited the path with a
    // hyphen (`test-runs`); the brief locks the underscore form
    // (`test_runs`). Regression guard so we don't silently accept the
    // wrong one.
    let req = "GET /api/v1/verticals/v/workflows/n/test-runs HTTP/1.1\r\nHost: x\r\n\r\n";
    assert!(parse_path(req, "GET ").is_none());
}

#[test]
fn parse_limit_defaults_when_query_empty() {
    assert_eq!(parse_limit(""), 10);
}

#[test]
fn parse_limit_reads_explicit_limit() {
    assert_eq!(parse_limit("limit=25"), 25);
}

#[test]
fn parse_limit_clamps_below_one() {
    assert_eq!(parse_limit("limit=0"), 1);
    assert_eq!(parse_limit("limit=-5"), 1);
}

#[test]
fn parse_limit_clamps_above_max() {
    assert_eq!(parse_limit("limit=1000"), 100);
}

#[test]
fn parse_limit_falls_back_for_garbage() {
    assert_eq!(parse_limit("limit=abc"), 10);
    assert_eq!(parse_limit("other=5"), 10);
}

#[test]
fn decode_record_accepts_minimal_complete_payload() {
    let body = json!({
        "status": "complete",
        "started_at": 1_716_336_600_000_i64,
        "ended_at": 1_716_336_605_000_i64,
    });
    let record = decode_record(&body).expect("must decode");
    assert_eq!(record.status, "complete");
    assert_eq!(record.started_at, 1_716_336_600_000);
    assert_eq!(record.ended_at, 1_716_336_605_000);
    assert_eq!(record.summary_json, "null");
    assert_eq!(record.step_states_json, "{}");
    assert!(record.transport_error.is_none());
}

#[test]
fn decode_record_accepts_full_payload() {
    let body = json!({
        "status": "failed",
        "started_at": 1_716_336_600_000_i64,
        "ended_at": 1_716_336_607_000_i64,
        "summary": {"ok": 1, "fail": 1, "durationMs": 7000},
        "step_states": {"step1": {"status": "error", "error": "boom"}},
        "transport_error": null,
    });
    let record = decode_record(&body).expect("must decode");
    assert_eq!(record.status, "failed");
    assert!(record.summary_json.contains("\"ok\":1"));
    assert!(record.step_states_json.contains("step1"));
    assert!(record.transport_error.is_none());
}

#[test]
fn decode_record_captures_transport_error_string() {
    let body = json!({
        "status": "failed",
        "started_at": 1_i64,
        "ended_at": 2_i64,
        "transport_error": "network: connection reset",
    });
    let record = decode_record(&body).expect("must decode");
    assert_eq!(
        record.transport_error.as_deref(),
        Some("network: connection reset")
    );
}

#[test]
fn decode_record_rejects_invalid_status() {
    let body = json!({
        "status": "passed",
        "started_at": 1_i64,
        "ended_at": 2_i64,
    });
    assert!(decode_record(&body).is_err());
}

#[test]
fn decode_record_rejects_missing_status() {
    let body = json!({
        "started_at": 1_i64,
        "ended_at": 2_i64,
    });
    let err = decode_record(&body).unwrap_err();
    assert!(err.contains("status"), "got: {err}");
}

#[test]
fn decode_record_rejects_missing_started_at() {
    let body = json!({
        "status": "complete",
        "ended_at": 2_i64,
    });
    let err = decode_record(&body).unwrap_err();
    assert!(err.contains("started_at"), "got: {err}");
}

#[test]
fn decode_record_rejects_non_positive_timestamps() {
    let body = json!({
        "status": "complete",
        "started_at": 0_i64,
        "ended_at": 0_i64,
    });
    let err = decode_record(&body).unwrap_err();
    assert!(
        err.contains("positive Unix-millis"),
        "got: {err}"
    );
}

#[test]
fn decode_record_rejects_object_transport_error() {
    let body = json!({
        "status": "failed",
        "started_at": 1_i64,
        "ended_at": 2_i64,
        "transport_error": {"code": "x"},
    });
    let err = decode_record(&body).unwrap_err();
    assert!(err.contains("transport_error"), "got: {err}");
}
