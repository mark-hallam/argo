//! Wiki / web-URL intake for `POST /api/v1/genesis/ingest-document`
//! (Genesis Phase-2 multi-modal onboarding).
//!
//! The operator pastes a wiki / docs URL; we fetch it and ingest the page text
//! as an onboarding document. The fetch goes through
//! [`argo_genesis::safe_fetch_text`], the **SSRF-guarded** fetcher — NOT the
//! job-description `fetch_url`, which has no private-address blocking. See the
//! `safe_fetch` module docs for the threat model; the security review this brief
//! calls out is enforced there (scheme allow-list, pre-connect address
//! blocking, redirect re-validation, body cap + timeout).

use argo_genesis::{safe_fetch_text, MAX_FILE_BYTES, MAX_TOTAL_BYTES};

/// Produce `(page_text, bytes_charged)` for one wiki-URL intake document.
///
/// `url` is the operator-supplied URL (carried in the document's `content`
/// field). Returns a friendly per-file error string (never panics) so a bad /
/// blocked URL is reported "skipped" while the rest of the batch ingests.
pub(super) async fn fetch_wiki(url: &str, total_so_far: usize) -> Result<(String, usize), String> {
    let url = url.trim();
    if url.is_empty() {
        return Err("That wiki URL looks empty — skipped.".to_string());
    }

    let text = safe_fetch_text(url)
        .await
        .map_err(|e| format!("Couldn't fetch the URL — skipped ({e})"))?;

    let size = text.len();
    if size > MAX_FILE_BYTES {
        return Err(format!(
            "fetched page is {} MB, over the {}-MB per-file limit — skipped.",
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
    Ok((text, size))
}

#[cfg(test)]
#[path = "routes_genesis_ingest_url_tests.rs"]
mod tests;
