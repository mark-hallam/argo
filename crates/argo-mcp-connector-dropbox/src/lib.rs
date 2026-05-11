//! Dropbox MCP connector.
//!
//! Typed REST client for Dropbox's file-storage surface — list, upload,
//! download, delete, shared-link generation, and search. Pairs with the
//! bookkeeping-assistant and data-entry-clerk verticals which need a place to
//! read source documents and write outputs that is not tied to Microsoft
//! Graph (already covered by the document-executor's MS Graph path).
//!
//! ```no_run
//! use argo_mcp_connector_dropbox::{DbxWriteMode, DropboxConnector};
//!
//! # async fn doc() -> argo_types::ArgoResult<()> {
//! let dbx = DropboxConnector::new("oauth2-access-token".into());
//! let entries = dbx.list_folder("/inbox", false).await?;
//! let uploaded = dbx
//!     .upload("/inbox/receipt.txt", b"hello", DbxWriteMode::Overwrite)
//!     .await?;
//! let _bytes = dbx.download(&uploaded.path_display).await?;
//! # Ok(()) }
//! ```
//!
//! ## Wire-protocol quirks
//!
//! Dropbox v2 is RPC-over-HTTP-POST — every endpoint is `POST`, even for
//! "reads" like `list_folder` and `search_v2`. Two host bases are used:
//! `api.dropboxapi.com` for metadata operations (JSON in, JSON out) and
//! `content.dropboxapi.com` for content operations (`upload`, `download`)
//! which carry the request metadata in the `Dropbox-API-Arg` header and the
//! payload (or response body) as raw bytes. The host is a function of the
//! operation, not a per-connector setting.
//!
//! Paths are always absolute and start with `/`; the empty string `""`
//! addresses the user's root. The connector validates that contract before
//! issuing any network call and rejects malformed paths with
//! [`argo_types::ArgoError::Schema`].
//!
//! ## Auth
//!
//! The connector accepts a long-lived OAuth2 access token. Refresh-via-
//! `refresh_token` is intentionally out of scope for this brief; the
//! follow-up `mission-mcp-connector-dropbox-refresh` will layer that on top.
//!
//! ## Upload size cap
//!
//! `/files/upload` accepts a single-request body up to 150 MiB. Larger
//! payloads must be sent through `/files/upload_session/*` chunked endpoints,
//! which are NOT implemented in v1. The connector rejects oversized uploads
//! with [`argo_types::ArgoError::Provider`] carrying a hint that the caller
//! should fall back to the document-executor's MS Graph `UploadOnly` path
//! until chunked upload lands.

#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

mod client;
mod types;

pub use client::{DropboxConnector, MAX_UPLOAD_BYTES};
pub use types::{DbxEntry, DbxFile, DbxFolder, DbxSharedLink, DbxWriteMode};
