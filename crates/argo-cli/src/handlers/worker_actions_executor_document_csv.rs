//! `DocumentCsvExecutor` — adapter between the Worker v3 dispatch
//! trait and the `argo-executor-document-csv` crate.
//!
//! Maps `action.payload` into a [`CsvWriteRequest`], invokes
//! [`DocumentCsv::write`], and surfaces the report (sha256, row
//! counts, rejections, target path) into the action bundle's
//! `result_json` so the audit ledger + downstream verify chains can
//! reach the artifact metadata without reading the file.
//!
//! ## Payload shape
//!
//! ```json
//! {
//!   "output_root": "/home/op/.argo/exports/csv",
//!   "path": "contacts-2026-05-11.csv",
//!   "schema": [
//!     { "header": "name",  "kind": "text",          "required": true,  "max_len": 64 },
//!     { "header": "email", "kind": "email_address", "required": true,  "max_len": 254 }
//!   ],
//!   "rows": [ { "name": "Ada", "email": "ada@example.com" } ],
//!   "include_header": true,
//!   "line_terminator": "crlf",
//!   "quote_minimal": true,
//!   "write_bom": false
//! }
//! ```
//!
//! `output_root` is optional in the payload; absence falls through to
//! the `[document_csv].output_root` config default. The dispatch
//! shim resolves the config lookup before calling `dispatch` so the
//! adapter stays config-free.

use super::worker_actions::ActionRecord;
use super::worker_actions_executor::{ActionOutcome, Executor, ExecutorKind};
use argo_executor_document_csv::{
    CsvColumnSchema, CsvWriteRequest, DocumentCsv, LineTerminator, DEFAULT_OUTPUT_ROOT,
};
use argo_types::{ArgoError, ArgoResult};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// `Executor` impl. Stateless — every dispatch is independent.
pub(crate) struct DocumentCsvExecutor;

impl DocumentCsvExecutor {
    pub(crate) fn new() -> Self {
        Self
    }
}

impl Executor for DocumentCsvExecutor {
    fn kind(&self) -> ExecutorKind {
        ExecutorKind::DocumentCsv
    }

    fn dispatch(&self, action: &ActionRecord, cwd: &Path) -> ArgoResult<ActionOutcome> {
        let request = parse_request(&action.payload)?;
        let bundle = ensure_bundle_dir(cwd, &action.job_id, &action.action_id)?;
        let report = DocumentCsv::new()
            .write(request)
            .map_err(|e| ArgoError::Provider(format!("document-csv executor: {e}")))?;
        let rejections_json = serde_json::to_value(&report.rejections).unwrap_or_else(|_| {
            serde_json::Value::Array(Vec::new())
        });
        Ok(ActionOutcome::Completed {
            bundle_path: bundle,
            result_json: serde_json::json!({
                "outcome": "completed",
                "executor": "document_csv",
                "output_path": report.path.display().to_string(),
                "bytes_written": report.bytes_written,
                "rows_written": report.rows_written,
                "rows_rejected": report.rows_rejected,
                "rejections": rejections_json,
                "sha256": report.sha256,
            }),
        })
    }
}

/// Manifest-side payload shape; deserialises directly into the
/// crate's `CsvWriteRequest` via per-field unpack so we can tolerate
/// the `output_root` default + provide a clear error message on
/// missing required fields.
#[derive(Debug, Deserialize)]
struct DocumentCsvPayload {
    #[serde(default)]
    output_root: Option<PathBuf>,
    path: PathBuf,
    schema: Vec<CsvColumnSchema>,
    #[serde(default)]
    rows: Vec<HashMap<String, serde_json::Value>>,
    #[serde(default = "default_include_header")]
    include_header: bool,
    #[serde(default)]
    line_terminator: LineTerminator,
    #[serde(default = "default_quote_minimal")]
    quote_minimal: bool,
    #[serde(default)]
    write_bom: bool,
}

fn default_include_header() -> bool {
    true
}

fn default_quote_minimal() -> bool {
    true
}

/// Parse `action.payload` into a [`CsvWriteRequest`]. The
/// `output_root` field is optional in the manifest; absence falls
/// back to [`DEFAULT_OUTPUT_ROOT`] so a vertical that doesn't carry
/// the operator's per-host config still works against the shipped
/// default.
pub(crate) fn parse_request(payload: &serde_json::Value) -> ArgoResult<CsvWriteRequest> {
    let parsed: DocumentCsvPayload = serde_json::from_value(payload.clone()).map_err(|e| {
        ArgoError::Schema(format!(
            "document-csv payload: {e} (need {{ path, schema, rows, ... }})"
        ))
    })?;
    let output_root = parsed
        .output_root
        .unwrap_or_else(|| PathBuf::from(DEFAULT_OUTPUT_ROOT));
    Ok(CsvWriteRequest {
        output_root,
        path: parsed.path,
        schema: parsed.schema,
        rows: parsed.rows,
        include_header: parsed.include_header,
        line_terminator: parsed.line_terminator,
        quote_minimal: parsed.quote_minimal,
        write_bom: parsed.write_bom,
    })
}

/// Build the per-action bundle dir on disk. Mirrors
/// `worker_actions_executor_document::ensure_bundle_dir` so the
/// bundle path is shaped consistently across executors.
fn ensure_bundle_dir(cwd: &Path, job_id: &str, action_id: &str) -> ArgoResult<PathBuf> {
    let dir = cwd
        .join(".skycot")
        .join("outputs")
        .join(job_id)
        .join("actions")
        .join(action_id);
    std::fs::create_dir_all(&dir).map_err(|e| {
        ArgoError::Provider(format!(
            "document-csv: create bundle dir {}: {e}",
            dir.display()
        ))
    })?;
    Ok(dir)
}

#[cfg(test)]
#[path = "worker_actions_executor_document_csv_tests.rs"]
mod tests;
