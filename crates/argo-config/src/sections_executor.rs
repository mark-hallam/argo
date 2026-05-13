//! Per-executor config sections: `[document_csv]`, `[document_gsuite]`.
//! Extracted from [`crate::sections`] to keep that module under the
//! 500-LOC ceiling. Re-exported through `sections` so the public API
//! is unchanged.
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

/// `[document_gsuite]` — operator-side knobs for the
/// `argo-executor-document-gsuite` ExecutorKind. Service-account
/// credentials themselves load env-indirect (the
/// `ARGO_GSUITE_SERVICE_ACCOUNT_*` vars); this section gates which
/// doc kinds the executor will produce and the default Drive folder
/// drafts land in.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DocumentGsuiteConfig {
    /// Drive folder id every draft lands under. Empty ⇒ the
    /// service-account's "My Drive" root. Operators wanting
    /// per-tenant isolation create folders out-of-band and share with
    /// the service-account; folder management itself is out-of-scope.
    pub default_drive_folder_id: String,
    /// Doc kinds the executor is allowed to produce. Default
    /// [`crate::defaults::DEFAULT_GSUITE_DOC_KINDS`] (all three).
    /// Operators may restrict (e.g. `["Doc"]`) for verticals that
    /// should only emit text drafts.
    pub permitted_doc_kinds: Vec<String>,
    /// Soft cap on the rendered request size (rows × cells, slides ×
    /// elements) in KB. Defaults to
    /// [`crate::defaults::DEFAULT_GSUITE_MAX_DOC_SIZE_KB`]. Enforced
    /// at the executor's preflight; oversize requests reject before
    /// the Drive `files.create` round-trip.
    pub max_doc_size_kb: u32,
}

impl Default for DocumentGsuiteConfig {
    fn default() -> Self {
        Self {
            default_drive_folder_id: String::new(),
            permitted_doc_kinds: defaults::DEFAULT_GSUITE_DOC_KINDS
                .iter()
                .map(|s| (*s).to_string())
                .collect(),
            max_doc_size_kb: defaults::DEFAULT_GSUITE_MAX_DOC_SIZE_KB,
        }
    }
}
