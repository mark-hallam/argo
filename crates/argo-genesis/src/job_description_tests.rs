//! Unit tests for [`crate::job_description`].
//!
//! Six required cases per the brief:
//!   1. pdf-input-extracts-text
//!   2. text-input-passes-through
//!   3. url-input-fetches
//!   4. empty-text-errors
//!   5. pdf-image-only-errors-with-hint
//!   6. synthesize-charter-falls-through-to-existing-pipeline
//!
//! PDF fixtures are constructed by [`make_minimal_pdf`] —
//! a hand-rolled PDF 1.4 writer that emits exactly the bytes
//! pdf-extract needs to surface the text we feed in. Keeps the
//! tests fixture-free (no binary blobs checked into the repo).

use super::{genesis_from_job_description, JobDescInput};

#[tokio::test]
async fn pdf_input_extracts_text_and_drives_pipeline() {
    let pdf = make_minimal_pdf(
        "Be my marketing intern, run weekly campaigns to cold leads",
    );
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("jd.pdf");
    std::fs::write(&path, &pdf).unwrap();

    let draft = genesis_from_job_description(JobDescInput::Pdf(path))
        .await
        .expect("pdf input pipes through");
    assert!(
        !draft.synthesized_charter.trim().is_empty(),
        "extracted text must be non-empty"
    );
    assert!(draft.manifest.vertical.id.starts_with("marketing-intern-"));
    assert!(!draft.eval_seed.is_empty());
}

#[tokio::test]
async fn text_input_passes_through_identity() {
    let charter = "Be my recruiter screener, screen inbound CVs.";
    let draft = genesis_from_job_description(JobDescInput::Text(charter.into()))
        .await
        .expect("text input pipes through");
    // v1 synthesise = identity. When the LLM lands this becomes
    // a sub-string assertion instead.
    assert_eq!(draft.synthesized_charter, charter);
    assert!(draft.manifest.vertical.id.starts_with("recruiter-screener-"));
}

#[tokio::test]
async fn url_input_fetches_and_drives_pipeline() {
    // Spin a one-shot local HTTP server so we don't reach the
    // public internet from a unit test.
    let body = "Be my sales SDR, qualify inbound leads on Slack and email.";
    let port = serve_once_text(body).await;
    let url = format!("http://127.0.0.1:{port}/");
    let draft = genesis_from_job_description(JobDescInput::Url(url))
        .await
        .expect("url input fetches");
    assert_eq!(draft.synthesized_charter.trim(), body);
    assert!(draft.manifest.vertical.id.starts_with("sales-sdr-"));
}

#[tokio::test]
async fn empty_text_returns_validation_error() {
    let err = genesis_from_job_description(JobDescInput::Text("   \n  ".into()))
        .await
        .expect_err("empty text rejects");
    let msg = err.to_string();
    assert!(
        msg.contains("empty") || msg.contains("image-only") || msg.contains("OCR"),
        "expected image-only hint, got: {msg}"
    );
}

#[tokio::test]
async fn pdf_image_only_returns_validation_error_with_hint() {
    // A PDF with no text content surfaces the same image-only hint.
    let pdf = make_minimal_pdf_without_text();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("img.pdf");
    std::fs::write(&path, &pdf).unwrap();

    let err = genesis_from_job_description(JobDescInput::Pdf(path))
        .await
        .expect_err("image-only pdf rejects");
    let msg = err.to_string();
    assert!(
        msg.contains("image-only") || msg.contains("OCR") || msg.contains("empty"),
        "expected image-only hint, got: {msg}"
    );
}

