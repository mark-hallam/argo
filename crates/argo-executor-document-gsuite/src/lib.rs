//! Google Workspace (Docs / Sheets / Slides) document-creation
//! executor. Drafts-only by design: every action emits a Google-native
//! draft document inside the service-account's Drive (optionally rooted
//! at an operator-configured folder). Sharing / publishing is left to
//! the operator; the executor never grants ACLs beyond what the
//! service-account already has.
//!
//! Mirrors the canonical crate layout of
//! [`argo-executor-document-csv`]: stateless `write` entry point that
//! returns a sha256-stamped report the worker-action bundle surfaces to
//! the audit ledger. The only delta is the API layer — Google Workspace
//! HTTPS instead of local disk.
//!
//! Auth: service-account OAuth2 ("assertion grant") flow. The signed
//! JWT is exchanged for a short-lived bearer token cached per-process;
//! the cache refreshes 5 min before stated expiry and force-refreshes
//! on any 401 from the API surface. The credential
//! (`private_key_pem`) is loaded env-indirect via
//! [`argo_config::cred`] and never leaves the process.
//!
//! ## Quick start
//!
//! ```ignore
//! use argo_executor_document_gsuite::{
//!     DocumentGsuite, GsuiteAuthConfig, GsuiteDocKind, GsuiteWriteRequest,
//! };
//!
//! # async fn doc() {
//! let auth = GsuiteAuthConfig::from_env().unwrap();
//! let executor = DocumentGsuite::new(auth);
//! let report = executor
//!     .write(GsuiteWriteRequest {
//!         kind: GsuiteDocKind::Doc,
//!         title: "Draft brief".into(),
//!         drive_folder_id: None,
//!         doc_body_text: Some("Hello world.".into()),
//!         sheet_schema: None,
//!         sheet_rows: vec![],
//!         slides: vec![],
//!     })
//!     .await
//!     .unwrap();
//! assert!(!report.doc_id.0.is_empty());
//! # }
//! ```

#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

pub mod auth;
pub mod document_gsuite;
mod document_gsuite_payload;
pub mod types;

pub use auth::{GsuiteAuth, GsuiteAuthConfig};
pub use document_gsuite::{DocumentGsuite, GsuiteWriteReport};
pub use types::{
    GsuiteDocId, GsuiteDocKind, GsuiteRejection, GsuiteSandboxError, GsuiteSheetRow,
    GsuiteSlideSpec, GsuiteWriteRequest, PERMITTED_DOC_KINDS_DEFAULT,
};

#[cfg(test)]
#[path = "document_gsuite_tests.rs"]
mod document_gsuite_tests;
