//! V1.2 Document-creation ExecutorKind — synthesise text artefacts
//! (markdown, CSV, PDF, HTML slides) under a per-worker output
//! root.
//!
//! Mission says "any task a human can do on a PC/Mac"; producing a
//! deliverable doc is one of the most common. This crate carries
//! the four primitive backends + the `DocExecutor` façade
//! `argo-cli`'s dispatch shim invokes.
//!
//! Each worker writes only beneath
//! `<worker.fs_root>/outputs/<job_id>/<action_id>/`. The path
//! resolver (`OutputPath::resolve`) refuses absolute targets +
//! parent traversal so a poisoned manifest can't escape the
//! sandbox into `/etc/passwd`.
//!
//! Approval-gating is the dispatch-shim's job (the
//! `AutoApprovalPolicy` consults the action's manifest
//! classification). DocumentCreation is approval-gated by default;
//! workers without explicit allowlist entries see operator
//! sign-off before each render.
//!
//! Quick start:
//! ```no_run
//! use argo_executor_document::{DocExecutor, DocRequest};
//! let exec = DocExecutor::new();
//! let req = DocRequest::Markdown {
//!     output_path: "report.md".into(),
//!     body: "# Hello\n\nworld".into(),
//! };
//! let bundle = std::path::PathBuf::from("/tmp/argo-fs/worker-1/outputs/job-1/act-1");
//! std::fs::create_dir_all(&bundle).unwrap();
//! let outcome = exec.run(&req, &bundle).unwrap();
//! assert_eq!(outcome.kind, "markdown");
//! ```

pub mod csv;
pub mod error;
pub mod executor;
pub mod markdown;
pub mod msgraph;
pub mod pdf;
pub mod slides;

pub use error::{DocExecutorError, DocResult};
pub use executor::{
    CsvRow, DocExecutor, DocOutcome, DocRequest, OutputPath, DOC_KINDS, ENV_CHROME_PATH,
};
pub use msgraph::{MsGraphExecutor, MsGraphFile, MsGraphFormat};
