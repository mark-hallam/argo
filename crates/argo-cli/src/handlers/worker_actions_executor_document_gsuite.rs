//! `DocumentGsuiteExecutor` — adapter between the Worker v3 dispatch
//! trait and the `argo-executor-document-gsuite` crate.
//!
//! Parses `action.payload` into a [`GsuiteWriteRequest`], invokes
//! [`DocumentGsuite::write`] (which is async), and surfaces the report
//! (`doc_id`, `web_view_link`, `mime_type`, `sha256_of_request`,
//! `rejections`) into the action bundle's `result_json` so the audit
//! ledger + downstream verify chains can reach the artifact metadata
//! without re-fetching from Drive.
//!
//! Auth comes from process-level env (the `ARGO_GSUITE_SERVICE_ACCOUNT_EMAIL`
//! and `ARGO_GSUITE_SERVICE_ACCOUNT_KEY` vars); the executor never inlines
//! credentials into the payload. Operators rotate by re-issuing the
//! service-account key and bouncing the gateway.
//!
//! ## Payload shape
//!
//! ```json
//! {
//!   "kind": "Doc",
//!   "title": "Draft kickoff brief",
//!   "drive_folder_id": "1A2B...optional",
//!   "doc_body_text": "Hello world.",
//!   "sheet_schema": [],
//!   "sheet_rows": [],
//!   "slides": []
//! }
//! ```

use super::worker_actions::ActionRecord;
use super::worker_actions_executor::{ActionOutcome, Executor, ExecutorKind};
use argo_executor_document_gsuite::{
    DocumentGsuite, GsuiteAuthConfig, GsuiteWriteRequest,
};
use argo_types::{ArgoError, ArgoResult};
use std::path::{Path, PathBuf};

/// `Executor` impl. Stateless — auth is rebuilt from env per dispatch
/// so credential rotation takes effect without a process restart.
pub(crate) struct DocumentGsuiteExecutor;

impl DocumentGsuiteExecutor {
    pub(crate) fn new() -> Self {
        Self
    }
}

impl Executor for DocumentGsuiteExecutor {
    fn kind(&self) -> ExecutorKind {
        ExecutorKind::DocumentGsuite
    }

    fn dispatch(&self, action: &ActionRecord, cwd: &Path) -> ArgoResult<ActionOutcome> {
        let request = parse_request(&action.payload)?;
        let bundle = ensure_bundle_dir(cwd, &action.job_id, &action.action_id)?;
        let auth = GsuiteAuthConfig::from_env().map_err(|e| {
            ArgoError::Provider(format!(
                "document-gsuite executor not configured: {e}; set \
                 ARGO_GSUITE_SERVICE_ACCOUNT_EMAIL + \
                 ARGO_GSUITE_SERVICE_ACCOUNT_KEY in your environment"
            ))
        })?;
        let report = run_blocking(auth, request)?;
        let rejections_json = serde_json::to_value(&report.rejections)
            .unwrap_or_else(|_| serde_json::Value::Array(Vec::new()));
        Ok(ActionOutcome::Completed {
            bundle_path: bundle,
            result_json: serde_json::json!({
                "outcome": "completed",
                "executor": "document_gsuite",
                "doc_id": report.doc_id.0,
                "web_view_link": report.web_view_link,
                "mime_type": report.mime_type,
                "kind": report.kind.as_lower_str(),
                "created_at": report.created_at,
                "sha256_of_request": report.sha256_of_request,
                "rejections": rejections_json,
            }),
        })
    }
}

/// Parse `action.payload` into a [`GsuiteWriteRequest`]. The payload
/// shape mirrors the public request struct one-to-one; serde handles
/// the field-default semantics (`drive_folder_id` / `doc_body_text` /
/// `sheet_*` / `slides` are all optional per their `#[serde(default)]`).
pub(crate) fn parse_request(payload: &serde_json::Value) -> ArgoResult<GsuiteWriteRequest> {
    serde_json::from_value::<GsuiteWriteRequest>(payload.clone()).map_err(|e| {
        ArgoError::Schema(format!(
            "document-gsuite payload: {e} (need {{ kind, title, ... }})"
        ))
    })
}

/// Build the per-action bundle dir on disk. Mirrors the CSV adapter's
/// shape so the bundle path layout is identical across executors.
fn ensure_bundle_dir(cwd: &Path, job_id: &str, action_id: &str) -> ArgoResult<PathBuf> {
    let dir = cwd
        .join(".skycot")
        .join("outputs")
        .join(job_id)
        .join("actions")
        .join(action_id);
    std::fs::create_dir_all(&dir).map_err(|e| {
        ArgoError::Provider(format!(
            "document-gsuite: create bundle dir {}: {e}",
            dir.display()
        ))
    })?;
    Ok(dir)
}

/// Bridge sync `Executor::dispatch` to the async
/// [`DocumentGsuite::write`]. Mirrors the `block_in_place` pattern
/// used by `worker_actions_executor_mcp::dispatch_blocking` and
/// `worker_actions_executor_stubs::run_blocking` so the cli's tokio
/// runtime keeps driving other futures while this dispatch parks on
/// the Google API.
fn run_blocking(
    auth: GsuiteAuthConfig,
    request: GsuiteWriteRequest,
) -> ArgoResult<argo_executor_document_gsuite::GsuiteWriteReport> {
    let handle = tokio::runtime::Handle::try_current().map_err(|e| {
        ArgoError::Provider(format!(
            "document-gsuite executor needs a tokio runtime to dispatch: {e}"
        ))
    })?;
    tokio::task::block_in_place(|| {
        handle.block_on(async {
            DocumentGsuite::new(auth)
                .write(request)
                .await
                .map_err(|e| ArgoError::Provider(format!("document-gsuite executor: {e}")))
        })
    })
}

#[cfg(test)]
#[path = "worker_actions_executor_document_gsuite_tests.rs"]
mod tests;
