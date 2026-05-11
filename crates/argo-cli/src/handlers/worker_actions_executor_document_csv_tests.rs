//! Adapter coverage for `DocumentCsvExecutor`.

use super::*;
use crate::handlers::worker_actions::ActionRecord;
use crate::handlers::worker_actions_executor::ExecutorKind;
use chrono::Utc;
use serde_json::json;
use tempfile::TempDir;

fn make_action(payload: serde_json::Value) -> ActionRecord {
    ActionRecord {
        schema_key: ActionRecord::SCHEMA_KEY.to_string(),
        action_id: "act-1".into(),
        job_id: "job-1".into(),
        worker_id: argo_identity::WorkerId::nil(),
        item_id: None,
        action_type: "create_csv".into(),
        risk_level: "medium".into(),
        requires_approval: true,
        state: "executing".into(),
        payload,
        approved_by: None,
        approved_at: None,
        executed_at: None,
        platform: None,
        result: None,
        evidence: Vec::new(),
        blocked_reason: None,
        created_at: Utc::now().to_rfc3339(),
    }
}

#[test]
fn parse_request_accepts_minimal_payload_and_defaults_quote_minimal_true() {
    let root = TempDir::new().unwrap();
    let payload = json!({
        "output_root": root.path().display().to_string(),
        "path": "out.csv",
        "schema": [
            { "header": "a", "kind": "text", "required": false }
        ],
        "rows": [ { "a": "hi" } ]
    });
    let req = parse_request(&payload).unwrap();
    assert_eq!(req.path.to_string_lossy(), "out.csv");
    assert!(req.include_header);
    assert!(req.quote_minimal);
    assert_eq!(req.schema.len(), 1);
}

#[test]
fn dispatch_sandbox_violation_surfaces_as_executor_error() {
    let root = TempDir::new().unwrap();
    let payload = json!({
        "output_root": root.path().display().to_string(),
        "path": "../escape.csv",
        "schema": [
            { "header": "a", "kind": "text", "required": false }
        ],
        "rows": []
    });
    let action = make_action(payload);
    let cwd = TempDir::new().unwrap();
    let err = DocumentCsvExecutor::new()
        .dispatch(&action, cwd.path())
        .unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("document-csv executor"), "msg: {msg}");
    assert!(msg.contains("sandbox"), "msg: {msg}");
}

#[test]
fn dispatch_surfaces_typed_rejection_in_bundle_result() {
    let root = TempDir::new().unwrap();
    let payload = json!({
        "output_root": root.path().display().to_string(),
        "path": "out.csv",
        "schema": [
            { "header": "a", "kind": "text", "required": true }
        ],
        "rows": [
            { "a": "ok" },
            { "b": "missing-a" }
        ]
    });
    let action = make_action(payload);
    let cwd = TempDir::new().unwrap();
    let outcome = DocumentCsvExecutor::new()
        .dispatch(&action, cwd.path())
        .unwrap();
    match outcome {
        ActionOutcome::Completed { result_json, .. } => {
            assert_eq!(result_json["rows_written"].as_u64().unwrap(), 1);
            assert_eq!(result_json["rows_rejected"].as_u64().unwrap(), 1);
            let rejs = result_json["rejections"].as_array().unwrap();
            assert_eq!(rejs.len(), 1);
            assert_eq!(rejs[0]["reason"].as_str().unwrap(), "missing_required");
        }
        ActionOutcome::HumanHandoff { .. } => panic!("expected Completed"),
    }
}

#[test]
fn dispatch_bundle_carries_sha256_path_and_row_counts() {
    let root = TempDir::new().unwrap();
    let payload = json!({
        "output_root": root.path().display().to_string(),
        "path": "out.csv",
        "schema": [
            { "header": "a", "kind": "text", "required": false }
        ],
        "rows": [ { "a": "x" }, { "a": "y" } ]
    });
    let action = make_action(payload);
    let cwd = TempDir::new().unwrap();
    let outcome = DocumentCsvExecutor::new()
        .dispatch(&action, cwd.path())
        .unwrap();
    match outcome {
        ActionOutcome::Completed {
            bundle_path,
            result_json,
        } => {
            assert!(bundle_path.ends_with(".skycot/outputs/job-1/actions/act-1"));
            assert_eq!(result_json["executor"].as_str().unwrap(), "document_csv");
            assert!(result_json["sha256"].as_str().unwrap().len() == 64);
            assert!(result_json["bytes_written"].as_u64().unwrap() > 0);
            assert_eq!(result_json["rows_written"].as_u64().unwrap(), 2);
            assert!(result_json["output_path"].as_str().unwrap().ends_with("out.csv"));
        }
        ActionOutcome::HumanHandoff { .. } => panic!("expected Completed"),
    }
}

#[test]
fn executor_kind_round_trips_to_document_csv() {
    let exec = DocumentCsvExecutor::new();
    assert_eq!(exec.kind(), ExecutorKind::DocumentCsv);
}
