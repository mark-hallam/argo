//! Per-executor config sections: `[document_csv]`. Extracted from
//! [`crate::sections`] to keep that module under the 500-LOC ceiling.
//! Re-exported through `sections` so the public API is unchanged.
use crate::defaults;
use serde::{Deserialize, Serialize};

/// `[document_csv]` — operator-side knobs for the
/// `argo-executor-document-csv` ExecutorKind. Today only
/// `output_root`; growth-room left for per-tenant overrides + size
/// caps when the in-memory-row cap (≤1M v1) needs to lift.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DocumentCsvConfig {
    /// Filesystem root every CSV write lands beneath. Path-traversal
    /// attempts (`..`, absolute paths outside, symlinks targeting
    /// outside) are refused with a typed sandbox-violation error.
    /// Default [`crate::defaults::DEFAULT_DOCUMENT_CSV_OUTPUT_ROOT`].
    pub output_root: String,
}

impl Default for DocumentCsvConfig {
    fn default() -> Self {
        Self {
            output_root: defaults::DEFAULT_DOCUMENT_CSV_OUTPUT_ROOT.into(),
        }
    }
}
