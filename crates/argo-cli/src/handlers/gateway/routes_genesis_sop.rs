//! SOP-upload ingestion for the `/genesis` SSE handler (Genesis P6 — the moat).
//!
//! Extracted from `routes_genesis_stream.rs` to keep that file under the
//! per-file LOC ceiling. Turns the optional SOP document on the `POST
//! /api/v1/genesis` request into tenant-scoped ingested knowledge + a
//! doc-grounded comprehension quiz, written into the teammate's vertical
//! directory so the hire gate (which reads `<dir>/knowledge/`) can grade the
//! teammate against the SPECIFIC uploaded document.
//!
//! ## Flow
//!
//! ```text
//! request.sop_document (text | base64 PDF)
//!   → extract text (reuses argo_genesis::extract_sop_pdf_text — same pdf_extract
//!     path as the JD ingest, no parallel extractor)
//!   → ingest_sop()      (chunk + reject empty/image-only)
//!   → build_sop_quiz(role_text)  (doc-specific terms, role-overlap filtered)
//!   → write_sop_knowledge(<vertical_dir>/knowledge/)  (tenant-scoped on disk)
//! ```
//!
//! Tenant scoping is inherited from the vertical directory: the SSE handler
//! resolves `<verticals_root>/.genesis/<tenant_slug>/<vertical_id>/` per the
//! caller's own tenant segment (P5 tenant-match), and the knowledge lands under
//! it, so a different tenant can never read this SOP.

use argo_genesis::{
    build_sop_quiz, extract_sop_pdf_text, ingest_sop, write_sop_knowledge, GenesisPlan,
};
use base64::{engine::general_purpose, Engine as _};
use std::path::Path;

/// Cap on the decoded SOP body (bytes). A document larger than this is refused —
/// the moat ingests ONE playbook, not a corpus; mirrors the JD-ingest URL cap.
const SOP_BODY_CAP_BYTES: usize = 5 * 1024 * 1024;

/// Outcome of attempting to ingest the request's optional SOP.
pub(super) enum SopIngestOutcome {
    /// No SOP was supplied on the request — the SOP axis is not applicable.
    None,
    /// A SOP was ingested and persisted; carries the source label + the number
    /// of doc-grounded quiz questions derived (surfaced in the SSE payload).
    Ingested {
        /// The source document label (filename or generic fallback).
        source_label: String,
        /// How many doc-grounded comprehension questions were derived.
        quiz_len: usize,
    },
    /// A SOP was supplied but could not be ingested (empty / image-only /
    /// malformed / too large). Carries an operator-facing reason. The SSE
    /// handler surfaces this as a recoverable error rather than silently
    /// dropping the moat.
    Rejected(String),
}

/// Ingest the request's optional SOP into the teammate's vertical directory.
///
/// `sop_document` is the raw request field (plain text, or base64-encoded PDF
/// when `sop_format == Some("pdf_base64")`). `role_text` is the teammate's
/// charter (so the quiz can exclude role-overlapping terms — keeping the quiz
/// doc-specific). Returns [`SopIngestOutcome`]; `None` when no SOP was supplied.
///
/// Writes `<vertical_dir>/knowledge/sop.md` + `sop-quiz.toml` on success, so the
/// hire gate (SSE + the disk fence) grades the SAME ingested knowledge.
pub(super) fn ingest_request_sop(
    sop_document: Option<&str>,
    sop_format: Option<&str>,
    sop_filename: Option<&str>,
    role_text: &str,
    vertical_dir: &Path,
) -> SopIngestOutcome {
    let Some(raw) = sop_document.map(str::trim).filter(|s| !s.is_empty()) else {
        return SopIngestOutcome::None;
    };
    // Decode to text (PDF base64 → extracted text, else verbatim).
    let text = match sop_format.map(str::trim) {
        Some("pdf_base64") => match decode_pdf(raw) {
            Ok(t) => t,
            Err(e) => return SopIngestOutcome::Rejected(e),
        },
        _ => raw.to_string(),
    };
    if text.len() > SOP_BODY_CAP_BYTES {
        return SopIngestOutcome::Rejected(format!(
            "SOP body {} bytes exceeds {SOP_BODY_CAP_BYTES}-byte cap",
            text.len()
        ));
    }
    let label = sop_filename
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("uploaded-sop");
    // Ingest (chunk + reject empty/image-only) — fail-closed with a precise hint.
    let knowledge = match ingest_sop(&text, label) {
        Ok(k) => k,
        Err(e) => return SopIngestOutcome::Rejected(e.to_string()),
    };
    let quiz = build_sop_quiz(&knowledge, role_text);
    if let Err(e) = write_sop_knowledge(vertical_dir, &knowledge, &quiz) {
        return SopIngestOutcome::Rejected(format!("could not persist ingested SOP: {e}"));
    }
    SopIngestOutcome::Ingested {
        source_label: knowledge.source_label,
        quiz_len: quiz.len(),
    }
}

/// The role text the SOP quiz keys its doc-specific filter off: the plan's
/// charter plus its action vocabulary. A salient SOP term that already appears
/// here can't distinguish "read the doc" from "knows the role", so the quiz
/// builder drops it — keeping the quiz genuinely doc-specific.
pub(super) fn role_text_for_plan(plan: &GenesisPlan) -> String {
    let mut out = plan.charter.clone();
    if let Some(actions) = plan.manifest.actions.as_ref() {
        for name in actions.actions.keys() {
            out.push(' ');
            out.push_str(name);
        }
    }
    out
}

/// Base64-decode a PDF upload and extract its text via the shared extractor.
fn decode_pdf(b64: &str) -> Result<String, String> {
    let bytes = general_purpose::STANDARD
        .decode(b64.as_bytes())
        .map_err(|e| format!("SOP pdf_base64 decode failed: {e}"))?;
    if bytes.len() > SOP_BODY_CAP_BYTES {
        return Err(format!(
            "SOP PDF {} bytes exceeds {SOP_BODY_CAP_BYTES}-byte cap",
            bytes.len()
        ));
    }
    extract_sop_pdf_text(&bytes).map_err(|e| e.to_string())
}

#[cfg(test)]
#[path = "routes_genesis_sop_tests.rs"]
mod tests;
