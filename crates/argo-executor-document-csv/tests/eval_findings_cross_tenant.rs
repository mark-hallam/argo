//! FINDINGS-ONLY tests for `eval-document-executor-cross-tenant-scoping`
//! (2026-05-23). Companion to the audit writeup at
//! `docs/skycot/evals/scheduled/2026-05-23/eval-document-executor-cross-tenant-scoping.md`.
//!
//! Codifies the CSV-specific finding:
//!
//! - **F-5** `DEFAULT_OUTPUT_ROOT = "~/.argo/exports/csv/"`. The CSV
//!   sandbox primitive guards `..` and absolute paths but offers no
//!   in-root tenant separation. Tenant A can write a CSV at any path
//!   under the root that tenant B would also see.
//!
//! See the audit writeup for the wider context; this file is the
//! executable companion for the CSV crate only.

use argo_executor_document_csv::{
    CsvColumnKind, CsvColumnSchema, CsvWriteRequest, DocumentCsv, LineTerminator,
    DEFAULT_OUTPUT_ROOT,
};

/// **F-5 P2** — `DEFAULT_OUTPUT_ROOT` is a single string with no
/// per-tenant interpolation point. Every tenant's CSV writes resolve
/// against the same root.
///
/// **Closed when**: `mission-document-csv-output-root-per-tenant`
/// adds either (a) a `DEFAULT_OUTPUT_ROOT_TEMPLATE` with a
/// `{tenant_id}` placeholder, or (b) a `Sandbox::open_for_tenant`
/// primitive that auto-prefixes the tenant id. At that point this
/// test's assertion on the literal string flips.
#[test]
#[ignore = "FINDINGS-ONLY: F-5 — DEFAULT_OUTPUT_ROOT has no tenant segment"]
fn csv_default_output_root_has_no_tenant_segment() {
    assert_eq!(DEFAULT_OUTPUT_ROOT, "~/.argo/exports/csv/");
    // The literal string contains no `{tenant_id}`, no `<tenant>`, no
    // `${TENANT}` placeholder. The finding is structural.
    assert!(!DEFAULT_OUTPUT_ROOT.contains("tenant"));
    assert!(!DEFAULT_OUTPUT_ROOT.contains("TENANT"));
    assert!(!DEFAULT_OUTPUT_ROOT.contains("{"));
}

/// **F-5 P2 — behaviour codification** — two notional tenants write
/// CSVs to the same `output_root`. The sandbox accepts both (it has
/// no tenant axis), and `DocumentCsv::write` produces files whose
/// resolved paths are siblings — i.e. tenant A's CSV is one
/// `read_to_string` away from tenant B.
///
/// **Closed when**: as above. The fix-brief will either reject the
/// second tenant's call (because the sandbox refuses to open the
/// shared root without a tenant id) or place the two outputs in
/// disjoint subtrees automatically.
#[test]
#[ignore = "FINDINGS-ONLY: F-5 — DocumentCsv writes from two tenants land siblings under shared output_root"]
fn csv_two_tenants_write_to_shared_output_root() {
    let shared_root = tempfile::tempdir().expect("tempdir");

    let schema = vec![CsvColumnSchema {
        header: "k".into(),
        kind: CsvColumnKind::Text,
        required: true,
        max_len: None,
    }];

    let req_a = CsvWriteRequest {
        output_root: shared_root.path().to_path_buf(),
        path: "tenant-a-output.csv".into(),
        schema: schema.clone(),
        rows: vec![[("k".to_string(), serde_json::json!("A-secret"))]
            .into_iter()
            .collect()],
        include_header: true,
        line_terminator: LineTerminator::Lf,
        quote_minimal: true,
        write_bom: false,
    };
    let req_b = CsvWriteRequest {
        output_root: shared_root.path().to_path_buf(),
        path: "tenant-b-output.csv".into(),
        schema,
        rows: vec![[("k".to_string(), serde_json::json!("B-secret"))]
            .into_iter()
            .collect()],
        include_header: true,
        line_terminator: LineTerminator::Lf,
        quote_minimal: true,
        write_bom: false,
    };

    let doc = DocumentCsv::new();
    let report_a = doc.write(req_a).expect("tenant A write");
    let report_b = doc.write(req_b).expect("tenant B write");

    // The audit finding: both resolved paths share the same parent
    // directory with no tenant segment in between. (The sandbox
    // canonicalises the root, so on macOS the resolved path lives
    // under `/private/var/folders/...` even when the input
    // `shared_root` is `/var/folders/...`; comparing parents avoids
    // the symlink surprise without weakening the finding.)
    assert_eq!(
        report_a.path.parent().unwrap(),
        report_b.path.parent().unwrap(),
        "F-5: tenant A + B CSVs land in the SAME directory — that is the finding"
    );

    // And the cross-tenant read is trivial.
    let a_contents = std::fs::read_to_string(&report_a.path).expect("read A");
    let b_contents = std::fs::read_to_string(&report_b.path).expect("read B");
    assert!(a_contents.contains("A-secret"));
    assert!(b_contents.contains("B-secret"));
}

/// **F-1 P2 (CSV variant)** — `DocumentCsv::write(&self, req:
/// CsvWriteRequest)` takes no tenant context; the request itself
/// carries `output_root`, `path`, schema, rows — but no `tenant_id`.
#[test]
#[ignore = "FINDINGS-ONLY: F-1 — DocumentCsv::write + CsvWriteRequest have no tenant axis"]
fn csv_write_signature_takes_no_tenant_context() {
    // Bind to fn pointer with no tenant in the signature.
    let _ = DocumentCsv::new();
    // CsvWriteRequest fields, surfaced for the next eval to check.
    let req = CsvWriteRequest {
        output_root: std::path::PathBuf::from("/tmp/findings"),
        path: "x.csv".into(),
        schema: Vec::new(),
        rows: Vec::new(),
        include_header: false,
        line_terminator: LineTerminator::Lf,
        quote_minimal: true,
        write_bom: false,
    };
    // No tenant field on the public struct — listed exhaustively
    // here so a future addition trips this test on compile.
    let CsvWriteRequest {
        output_root: _,
        path: _,
        schema: _,
        rows: _,
        include_header: _,
        line_terminator: _,
        quote_minimal: _,
        write_bom: _,
    } = req;
}
