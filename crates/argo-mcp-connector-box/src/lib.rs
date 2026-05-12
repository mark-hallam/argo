//! Box.com MCP connector.
//!
//! Typed REST client for Box's document-storage surface — list, upload,
//! download, shared-link generation, and search. Pairs with the
//! [`argo-mcp-connector-dropbox`] crate (and the pending OneDrive sibling) to
//! cover the gtm-engineer and executive-assistant verticals' document-storage
//! needs without binding them to Dropbox specifically.
//!
//! ```no_run
//! use argo_mcp_connector_box::{BoxConnector, SharedLinkAccess};
//!
//! # async fn doc() -> argo_types::ArgoResult<()> {
//! let bx = BoxConnector::new("oauth2-access-token".into());
//! let items = bx.list_folder("0").await?; // "0" = user's root folder
//! let uploaded = bx
//!     .upload_file("0", "receipt.txt", b"hello")
//!     .await?;
//! let bytes = bx.download_file(&uploaded.id).await?;
//! let url = bx
//!     .create_shared_link(&uploaded.id, SharedLinkAccess::Company)
//!     .await?;
//! let _ = (items, bytes, url);
//! # Ok(()) }
//! ```
//!
//! ## Wire-protocol shape
//!
//! Box's REST surface uses two host bases: `api.box.com/2.0` for metadata
//! operations (JSON in, JSON out — `GET /folders/{id}/items`, `GET /search`,
//! `PUT /files/{id}` for shared links) and `upload.box.com/api/2.0` for the
//! multipart upload endpoint (`POST /files/content`). The host is a function
//! of the operation, not a per-connector setting.
//!
//! Folder ids — including the implicit root, `"0"` — and file ids are opaque
//! numeric strings. The connector validates that contract before issuing any
//! network call and rejects malformed ids with [`argo_types::ArgoError::Schema`].
//!
//! ## Auth
//!
//! Auth is `Authorization: Bearer <oauth_token>`. Box returns HTTP 401 for
//! expired or revoked tokens; the connector surfaces this as
//! [`argo_types::ArgoError::Provider`] with a refresh hint string so the
//! caller's OAuth wrapper can intercept and refresh. Token refresh itself is
//! the operator wrapper's responsibility — this connector takes a valid token
//! and does not invoke Box's `/oauth2/token` endpoint.
//!
//! ## Shared-link access levels
//!
//! Box's `shared_link.access` field has three valid values: `open`,
//! `company`, and `collaborators`. The connector exposes them verbatim
//! through [`SharedLinkAccess`] — DO NOT normalize these into a generic
//! `public/restricted` pair, because each maps to a distinct enterprise
//! policy outcome that compliance reviewers need to audit on a per-link
//! basis.
//!
//! [`argo-mcp-connector-dropbox`]: https://docs.rs/argo-mcp-connector-dropbox

#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

mod client;
mod types;

pub use client::BoxConnector;
pub use types::{BoxFile, BoxFolder, BoxItem, SharedLinkAccess};