#[tokio::test]
async fn synthesize_charter_falls_through_to_existing_pipeline() {
    // The brief requires the rest of the pipeline to be byte-
    // identical to `argo genesis from-charter <synthesized>`. We
    // assert that by comparing the JD-driven archetype + vertical-
    // id against the plan() output for the same charter string.
    let charter = "Be my ops on-call, page me on Slack when alerts fire.";
    let jd_draft = genesis_from_job_description(JobDescInput::Text(charter.into()))
        .await
        .expect("jd plan");
    let direct =
        crate::plan(charter, "worker").expect("direct plan");
    assert_eq!(jd_draft.archetype, direct.archetype);
    assert_eq!(jd_draft.manifest.vertical.id, direct.vertical_id);
}

// ─── helpers ───────────────────────────────────────────────────

/// Minimal PDF 1.4 writer. Returns a byte vector pdf-extract can
/// parse — sufficient for "extract this literal string" tests.
fn make_minimal_pdf(text: &str) -> Vec<u8> {
    // Escape () and \ in the PDF string literal.
    let escaped = text
        .replace('\\', "\\\\")
        .replace('(', "\\(")
        .replace(')', "\\)");

    let mut out = Vec::new();
    let mut offsets: Vec<usize> = Vec::with_capacity(5);
    out.extend_from_slice(b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n");

    push_obj(
        &mut out,
        &mut offsets,
        b"<< /Type /Catalog /Pages 2 0 R >>",
    );
    push_obj(
        &mut out,
        &mut offsets,
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
    );
    push_obj(
        &mut out,
        &mut offsets,
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
          /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>",
    );

    let stream_content = format!("BT /F1 24 Tf 100 700 Td ({escaped}) Tj ET");
    let stream_obj = format!(
        "<< /Length {len} >>\nstream\n{stream}\nendstream",
        len = stream_content.len(),
        stream = stream_content
    );
    push_obj(&mut out, &mut offsets, stream_obj.as_bytes());

    push_obj(
        &mut out,
        &mut offsets,
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
    );

    let xref_offset = out.len();
    out.extend_from_slice(b"xref\n0 6\n0000000000 65535 f \n");
    for o in &offsets {
        out.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n"
        )
        .as_bytes(),
    );
    out
}

/// PDF with no text-drawing operators — pdf-extract returns empty
/// or whitespace text, which the image-only path catches.
fn make_minimal_pdf_without_text() -> Vec<u8> {
    let mut out = Vec::new();
    let mut offsets: Vec<usize> = Vec::with_capacity(4);
    out.extend_from_slice(b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n");
    push_obj(&mut out, &mut offsets, b"<< /Type /Catalog /Pages 2 0 R >>");
    push_obj(
        &mut out,
        &mut offsets,
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
    );
    push_obj(
        &mut out,
        &mut offsets,
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
          /Resources << >> /Contents 4 0 R >>",
    );
    // Empty content stream — no BT/ET, no text.
    push_obj(
        &mut out,
        &mut offsets,
        b"<< /Length 0 >>\nstream\n\nendstream",
    );
    let xref_offset = out.len();
    out.extend_from_slice(b"xref\n0 5\n0000000000 65535 f \n");
    for o in &offsets {
        out.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size 5 /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n"
        )
        .as_bytes(),
    );
    out
}

fn push_obj(out: &mut Vec<u8>, offsets: &mut Vec<usize>, body: &[u8]) {
    let id = offsets.len() + 1;
    offsets.push(out.len());
    out.extend_from_slice(format!("{id} 0 obj\n").as_bytes());
    out.extend_from_slice(body);
    out.extend_from_slice(b"\nendobj\n");
}

/// Tiny one-shot HTTP server: accepts a single connection,
/// replies with the supplied body, closes. Returns the bound
/// port so the caller can build the URL.
async fn serve_once_text(body: &'static str) -> u16 {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        if let Ok((mut sock, _)) = listener.accept().await {
            let mut buf = [0u8; 1024];
            let _ = sock.read(&mut buf).await;
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\
                 Content-Length: {len}\r\n\r\n{body}",
                len = body.len()
            );
            let _ = sock.write_all(resp.as_bytes()).await;
            let _ = sock.shutdown().await;
        }
    });
    port
}
