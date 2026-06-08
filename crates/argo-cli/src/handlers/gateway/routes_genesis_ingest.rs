//! `POST /api/v1/genesis/ingest-document` — onboarding document upload + memory
//! ingest (Genesis Phase 2; memory [[genesis-is-ai-teammate-onboarding-studio]]).
//!
//! The operator drops SOPs / wikis / process docs into `/genesis` after the
//! teammate is drafted; each file is parsed to text, chunked, and persisted
//! tenant-scoped under the teammate's vertical directory so it boots already
//! knowing the material. This handler is the HTTP shim around
//! `argo_genesis::onboarding_documents` (chunking + disk) +
//! `argo_memory::onboarding_namespace` (the tenant-bound namespace contract).
//!
//! ## Request (JSON batch)
//!
//! ```json
//! {
//!   "vertical_id": "genesis-sam-ab12cd",
//!   "documents": [
//!     { "filename": "return-policy.pdf", "format": "pdf_base64", "content": "<base64>" },
//!     { "filename": "escalation.md",     "format": "text",       "content": "..." }
//!   ]
//! }
//! ```
//!
//! ## Phase-2 multi-modal intake (additive `format` values)
//!
//! The same batch endpoint accepts three more modalities, each persisted into
//! the SAME tenant-scoped onboarding namespace and gradeable by the quiz:
//!
//! - `"voice"` / `"voice_base64"` — a voice-narrated role intake. Supply either
//!   a `"transcript"` (the SPA's live `/voice/stt` already transcribed it, the
//!   production path) or base64 audio in `"content"` (transcribed via the
//!   `argo_voice_io` STT seam). See [`super::routes_genesis_ingest_voice`].
//! - `"image"` / `"image_base64"` — a screenshot. `"content"` is base64 image
//!   bytes; `"caption"` is the operator's description (the gradeable text, plus
//!   a SHA-256 provenance descriptor — no fake OCR). See
//!   [`super::routes_genesis_ingest_image`].
//! - `"wiki_url"` / `"url"` — a wiki/docs page. `"content"` is the URL, fetched
//!   via the **SSRF-safe** [`argo_genesis::safe_fetch_text`]. See
//!   [`super::routes_genesis_ingest_url`].
//!
//! ## Per-file lifecycle (mirrors the SPA's 4-phase progress bar)
//!
//! `uploaded → parsed → chunked → embedded`. A file that fails any phase
//! (corrupt PDF, image-only, empty, oversize) is reported `rejected` with a
//! friendly reason; the remaining files still ingest — no all-or-nothing.
//!
//! ## Tenant scoping (fail-closed)
//!
//! The teammate's directory is resolved with
//! [`super::genesis_tenant::genesis_vertical_dir_for_tenant`] against the
//! caller's OWN write-tenant (the same tenant the SSE writer used). A
//! `vertical_id` that belongs to a different tenant resolves to `None` → 404, so
//! tenant A can never write onboarding documents into tenant B's teammate.

use super::body_from_request;
use argo_audit::{AuditKind, AuditSink};
use argo_genesis::{
    extract_sop_pdf_text, ingest_onboarding_document_with_kind, write_onboarding_documents,
    OnboardingDocument, SourceKind, MAX_FILES, MAX_FILE_BYTES, MAX_TOTAL_BYTES,
};
use argo_memory::OnboardingNamespaceKey;
use argo_store::SessionStore;
use base64::{engine::general_purpose, Engine as _};
use serde::Serialize;
use std::sync::Arc;

/// All four per-file phases, reported on success so the SPA can settle the
/// progress bar to "embedded into memory".
const PHASES: [&str; 4] = ["uploaded", "parsed", "chunked", "embedded"];

/// Per-file ingest outcome surfaced to the SPA.
#[derive(Debug, Serialize)]
struct FileResult {
    filename: String,
    status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    chunks: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    phases: Vec<&'static str>,
}

impl FileResult {
    fn ingested(filename: String, chunks: usize) -> Self {
        Self {
            filename,
            status: "ingested",
            chunks: Some(chunks),
            reason: None,
            phases: PHASES.to_vec(),
        }
    }

    fn rejected(filename: String, reason: String) -> Self {
        Self {
            filename,
            status: "rejected",
            chunks: None,
            reason: Some(reason),
            phases: Vec::new(),
        }
    }
}

#[derive(Debug, Serialize)]
struct IngestResponse {
    results: Vec<FileResult>,
    ingested: usize,
    rejected: usize,
    labels: Vec<String>,
    namespace: String,
}

