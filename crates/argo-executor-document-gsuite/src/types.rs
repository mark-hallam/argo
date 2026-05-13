//! Typed request / report / error shapes for the
//! [`crate::DocumentGsuite`] executor.

use argo_executor_document_csv::CsvColumnSchema;
use argo_types::ArgoError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Three Google Workspace document kinds the executor knows how to
/// create. New kinds (Forms, Sites, Apps Script) would land via
/// follow-up briefs; the variants below mirror exactly the surfaces
/// covered by the mission-load-bearing-wire scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum GsuiteDocKind {
    /// Google Docs (`application/vnd.google-apps.document`). Created
    /// via Drive `files.create` then `documents.batchUpdate` to insert
    /// the body text.
    Doc,
    /// Google Sheets (`application/vnd.google-apps.spreadsheet`).
    /// Created via Drive `files.create` then
    /// `spreadsheets.batchUpdate` with `appendCells` for typed rows.
    Sheet,
    /// Google Slides (`application/vnd.google-apps.presentation`).
    /// Created via Drive `files.create` then
    /// `presentations.batchUpdate` with `createSlide` / `insertText` /
    /// `createImage` requests.
    Slides,
}

impl GsuiteDocKind {
    /// MIME type emitted to Drive `files.create`. Driving Drive to
    /// pick the right native MIME is what makes the resulting file a
    /// Google-native doc rather than an uploaded blob.
    #[must_use]
    pub fn mime_type(self) -> &'static str {
        match self {
            Self::Doc => "application/vnd.google-apps.document",
            Self::Sheet => "application/vnd.google-apps.spreadsheet",
            Self::Slides => "application/vnd.google-apps.presentation",
        }
    }

    /// Lowercase tag used in serialized payloads + the manifest
    /// validator's `permitted_doc_kinds` config entry.
    #[must_use]
    pub fn as_lower_str(self) -> &'static str {
        match self {
            Self::Doc => "doc",
            Self::Sheet => "sheet",
            Self::Slides => "slides",
        }
    }
}

/// Default `[document_gsuite].permitted_doc_kinds` value: all three.
/// Operators may restrict (e.g. `["Doc"]` for text-only verticals).
pub const PERMITTED_DOC_KINDS_DEFAULT: &[&str] = &["Doc", "Sheet", "Slides"];

/// Opaque Drive file id (the `id` field returned by `files.create`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GsuiteDocId(pub String);

/// One typed row of a Sheet write. Cell types re-use the CSV
/// executor's [`CsvColumnSchema`] so coercion is identical across the
/// two executors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GsuiteSheetRow {
    /// Per-cell JSON values keyed by header name. Mirrors the CSV
    /// writer's row shape.
    pub cells: std::collections::HashMap<String, serde_json::Value>,
}

/// One slide spec inside a Slides write request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GsuiteSlideSpec {
    /// Slide title (rendered as the title placeholder text).
    pub title: String,
    /// Optional body text under the title placeholder. Empty string
    /// means no body insert.
    #[serde(default)]
    pub body: String,
    /// Optional HTTPS image URL. Must respond `200` to a HEAD probe
    /// and carry an `image/*` mime type. Rejected at preflight
    /// otherwise.
    #[serde(default)]
    pub image_url: Option<String>,
}

/// Caller-supplied request body. The three doc-kind fields
/// (`doc_body_text` / `sheet_schema` + `sheet_rows` / `slides`) are
/// shaped per-kind; the unused fields for a given kind are ignored.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GsuiteWriteRequest {
    /// Which Google-native document kind to create.
    pub kind: GsuiteDocKind,
    /// Title of the new doc (also the Drive filename).
    pub title: String,
    /// Drive folder id to land the new doc in. `None` → service-
    /// account's "My Drive" root.
    #[serde(default)]
    pub drive_folder_id: Option<String>,
    /// `Doc` body text. Inserted at index 1 via a single `insertText`
    /// request. `Sheet` / `Slides` ignore this field.
    #[serde(default)]
    pub doc_body_text: Option<String>,
    /// `Sheet` column schema (header + typed-cell coercion). Mirrors
    /// the CSV executor's schema. `Doc` / `Slides` ignore.
    #[serde(default)]
    pub sheet_schema: Option<Vec<CsvColumnSchema>>,
    /// `Sheet` rows (typed per `sheet_schema`). `Doc` / `Slides` ignore.
    #[serde(default)]
    pub sheet_rows: Vec<GsuiteSheetRow>,
    /// `Slides` slides specs. `Doc` / `Sheet` ignore.
    #[serde(default)]
    pub slides: Vec<GsuiteSlideSpec>,
}

/// One rejection surfaced in [`crate::GsuiteWriteReport::rejections`].
/// Mirrors the CSV writer's per-row rejection shape so downstream
/// audit tooling can read both executor reports with one schema.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GsuiteRejection {
    /// Zero-based index of the offending row / slide.
    pub index: u32,
    /// Optional field / column name attached to the rejection.
    pub field: Option<String>,
    /// One-line human description.
    pub message: String,
}

/// Sandbox / preflight rejection separate from the per-row
/// [`GsuiteRejection`] surface. Returned as a hard error from
/// [`crate::DocumentGsuite::write`].
#[derive(Debug, Error)]
pub enum GsuiteSandboxError {
    /// Doc title was empty after trimming whitespace. Drive accepts
    /// empty titles but they make the resulting doc anonymous in
    /// search results, which we treat as misuse.
    #[error("doc title must not be empty")]
    EmptyTitle,
    /// `kind` is set to a doc kind not in `permitted_doc_kinds`.
    #[error("doc kind '{0}' is not permitted by the operator config")]
    KindNotPermitted(String),
    /// Slides `image_url` failed validation (not HTTPS, HEAD probe
    /// non-200, mime not `image/*`).
    #[error("slide {slide_index} image_url rejected: {reason}")]
    InvalidImageUrl {
        /// Zero-based index of the slide carrying the bad URL.
        slide_index: u32,
        /// Reason the URL was rejected.
        reason: String,
    },
}

impl From<GsuiteSandboxError> for ArgoError {
    fn from(e: GsuiteSandboxError) -> Self {
        ArgoError::Schema(format!("document-gsuite sandbox: {e}"))
    }
}
