//! Image / screenshot intake for `POST /api/v1/genesis/ingest-document`
//! (Genesis Phase-2 multi-modal onboarding).
//!
//! An operator can drop a screenshot (an escalation board, an org chart, a
//! dashboard) into onboarding. We do **not** ship a fake OCR engine — that
//! would be theater ([DEC-007]). Instead the gradeable onboarding text is the
//! operator-supplied **caption / description** of the image, stamped with a
//! provenance descriptor (filename, byte size, and a SHA-256 of the bytes so
//! the image is tamper-evident and a future real vision/OCR pass can be matched
//! back to exactly these bytes).
//!
//! This keeps the modality honest: the teammate learns from what the operator
//! actually said the image contains, and the comprehension floor
//! (`MIN_SOP_CHARS`) rejects an image with no usable description so we never
//! persist an empty "I learned from a picture" claim.

use argo_genesis::{MAX_FILE_BYTES, MAX_TOTAL_BYTES};
use base64::{engine::general_purpose, Engine as _};
use sha2::{Digest, Sha256};

/// Produce `(gradeable_text, bytes_charged)` for one image-intake document.
///
/// `content` is base64 image bytes; `caption` is the operator's description of
/// what the image shows. Returns a friendly per-file error string (never
/// panics) so a bad image is reported "skipped" while the batch continues.
pub(super) fn describe_image(
    content: &str,
    caption: &str,
    filename: &str,
    total_so_far: usize,
) -> Result<(String, usize), String> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Err("That image looks empty — skipped.".to_string());
    }
    let bytes = general_purpose::STANDARD
        .decode(trimmed.as_bytes())
        .map_err(|e| format!("image base64 decode failed — skipped ({e})"))?;
    let size = bytes.len();
    if size > MAX_FILE_BYTES {
        return Err(format!(
            "image is {} MB, over the {}-MB per-file limit — skipped.",
            size / (1024 * 1024),
            MAX_FILE_BYTES / (1024 * 1024)
        ));
    }
    if total_so_far + size > MAX_TOTAL_BYTES {
        return Err(format!(
            "this upload would exceed the {}-MB total onboarding limit — skipped.",
            MAX_TOTAL_BYTES / (1024 * 1024)
        ));
    }

    let caption = caption.trim();
    if caption.is_empty() {
        return Err(
            "Add a short description of what this image shows so the teammate can learn from it — skipped."
                .to_string(),
        );
    }

    let digest = hex_sha256(&bytes);
    let text = format!(
        "Image onboarding reference: {filename} ({size} bytes, sha256 {digest}).\n\n\
         Operator description of the image: {caption}",
    );
    Ok((text, size))
}

/// Lowercase hex SHA-256 of the image bytes (provenance / tamper-evidence).
fn hex_sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    hex_encode(&digest)
}

/// Minimal lowercase-hex encoder (avoids pulling another dep at the call site).
fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

#[cfg(test)]
#[path = "routes_genesis_ingest_image_tests.rs"]
mod tests;
