//! OneDrive MCP connector.
//!
//! Typed REST client for Microsoft Graph's drive surface — list, upload
//! (simple + resumable), download, share-link creation, and search. Pairs
//! with the Microsoft-365-tenant verticals which need a place to read source
//! documents and write outputs that lives outside the document-executor's
//! direct MS Graph path.
//!
//! ```no_run
//! use argo_mcp_connector_onedrive::{LinkScope, LinkType, OneDriveConnector};
//!
//! # async fn doc() -> argo_types::ArgoResult<()> {
//! let od = OneDriveConnector::new("oauth2-access-token".into());
//! let entries = od.list_folder(None).await?; // /me/drive/root
//! let uploaded = od.upload_small_file("root", "hello.txt", b"hello").await?;
//! let link = od.create_share_link("root", LinkType::View, LinkScope::Anonymous).await?;
//! let _ = link;
//! let _entries = entries;
//! let _uploaded = uploaded;
//! # Ok(()) }
//! ```
//!
//! ## Auth scopes
//!
//! The connector calls Graph endpoints that require, at minimum,
//! `Files.ReadWrite` for the user's own drive. Shared-drive operations
//! (composite ids of the form `"<driveId>!<itemId>"`) additionally require
//! `Files.ReadWrite.All`. Operators must request both scopes in their OAuth
//! app registration if they want the connector to span personal +
//! shared drives.
//!
//! ## Upload size split
//!
//! Microsoft Graph caps the simple `PUT /content` upload at 4 MiB; larger
//! payloads must go through `createUploadSession` + chunked PUTs. The
//! connector exposes the two paths as separate methods —
//! [`OneDriveConnector::upload_small_file`] and
//! [`OneDriveConnector::upload_large_file`] — and rejects calls where the
//! payload size doesn't match the chosen method, so a caller can't
//! accidentally upload a 1 KiB file through the resumable code path.
//!
//! ## Out of scope
//!
//! SharePoint (different drive resource), Microsoft Lists, Outlook
//! attachment scraping, admin/tenant APIs, and token refresh. The
//! `mission-mcp-connector-onedrive-sharepoint` and
//! `-onedrive-refresh` follow-ups will layer those on top.

#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

mod client;
mod types;

pub use client::{OneDriveConnector, DEFAULT_CHUNK_BYTES, SIMPLE_UPLOAD_CAP};
pub use types::{DriveFile, DriveFolder, DriveItem, LinkScope, LinkType};
