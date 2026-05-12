//! Wire and public DTOs for the Box connector.
//!
//! Box's REST surface tags polymorphic entries with a string `type` field
//! (`"file"` or `"folder"`). The wire types below mirror that shape; the
//! public [`BoxItem`] / [`BoxFile`] / [`BoxFolder`] types are the ergonomic
//! Rust enum the connector exposes to callers.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A single entry returned by `list_folder` or `search`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoxItem {
    /// A regular file.
    File(BoxFile),
    /// A folder.
    Folder(BoxFolder),
}

/// File metadata returned by upload, `list_folder`, and `search`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoxFile {
    /// Stable opaque numeric id (rendered as a string by Box).
    pub id: String,
    /// Leaf file name.
    pub name: String,
    /// File size in bytes.
    pub size_bytes: u64,
    /// Server-side last-modified timestamp.
    pub modified_at: DateTime<Utc>,
    /// Box's content etag (string-encoded integer, monotonically increasing
    /// per revision of the file).
    pub etag: String,
}

/// Folder metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoxFolder {
    /// Stable opaque numeric id (rendered as a string by Box).
    pub id: String,
    /// Leaf folder name.
    pub name: String,
    /// Server-side last-modified timestamp (folders have one too).
    pub modified_at: DateTime<Utc>,
}

/// Visibility levels supported by Box's `shared_link.access` field.
///
/// Box's terminology is preserved verbatim — the connector does NOT normalize
/// these into a generic "public/restricted" pair, because each level maps to
/// a distinct enterprise-policy outcome:
///
/// - `Open`: anyone with the URL can access (subject to global enterprise policy).
/// - `Company`: only authenticated users in the file owner's enterprise.
/// - `Collaborators`: only explicit file collaborators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SharedLinkAccess {
    /// Public — anyone with the URL.
    Open,
    /// Enterprise-internal only.
    Company,
    /// Restricted to explicit collaborators.
    Collaborators,
}

impl SharedLinkAccess {
    /// Box's wire token for this access level.
    pub(crate) fn as_wire(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Company => "company",
            Self::Collaborators => "collaborators",
        }
    }
}

// ---------- wire shapes ----------

#[derive(Debug, Deserialize)]
pub(crate) struct ItemsResp {
    pub entries: Vec<EntryWire>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
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
    /// Box returns `size` for files. Optional because `list_folder` callers
    /// that select narrower field sets may not get it; the connector requests
    /// the full set so it should always be present in our paths.
    pub size: u64,
    pub modified_at: DateTime<Utc>,
    pub etag: String,
}

impl From<FileWire> for BoxFile {
    fn from(w: FileWire) -> Self {
        Self {
            id: w.id,
            name: w.name,
            size_bytes: w.size,
            modified_at: w.modified_at,
            etag: w.etag,
        }
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct FolderWire {
    pub id: String,
    pub name: String,
    pub modified_at: DateTime<Utc>,
}

impl From<FolderWire> for BoxFolder {
    fn from(w: FolderWire) -> Self {
        Self {
            id: w.id,
            name: w.name,
            modified_at: w.modified_at,
        }
    }
}

impl EntryWire {
    pub(crate) fn into_public(self) -> Option<BoxItem> {
        match self {
            Self::File(f) => Some(BoxItem::File(f.into())),
            Self::Folder(f) => Some(BoxItem::Folder(f.into())),
            Self::Other => None,
        }
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct SharedLinkUpdateResp {
    pub shared_link: SharedLinkWire,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SharedLinkWire {
    pub url: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct SharedLinkUpdateBody {
    pub shared_link: SharedLinkSettings,
}

#[derive(Debug, Serialize)]
pub(crate) struct SharedLinkSettings {
    pub access: &'static str,
}

/// `attributes` JSON part of the upload multipart body.
#[derive(Debug, Serialize)]
pub(crate) struct UploadAttributes<'a> {
    pub name: &'a str,
    pub parent: UploadParent<'a>,
}

#[derive(Debug, Serialize)]
pub(crate) struct UploadParent<'a> {
    pub id: &'a str,
}
