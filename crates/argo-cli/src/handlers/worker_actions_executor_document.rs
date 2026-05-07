//! `DocumentExecutor` — V1.2 document-creation executor for the
//! Worker v3 dispatch trait (Stream A `#A-executor-trait`).
//!
//! Routes the action's payload into one of four backends — markdown,
//! CSV, PDF (markdown→headless-Chrome), or slides (markdown with
//! `---` separators → reveal.js HTML). Outputs land beneath the
//! per-action bundle dir
//! (`<cwd>/.skycot/outputs/<job_id>/<action_id>/`); the
//! `argo-executor-document` crate's `OutputPath::resolve` rejects
//! absolute paths + parent traversal so a poisoned manifest can't
//! escape the bundle.
//!
//! ## Wiring
//!
//! `[actions.<name>] executor = "document_creation"` (or the tagged
//! form `executor = { kind = "document_creation", ... }`) routes
//! the action through this impl. The auto-approval gate sees
//! [`ExecutorKind::DocumentCreation`] and applies the workspace's
//! standard policy — DocumentCreation is approval-gated by default
//! since outputs are user-visible.
//!
//! ## Payload shape
//!
//! ```json
//! { "kind": "markdown" | "csv" | "pdf" | "slides",
//!   "output_path": "report.md",         // required, all kinds
//!   "body": "# Hello",                   // markdown / pdf / slides
//!   "title": "Doc",                      // pdf / slides
//!   "header": ["a", "b"], "rows": [...]  // csv only
//! }
//! ```
//!
//! Unrecognised `kind` → `parse_request` returns
//! [`ArgoError::Provider`] with a clear message listing the four
//! valid values.

use super::worker_actions::ActionRecord;
use super::worker_actions_executor::{ActionOutcome, Executor, ExecutorKind};
use argo_executor_document::{DocExecutor, DocRequest, DOC_KINDS};
use argo_types::{ArgoError, ArgoResult};
use std::path::{Path, PathBuf};

/// `Executor` impl. Holds nothing — every dispatch is independent.
pub(crate) struct DocumentExecutor;

impl DocumentExecutor {
    pub(crate) fn new() -> Self {
        Self
    }
}

impl Executor for DocumentExecutor {
    fn kind(&self) -> ExecutorKind {
        ExecutorKind::DocumentCreation
    }

    fn dispatch(&self, action: &ActionRecord, cwd: &Path) -> ArgoResult<ActionOutcome> {
        let request = parse_request(&action.payload)?;
        let bundle = ensure_bundle_dir(cwd, &action.job_id, &action.action_id)?;
        let exec = DocExecutor::new();
        let outcome = exec.run(&request, &bundle).map_err(|e| {
            ArgoError::Provider(format!("document-creation executor: {e}"))
        })?;
        Ok(ActionOutcome::Completed {
            bundle_path: bundle,
            result_json: serde_json::json!({
                "outcome": "completed",
                "executor": "document_creation",
                "kind": outcome.kind,
                "output_path": outcome.output_path.display().to_string(),
                "bytes_written": outcome.bytes_written,
            }),
        })
    }
}

/// Parse `action.payload` into a [`DocRequest`]. Surfaces a clean
/// error message when the `kind` field is missing or unknown so the
/// audit ledger captures the gap.
pub(crate) fn parse_request(payload: &serde_json::Value) -> ArgoResult<DocRequest> {
    if let Some(kind) = payload.get("kind").and_then(|v| v.as_str()) {
        if !DOC_KINDS.contains(&kind) {
            return Err(ArgoError::Provider(format!(
                "document-creation: unknown kind '{kind}' — valid: {}",
                DOC_KINDS.join(" | ")
            )));
        }
    } else {
        return Err(ArgoError::Provider(format!(
            "document-creation payload: missing 'kind' field — valid: {}",
            DOC_KINDS.join(" | ")
        )));
    }
    serde_json::from_value(payload.clone()).map_err(|e| {
        ArgoError::Provider(format!(
            "document-creation payload: {e} (need {{ kind, output_path, ... }})"
        ))
    })
}