/// Handle one onboarding-document ingest request.
pub(super) async fn ingest_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let body = body_from_request(req);
    let parsed: serde_json::Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => return bad_request(&format!("parse body: {e}")),
    };

    let vertical_id = match parsed.get("vertical_id").and_then(|v| v.as_str()) {
        Some(s) if !s.trim().is_empty() => s.trim().to_string(),
        _ => return bad_request("`vertical_id` (string) is required"),
    };
    let docs_json = match parsed.get("documents").and_then(|v| v.as_array()) {
        Some(arr) if !arr.is_empty() => arr,
        _ => return bad_request("`documents` (non-empty array) is required"),
    };
    if docs_json.len() > MAX_FILES {
        return bad_request(&format!(
            "too many documents ({}); the limit is {MAX_FILES} per onboarding session",
            docs_json.len()
        ));
    }

    // Resolve the teammate's directory scoped to the caller's OWN write-tenant —
    // fail-closed on a cross-tenant or unknown id.
    let tenant_id = super::routes_genesis_stream_tenant::resolve_write_tenant(store, req).await;
    let verticals_root = argo_vertical::default_dir();
    let Some(vertical_dir) = super::genesis_tenant::genesis_vertical_dir_for_tenant(
        &verticals_root,
        &tenant_id,
        &vertical_id,
    ) else {
        return not_found(&format!(
            "no teammate '{vertical_id}' for this workspace; describe the role first"
        ));
    };

    let (results, ingested) = ingest_documents(docs_json).await;

    // Persist the successfully-ingested batch tenant-scoped on disk.
    if !ingested.is_empty() {
        if let Err(e) = write_onboarding_documents(&vertical_dir, &ingested) {
            return server_error(&format!("could not persist onboarding documents: {e}"));
        }
    }

    let labels: Vec<String> = ingested.iter().map(|d| d.source_label.clone()).collect();
    let rejected = results.iter().filter(|r| r.status == "rejected").count();
    let namespace = OnboardingNamespaceKey::new(super::genesis_tenant::sanitize_tenant(&tenant_id))
        .qualified();

    emit_audit(store, req, &vertical_id, labels.len(), rejected).await;

    let resp = IngestResponse {
        ingested: labels.len(),
        rejected,
        labels,
        namespace,
        results,
    };
    (
        "200 OK",
        serde_json::to_string(&resp).unwrap_or_else(|_| "{}".to_string()),
    )
}

/// Extract + ingest each document in the batch, returning the per-file
/// results and the successfully-parsed [`OnboardingDocument`]s. Each file is
/// charged in order against a running session-byte cap (`extract_for_format`
/// enforces the per-file and cumulative limits).
async fn ingest_documents(
    docs_json: &[serde_json::Value],
) -> (Vec<FileResult>, Vec<OnboardingDocument>) {
    let mut results: Vec<FileResult> = Vec::with_capacity(docs_json.len());
    let mut ingested: Vec<OnboardingDocument> = Vec::new();
    let mut total_bytes: usize = 0;

    for doc in docs_json {
        let filename = doc
            .get("filename")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("uploaded-document")
            .to_string();
        let format = doc.get("format").and_then(|v| v.as_str()).unwrap_or("text");

        match extract_for_format(doc, format, &filename, total_bytes).await {
            Ok((text, bytes, kind)) => {
                total_bytes += bytes;
                match ingest_onboarding_document_with_kind(&text, &filename, kind) {
                    Ok(d) => {
                        let chunks = d.chunk_count();
                        ingested.push(d);
                        results.push(FileResult::ingested(filename, chunks));
                    }
                    Err(e) => results.push(FileResult::rejected(
                        filename,
                        format!("Couldn't parse — skipped ({e})"),
                    )),
                }
            }
            Err(reason) => results.push(FileResult::rejected(filename, reason)),
        }
    }
    (results, ingested)
}

