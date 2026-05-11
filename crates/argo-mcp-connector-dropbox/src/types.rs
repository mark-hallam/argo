//! Wire and public DTOs for the Dropbox connector.
//!
//! The Dropbox v2 surface tags polymorphic entries with `.tag` discriminators
//! (`"file"`, `"folder"`, `"deleted"`). The wire types below mirror that
//! shape; the public [`DbxEntry`] / [`DbxFile`] / [`DbxFolder`] types are the
//! ergonomic Rust enum the connector exposes to callers.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A single entry returned by `list_folder` or `search`.
///
/// `Deleted` tombstones from `list_folder/continue` cursors are intentionally
/// dropped during conversion — v1 only surfaces live entries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DbxEntry {
    /// A regular file.
    File(DbxFile),
    /// A folder.
    Folder(DbxFolder),
}

/// File metadata returned by upload, download (via `Dropbox-API-Result`),
/// `list_folder`, and `search`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DbxFile {
    /// Stable opaque id (`"id:..."`). Survives renames.
    pub id: String,
    /// Leaf file name.
    pub name: String,
    /// Absolute display path (`/folder/file.ext`).
    pub path_display: String,
    /// File size in bytes.
    pub size_bytes: u64,
    /// Dropbox content hash (4 MiB-block SHA-256 tree-hash, hex).
    pub content_hash: String,
    /// Server-side last-modified timestamp.
    pub server_modified: DateTime<Utc>,
}

/// Folder metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DbxFolder {
    /// Stable opaque id.
    pub id: String,
    /// Leaf folder name.
    pub name: String,
    /// Absolute display path.
    pub path_display: String,
}

/// Write disposition for [`DropboxConnector::upload`].
///
/// Serializes into the `mode` field of the Dropbox upload arg. `Add` is the
/// non-overwriting default (fails if the path exists). `Overwrite` replaces
/// any existing file. `Update { rev }` is a CAS-style update that only
/// succeeds if the current `rev` matches.
///
/// [`DropboxConnector::upload`]: crate::DropboxConnector::upload
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DbxWriteMode {
    /// Fail if a file already exists at the path.
    Add,
    /// Replace any existing file at the path.
    Overwrite,
    /// Compare-and-swap update — only succeeds if `rev` matches the
    /// server's current revision for the file.
    Update {
        /// The expected current revision.
        rev: String,
    },
}

impl DbxWriteMode {
    pub(crate) fn to_json(&self) -> serde_json::Value {
        match self {
            Self::Add => serde_json::Value::String("add".into()),
            Self::Overwrite => serde_json::Value::String("overwrite".into()),
            Self::Update { rev } => serde_json::json!({ ".tag": "update", "update": rev }),
        }
    }
}

/// A shared link as returned by `create_shared_link_with_settings`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DbxSharedLink {
    /// The public URL.
    pub url: String,
    /// Lower-cased absolute path of the target.
    pub path: String,
    /// Optional explicit expiry; absent for non-expiring links.
    pub expires: Option<DateTime<Utc>>,
}

// ---------- wire shapes ----------

#[derive(Debug, Deserialize)]
pub(crate) struct ListFolderResp {
    pub entries: Vec<EntryWire>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = ".tag", rename_all = "lowercase")]
pub(crate) enum EntryWire {
    File(FileWire),
    Folder(FolderWire),
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize)]
pub(crate) struct FileWire {
    pub id: String,
    pub name: String,
    pub path_display: String,
    pub size: u64,
    pub content_hash: String,
    pub server_modified: DateTime<Utc>,
}

impl From<FileWire> for DbxFile {
    fn from(w: FileWire) -> Self {
        Self {
            id: w.id,
            name: w.name,
            path_display: w.path_display,
            size_bytes: w.size,
            content_hash: w.content_hash,
            server_modified: w.server_modified,
        }
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct FolderWire {
    pub id: String,
    pub name: String,
    pub path_display: String,
}

impl From<FolderWire> for DbxFolder {
    fn from(w: FolderWire) -> Self {
        Self {
            id: w.id,
            name: w.name,
            path_display: w.path_display,
        }
    }
}

impl EntryWire {
    pub(crate) fn into_public(self) -> Option<DbxEntry> {
        match self {
            Self::File(f) => Some(DbxEntry::File(f.into())),
            Self::Folder(f) => Some(DbxEntry::Folder(f.into())),
            Self::Other => None,
        }
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct SearchResp {
    pub matches: Vec<SearchMatch>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SearchMatch {
    pub metadata: SearchMetadata,
}

#[derive(Debug, Deserialize)]
#[serde(tag = ".tag", rename_all = "lowercase")]
pub(crate) enum SearchMetadata {
    Metadata {
        /// Nested actual entry. The double-wrap is a Dropbox quirk.
        metadata: EntryWire,
    },
    #[serde(other)]
    Other,
}

impl SearchMetadata {
    pub(crate) fn into_entry(self) -> Option<DbxEntry> {
        match self {
            Self::Metadata { metadata } => metadata.into_public(),
            Self::Other => None,
        }
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct SharedLinkResp {
    pub url: String,
    pub path_lower: String,
    pub expires: Option<DateTime<Utc>>,
}

impl From<SharedLinkResp> for DbxSharedLink {
    fn from(w: SharedLinkResp) -> Self {
        Self {
            url: w.url,
            path: w.path_lower,
            expires: w.expires,
        }
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct PathArg<'a> {
    pub path: &'a str,
}

#[derive(Debug, Serialize)]
pub(crate) struct ListFolderArg<'a> {
    pub path: &'a str,
    pub recursive: bool,
}

#[derive(Debug, Serialize)]
pub(crate) struct SearchOpts {
    pub max_results: u32,
}

#[derive(Debug, Serialize)]
pub(crate) struct SearchArg<'a> {
    pub query: &'a str,
    pub options: SearchOpts,
}