/// Build the per-action bundle dir on disk. Mirrors
/// `worker_actions_executor_code::ensure_bundle_dir`.
fn ensure_bundle_dir(cwd: &Path, job_id: &str, action_id: &str) -> ArgoResult<PathBuf> {
    let dir = cwd
        .join(".skycot")
        .join("outputs")
        .join(job_id)
        .join("actions")
        .join(action_id);
    std::fs::create_dir_all(&dir).map_err(|e| {
        ArgoError::Provider(format!(
            "document-creation: create bundle dir {}: {e}",
            dir.display()
        ))
    })?;
    Ok(dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handlers::worker_actions::ActionRecord;
    use chrono::Utc;
    use serde_json::json;

    fn make_action(payload: serde_json::Value) -> ActionRecord {
        ActionRecord {
            schema_key: ActionRecord::SCHEMA_KEY.to_string(),
            action_id: "act-1".into(),
            job_id: "job-1".into(),
            worker_id: argo_identity::WorkerId::nil(),
            item_id: None,
            action_type: "create_doc".into(),
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
    fn parse_request_accepts_markdown_payload() {
        let payload = json!({
            "kind": "markdown",
            "output_path": "report.md",
            "body": "# Hello",
        });
        let req = parse_request(&payload).unwrap();
        assert!(matches!(req, DocRequest::Markdown { .. }));
    }

    #[test]
    fn parse_request_accepts_csv_payload_with_bare_rows() {
        let payload = json!({
            "kind": "csv",
            "output_path": "data.csv",
            "header": ["a", "b"],
            "rows": [["1", "2"], ["3", "4"]],
        });
        let req = parse_request(&payload).unwrap();
        assert!(matches!(req, DocRequest::Csv { .. }));
    }

    #[test]
    fn parse_request_rejects_unknown_kind_with_listing() {
        let payload = json!({
            "kind": "epub",
            "output_path": "x.epub",
        });
        let err = parse_request(&payload).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("unknown kind 'epub'"));
        assert!(msg.contains("markdown"));
    }

    #[test]
    fn parse_request_rejects_missing_kind() {
        let payload = json!({ "output_path": "x.md" });
        let err = parse_request(&payload).unwrap_err();
        assert!(format!("{err}").contains("missing 'kind' field"));
    }

    #[test]
    fn dispatch_completes_markdown_action_into_bundle() {
        let cwd = tempfile::tempdir().unwrap();
        let action = make_action(json!({
            "kind": "markdown",
            "output_path": "out.md",
            "body": "# Hi",
        }));
        let exec = DocumentExecutor::new();
        let outcome = exec.dispatch(&action, cwd.path()).unwrap();
        match outcome {
            ActionOutcome::Completed {
                bundle_path,
                result_json,
            } => {
                assert!(bundle_path
                    .ends_with(".skycot/outputs/job-1/actions/act-1"));
                assert_eq!(result_json["executor"].as_str().unwrap(), "document_creation");
                assert_eq!(result_json["kind"].as_str().unwrap(), "markdown");
                assert_eq!(result_json["bytes_written"].as_u64().unwrap(), 4);
            }
            ActionOutcome::HumanHandoff { .. } => panic!("expected Completed"),
        }
    }

    #[test]
    fn dispatch_propagates_path_escape_error_with_executor_prefix() {
        let cwd = tempfile::tempdir().unwrap();
        let action = make_action(json!({
            "kind": "markdown",
            "output_path": "/etc/passwd",
            "body": "x",
        }));
        let exec = DocumentExecutor::new();
        let err = exec.dispatch(&action, cwd.path()).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("document-creation executor"), "msg: {msg}");
        assert!(msg.contains("escapes"), "msg: {msg}");
    }

    #[test]
    fn executor_kind_round_trips_to_executorkind_document_creation() {
        let exec = DocumentExecutor::new();
        assert_eq!(exec.kind(), ExecutorKind::DocumentCreation);
    }
}
