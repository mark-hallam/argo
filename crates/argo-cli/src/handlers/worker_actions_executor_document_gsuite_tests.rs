//! Adapter coverage for `DocumentGsuiteExecutor`. We can't run the
//! full async dispatch path here (it needs a live wiremock server +
//! a tokio runtime), but we exercise: payload parsing happy / sad,
//! env-not-set surfaces a clean Provider error, and the kind
//! round-trip.

use super::*;
use crate::handlers::worker_actions::ActionRecord;
use crate::handlers::worker_actions_executor::ExecutorKind;
use argo_executor_document_gsuite::GsuiteDocKind;
use chrono::Utc;
use serde_json::json;
use tempfile::TempDir;

fn make_action(payload: serde_json::Value) -> ActionRecord {
    ActionRecord {
        schema_key: ActionRecord::SCHEMA_KEY.to_string(),
        action_id: "act-g1".into(),
        job_id: "job-g1".into(),
        worker_id: argo_identity::WorkerId::nil(),
        item_id: None,
        action_type: "create_gsuite_doc".into(),
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
fn parse_request_accepts_minimal_doc_payload() {
    let payload = json!({
        "kind": "Doc",
        "title": "T",
        "doc_body_text": "Hello"
    });
    let req = parse_request(&payload).unwrap();
    assert_eq!(req.kind, GsuiteDocKind::Doc);
    assert_eq!(req.title, "T");
    assert_eq!(req.doc_body_text.as_deref(), Some("Hello"));
    assert!(req.drive_folder_id.is_none());
    assert!(req.slides.is_empty());
}

#[test]
fn parse_request_rejects_missing_required_field() {
    let payload = json!({ "title": "no kind" });
    let err = parse_request(&payload).unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("document-gsuite payload"), "msg: {msg}");
}

#[test]
fn parse_request_rejects_unknown_kind_value() {
    let payload = json!({ "kind": "Form", "title": "T" });
    let err = parse_request(&payload).unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("document-gsuite payload"), "msg: {msg}");
}

#[test]
fn dispatch_without_env_surfaces_clean_provider_error() {
    // Save + clear env so the test is hermetic; restore on drop.
    let restore_email = std::env::var("ARGO_GSUITE_SERVICE_ACCOUNT_EMAIL").ok();
    let restore_key = std::env::var("ARGO_GSUITE_SERVICE_ACCOUNT_KEY").ok();
    // SAFETY: tests are single-threaded by default for this crate's
    // adapter coverage; the env mutation is bounded to this test
    // body and restored before return.
    unsafe {
        std::env::remove_var("ARGO_GSUITE_SERVICE_ACCOUNT_EMAIL");
        std::env::remove_var("ARGO_GSUITE_SERVICE_ACCOUNT_KEY");
    }
    let payload = json!({
        "kind": "Doc",
        "title": "T",
        "doc_body_text": "Hello"
    });
    let action = make_action(payload);
    let cwd = TempDir::new().unwrap();
    let err = DocumentGsuiteExecutor::new()
        .dispatch(&action, cwd.path())
        .unwrap_err();
    // SAFETY: see above; env mutation is bounded.
    unsafe {
        if let Some(v) = restore_email {
            std::env::set_var("ARGO_GSUITE_SERVICE_ACCOUNT_EMAIL", v);
        }
        if let Some(v) = restore_key {
            std::env::set_var("ARGO_GSUITE_SERVICE_ACCOUNT_KEY", v);
        }
    }
    let msg = format!("{err}");
    assert!(
        msg.contains("ARGO_GSUITE_SERVICE_ACCOUNT_EMAIL")
            || msg.contains("not configured"),
        "msg: {msg}"
    );
}

#[test]
fn executor_kind_round_trips_to_document_gsuite() {
    let exec = DocumentGsuiteExecutor::new();
    assert_eq!(exec.kind(), ExecutorKind::DocumentGsuite);
}
