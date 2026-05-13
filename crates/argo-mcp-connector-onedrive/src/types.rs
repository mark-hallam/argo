//! Wire and public DTOs for the OneDrive connector.
//!
//! Microsoft Graph exposes drive items through a single `driveItem` resource
//! that carries a `file` *or* `folder` facet (occasionally both — Graph also
//! has `package`, `image`, `audio`, etc. facets but those don't change the
//! file-vs-folder distinction). The public [`DriveItem`] enum collapses that
//! to two variants and drops everything Graph emits that the connector does
//! not use.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A single Graph `driveItem` projected to the connector's public surface.
///
/// `Other` is intentionally absent — items missing both `file` and `folder`
/// facets are dropped during conversion. v1 surfaces only the two shapes the
/// brief requires.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DriveItem {
    /// A regular file.
    File(DriveFile),
    /// A folder.
    Folder(DriveFolder),
}

/// File-shaped drive item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriveFile {
    /// Stable Graph id. For a shared-drive item this carries Graph's native
    /// `driveId!itemId` composite — the connector accepts that form verbatim
    /// (see [`OneDriveConnector::encode_item_id`]).
    ///
    /// [`OneDriveConnector::encode_item_id`]: crate::OneDriveConnector::encode_item_id
    pub id: String,
    /// Leaf file name.
    pub name: String,
    /// File size in bytes.
    pub size_bytes: u64,
    /// MIME type per Graph's `file.mimeType` field; absent on some legacy
    /// items so the wire decode tolerates omission.
    pub mime_type: Option<String>,
    /// Server-side last-modified timestamp.
    pub last_modified: DateTime<Utc>,
    /// Web-app URL (`webUrl`) — useful for share dialogs even though the
    /// connector also exposes [`OneDriveConnector::create_share_link`].
    ///
    /// [`OneDriveConnector::create_share_link`]: crate::OneDriveConnector::create_share_link
    pub web_url: Option<String>,
}

/// Folder-shaped drive item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriveFolder {
    /// Stable Graph id (see [`DriveFile::id`] for shared-drive notes).
    pub id: String,
    /// Leaf folder name.
    pub name: String,
    /// Child count Graph publishes on the folder facet. May be 0 on freshly
    /// created folders before Graph indexes children.
    pub child_count: u32,
    /// Web-app URL.
    pub web_url: Option<String>,
}

/// Type of share link requested via `createLink`.
///
/// Maps directly onto Graph's `Permission.Link.type` enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkType {
    /// Read-only access.
    View,
    /// Read-write access.
    Edit,
    /// Embeddable read-only access (returns an `<iframe>` URL).
    Embed,
}

impl LinkType {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::View => "view",
            Self::Edit => "edit",
            Self::Embed => "embed",
        }
    }
}

/// Audience scope for a share link.
///
/// `Users` is a directory-targeted link that Graph also accepts under the
/// `createLink` action, even though Graph's docs primarily describe it as a
/// follow-up `invite` parameter. The connector serializes the value
/// verbatim; OneDrive Business tenants honor it, consumer accounts may
/// downgrade to `Organization`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkScope {
    /// Anyone with the link (anonymous, no sign-in).
    Anonymous,
    /// Anyone in the tenant.
    Organization,
    /// Specific named users (directory-scoped).
    Users,
}

impl LinkScope {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Anonymous => "anonymous",
            Self::Organization => "organization",
            Self::Users => "users",
        }
    }
}

// ---------- wire shapes ----------

#[derive(Debug, Deserialize)]
pub(crate) struct ChildrenResp {
    pub value: Vec<DriveItemWire>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SearchResp {
    pub value: Vec<DriveItemWire>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DriveItemWire {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub size: u64,
    #[serde(rename = "lastModifiedDateTime")]
    pub last_modified: Option<DateTime<Utc>>,
    #[serde(rename = "webUrl")]
    pub web_url: Option<String>,
    pub file: Option<FileFacet>,
    pub folder: Option<FolderFacet>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct FileFacet {
    #[serde(rename = "mimeType")]
    pub mime_type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct FolderFacet {
    #[serde(rename = "childCount", default)]
    pub child_count: u32,
}

impl DriveItemWire {
    pub(crate) fn into_public(self) -> Option<DriveItem> {
        let last_modified = self.last_modified.unwrap_or_else(Utc::now);
        if let Some(folder) = self.folder {
            return Some(DriveItem::Folder(DriveFolder {
                id: self.id,
                name: self.name,
                child_count: folder.child_count,
                web_url: self.web_url,
            }));
        }
        if let Some(file) = self.file {
            return Some(DriveItem::File(DriveFile {
                id: self.id,
                name: self.name,
                size_bytes: self.size,
                mime_type: file.mime_type,
                last_modified,
                web_url: self.web_url,
            }));
        }
        None
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct UploadSessionResp {
    #[serde(rename = "uploadUrl")]
    pub upload_url: String,
}

/// Body of an in-progress resumable-upload `nextExpectedRanges` poll or
/// chunk-PUT 202 reply. Ranges are `"start-end"` byte-inclusive strings.
#[derive(Debug, Deserialize)]
pub(crate) struct UploadStatusResp {
    #[serde(rename = "nextExpectedRanges", default)]
    pub next_expected_ranges: Vec<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct CreateLinkArg {
    #[serde(rename = "type")]
    pub link_type: &'static str,
    pub scope: &'static str,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CreateLinkResp {
    pub link: ShareLinkWire,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ShareLinkWire {
    #[serde(rename = "webUrl")]
    pub web_url: String,
}
