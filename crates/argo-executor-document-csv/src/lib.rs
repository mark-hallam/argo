//! V1.2 Document-CSV ExecutorKind — typed, RFC 4180-conforming CSV
//! writer for vertical agents that emit structured tabular output
//! (data-entry-clerk, bookkeeping-assistant, procurement-assistant).
//!
//! Distinct from `argo-executor-document::csv` (which is a thin
//! string-array primitive): this crate adds a typed `CsvColumnSchema`
//! with six column kinds, record-level rejection (one bad row does
//! not abort the file), and a sha256-stamped output report that the
//! worker-action bundle surfaces back to the operator for downstream
//! verify chains.
//!
//! ## Spec
//!
//! - RFC 4180 conformance: fields containing `,`, `"`, `CR`, or `LF`
//!   are quoted; embedded `"` is doubled (`""`); default `CRLF` line
//!   terminator.
//! - `quote_minimal = false` quote-wraps every field (including
//!   header) — useful for downstream tools that mis-handle minimal
//!   quoting.
//! - UTF-8 BOM is opt-in via the request (default off). When enabled
//!   the BOM bytes are included in the sha256.
//! - Six column kinds with a coercion table: Text, Integer, Decimal,
//!   Date (RFC 3339 → YYYY-MM-DD), Boolean, EmailAddress (shape
//!   check only).
//! - Sandbox: `path` is resolved beneath the operator-configured
//!   `[document_csv].output_root` (default `~/.argo/exports/csv/`).
//!   Path-traversal returns a typed sandbox-violation error.
//!
//! ## In-memory cap (v1)
//!
//! Writes ≤ 1 M rows in v1; the entire row vector lives in memory
//! before serialise. Streaming write is a follow-up brief once a
//! vertical actually needs ≥ 1 M rows.
//!
//! ## Quick start
//!
//! ```no_run
//! use argo_executor_document_csv::{
//!     CsvColumnKind, CsvColumnSchema, CsvWriteRequest, DocumentCsv,
//!     LineTerminator,
//! };
//! use std::path::PathBuf;
//!
//! let req = CsvWriteRequest {
//!     output_root: PathBuf::from("/tmp/argo-exports-csv"),
//!     path: PathBuf::from("contacts.csv"),
//!     schema: vec![
//!         CsvColumnSchema { header: "name".into(), kind: CsvColumnKind::Text,
//!             required: true, max_len: Some(64) },
//!         CsvColumnSchema { header: "email".into(), kind: CsvColumnKind::EmailAddress,
//!             required: true, max_len: None },
//!     ],
//!     rows: vec![],
//!     include_header: true,
//!     line_terminator: LineTerminator::Crlf,
//!     quote_minimal: true,
//!     write_bom: false,
//! };
//! let report = DocumentCsv::new().write(req).unwrap();
//! assert_eq!(report.rows_written, 0);
//! ```

#![warn(missing_docs)]

pub mod document_csv;
pub mod render;
pub mod sandbox;
pub mod types;

pub use document_csv::{CsvWriteRequest, CsvWriteReport, DocumentCsv};
pub use render::render_rfc4180;
pub use sandbox::{resolve_sandboxed_path, CsvSandboxError, DEFAULT_OUTPUT_ROOT};
pub use types::{CsvColumnKind, CsvColumnSchema, LineTerminator, RejectionReason, RowRejection};

#[cfg(test)]
#[path = "document_csv_tests.rs"]
mod document_csv_tests;
