//! Error type for the document-creation executor.
//!
//! Every variant carries enough context for the audit ledger to
//! surface a useful failure reason without swallowing the cause.

use std::path::PathBuf;
use thiserror::Error;

pub type DocResult<T> = Result<T, DocExecutorError>;

#[derive(Debug, Error)]
pub enum DocExecutorError {
    /// The action payload referenced a path outside the per-action
    /// bundle directory (absolute path, parent traversal, or empty
    /// component).
    #[error("output path '{path}' escapes the per-action bundle ({reason})")]
    PathEscapesSandbox { path: String, reason: String },

    /// Filesystem op (mkdir / write / read-back) failed.
    #[error("io: {context}: {source}")]
    Io {
        context: String,
        #[source]
        source: std::io::Error,
    },

    /// The PDF backend needs headless Chrome (or another HTML→PDF
    /// renderer); none was found at the configured path.
    #[error(
        "pdf backend: chrome not found at '{path}' — set {env} to override, \
         or install Google Chrome ({hint})"
    )]
    ChromeMissing {
        path: PathBuf,
        env: &'static str,
        hint: &'static str,
    },

    /// Headless Chrome ran but exited non-zero. Stderr captured for
    /// the operator.
    #[error("pdf backend: chrome exited {code} (stderr: {stderr})")]
    ChromeFailed { code: i32, stderr: String },

    /// CSV writer rejected a row whose column count differs from
    /// the header. Catching this in the writer (vs serialising bad
    /// CSV) keeps downstream consumers honest.
    #[error("csv: row {row_index} has {got} cols, header has {expected}")]
    CsvShapeMismatch {
        row_index: usize,
        expected: usize,
        got: usize,
    },
}

impl DocExecutorError {
    pub fn io(context: impl Into<String>, source: std::io::Error) -> Self {
        Self::Io {
            context: context.into(),
            source,
        }
    }
}