/// Dispatch one document to its modality extractor, returning the gradeable
/// text, the bytes charged against the session cap, and the [`SourceKind`]
/// provenance tag. The text/PDF path stays inline ([`decode_one`]); the
/// Phase-2 multi-modal paths live in sibling modules:
///
/// - `voice_base64` / `audio` → transcript (via the voice STT seam), `Voice`.
/// - `image_base64` / `image` → caption + provenance descriptor, `Image`.
/// - `wiki_url` / `url` → SSRF-safe fetched page text, `WikiUrl`.
async fn extract_for_format(
    doc: &serde_json::Value,
    format: &str,
    filename: &str,
    total_so_far: usize,
) -> Result<(String, usize, SourceKind), String> {
    let content = doc.get("content").and_then(|v| v.as_str()).unwrap_or("");
    match format {
        "voice_base64" | "audio_base64" | "voice" | "audio" => {
            super::routes_genesis_ingest_voice::transcribe_voice(doc, total_so_far)
                .await
                .map(|(t, b)| (t, b, SourceKind::Voice))
        }
        "image_base64" | "image" => {
            let caption = doc.get("caption").and_then(|v| v.as_str()).unwrap_or("");
            super::routes_genesis_ingest_image::describe_image(
                content,
                caption,
                filename,
                total_so_far,
            )
            .map(|(t, b)| (t, b, SourceKind::Image))
        }
        "wiki_url" | "url" => super::routes_genesis_ingest_url::fetch_wiki(content, total_so_far)
            .await
            .map(|(t, b)| (t, b, SourceKind::WikiUrl)),
        _ => decode_one(content, format, total_so_far).map(|(t, b)| (t, b, SourceKind::Document)),
    }
}

/// Decode one document's `content` per `format`, enforcing the per-file +
/// running-total byte caps. Returns the extracted text + its decoded byte size.
fn decode_one(content: &str, format: &str, total_so_far: usize) -> Result<(String, usize), String> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Err("That file looks empty — skipped.".to_string());
    }
    let (text, bytes) = match format {
        "pdf_base64" => {
            let raw = general_purpose::STANDARD
                .decode(trimmed.as_bytes())
                .map_err(|e| format!("base64 decode failed — skipped ({e})"))?;
            let size = raw.len();
            if size > MAX_FILE_BYTES {
                return Err(over_file_cap(size));
            }
            let text = extract_sop_pdf_text(&raw).map_err(|e| {
                format!("Couldn't parse the PDF — skipped ({e})")
            })?;
            (text, size)
        }
        // "text" | "md" | anything else → treat as UTF-8 text verbatim.
        _ => {
            let size = trimmed.len();
            if size > MAX_FILE_BYTES {
                return Err(over_file_cap(size));
            }
            (trimmed.to_string(), size)
        }
    };
    if total_so_far + bytes > MAX_TOTAL_BYTES {
        return Err(format!(
            "this upload would exceed the {}-MB total onboarding limit — skipped.",
            MAX_TOTAL_BYTES / (1024 * 1024)
        ));
    }
    Ok((text, bytes))
}

fn over_file_cap(size: usize) -> String {
    format!(
        "file is {} MB, over the {}-MB per-file limit — skipped.",
        size / (1024 * 1024),
        MAX_FILE_BYTES / (1024 * 1024)
    )
}

fn bad_request(message: &str) -> (&'static str, String) {
    ("400 Bad Request", error_envelope("bad_request", message))
}

fn not_found(message: &str) -> (&'static str, String) {
    ("404 Not Found", error_envelope("not_found", message))
}

fn server_error(message: &str) -> (&'static str, String) {
    (
        "500 Internal Server Error",
        error_envelope("internal", message),
    )
}

fn error_envelope(code: &str, message: &str) -> String {
    serde_json::json!({ "error": { "code": code, "message": message } }).to_string()
}

/// Best-effort audit row — an audit hiccup never fails a successful ingest.
async fn emit_audit(
    store: Option<&Arc<SessionStore>>,
    req: &str,
    vertical_id: &str,
    ingested: usize,
    rejected: usize,
) {
    let Some(store) = store else { return };
    let payload = serde_json::json!({
        "endpoint": "genesis/ingest-document",
        "vertical_id": vertical_id,
        "documents_ingested": ingested,
        "documents_rejected": rejected,
    });
    let principal_id = super::routes_auth_principal::principal_id_for_audit(req);
    // Round-trip with the audit feed (canonical session/header/cwd
    // resolver), not the rate-limit config/anonymous tail. See
    // `resolve_tenant::resolve_audit_write_tenant`.
    let tenant = super::resolve_tenant::resolve_audit_write_tenant(req, store).await;
    let sink = AuditSink::new_with_tenant(Arc::clone(store), tenant);
    if let Err(e) = sink
        .write(None, AuditKind::HttpRequest, payload, principal_id)
        .await
    {
        tracing::warn!(
            target: "gateway::genesis_ingest",
            vertical_id = %vertical_id,
            error = %e,
            "genesis ingest audit write failed"
        );
    }
}

#[cfg(test)]
#[path = "routes_genesis_ingest_tests.rs"]
mod tests;
