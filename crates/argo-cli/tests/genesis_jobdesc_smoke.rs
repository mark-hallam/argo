//! End-to-end smoke for `argo genesis from-job-description`.
//!
//! Mission `mission-genesis-job-description-ingest` (#64). Builds
//! a minimal PDF in-process (same routine the argo-genesis unit
//! tests use) and feeds it to the CLI through the file system.
//! Asserts the pipeline produced a non-empty manifest in
//! `--json` mode.

use std::path::Path;
use std::process::{Command, Output};

fn argo_bin() -> &'static str {
    env!("CARGO_BIN_EXE_argo")
}

fn run(workspace: &Path, home: &Path, args: &[&str]) -> Output {
    Command::new(argo_bin())
        .current_dir(workspace)
        .env("ARGO_HOME", home)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("spawn argo {args:?}: {e}"))
}

fn assert_ok(out: &Output, label: &str) {
    assert!(
        out.status.success(),
        "{label} failed: stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
}

#[test]
fn from_job_description_pdf_produces_non_empty_manifest() {
    let workspace = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let pdf_path = workspace.path().join("jd.pdf");
    std::fs::write(
        &pdf_path,
        make_minimal_pdf("Be my marketing intern, run weekly campaigns to cold leads."),
    )
    .unwrap();

    let out = run(
        workspace.path(),
        home.path(),
        &[
            "genesis",
            "from-job-description",
            pdf_path.to_str().unwrap(),
            "--json",
        ],
    );
    assert_ok(&out, "genesis from-job-description --json");
    let envelope: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "stdout was not JSON: {e}\nstdout={}\nstderr={}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr),
        )
    });
    let action_count = envelope["action_count"].as_u64().unwrap_or(0);
    assert!(action_count > 0, "expected non-empty manifest: {envelope}");
    assert!(
        envelope["vertical_id"]
            .as_str()
            .unwrap_or("")
            .starts_with("marketing-intern-"),
        "expected marketing-intern archetype: {envelope}"
    );
}

#[test]
fn from_job_description_raw_text_works_too() {
    let workspace = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let out = run(
        workspace.path(),
        home.path(),
        &[
            "genesis",
            "from-job-description",
            "Be my recruiter screener, qualify inbound CVs.",
            "--json",
        ],
    );
    assert_ok(&out, "genesis from-job-description text --json");
    let envelope: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(
        envelope["vertical_id"]
            .as_str()
            .unwrap_or("")
            .starts_with("recruiter-screener-"),
        "expected recruiter-screener archetype: {envelope}"
    );
}

/// Minimal PDF 1.4 writer — text-content stream only, no font
/// metrics, just enough for `pdf-extract` to surface the literal
/// string. Mirrors the helper in the argo-genesis unit tests.
fn make_minimal_pdf(text: &str) -> Vec<u8> {
    let escaped = text
        .replace('\\', "\\\\")
        .replace('(', "\\(")
        .replace(')', "\\)");
    let mut out = Vec::new();
    let mut offsets: Vec<usize> = Vec::with_capacity(5);
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
          /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>",
    );
    let stream = format!("BT /F1 24 Tf 100 700 Td ({escaped}) Tj ET");
    let body = format!(
        "<< /Length {len} >>\nstream\n{stream}\nendstream",
        len = stream.len(),
    );
    push_obj(&mut out, &mut offsets, body.as_bytes());
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

fn push_obj(out: &mut Vec<u8>, offsets: &mut Vec<usize>, body: &[u8]) {
    let id = offsets.len() + 1;
    offsets.push(out.len());
    out.extend_from_slice(format!("{id} 0 obj\n").as_bytes());
    out.extend_from_slice(body);
    out.extend_from_slice(b"\nendobj\n");
}
