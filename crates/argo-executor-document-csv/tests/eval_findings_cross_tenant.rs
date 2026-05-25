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

/// **F-5 P2 regression** — the configured root remains an operator
/// root, but the writer auto-prefixes a tenant segment under it.
#[test]
fn csv_default_output_root_is_tenant_scoped_at_resolution() {
    let root = tempfile::tempdir().expect("tempdir");
    let report = DocumentCsv::for_tenant("tenant-a")
        .write(CsvWriteRequest {
            output_root: root.path().to_path_buf(),
            path: "out.csv".into(),
            schema: Vec::new(),
            rows: Vec::new(),
            include_header: false,
            line_terminator: LineTerminator::Lf,
            quote_minimal: true,
            write_bom: false,
        })
        .expect("tenant-scoped write");

    let root_canon = root.path().canonicalize().expect("canonical root");
    assert_eq!(DEFAULT_OUTPUT_ROOT, "~/.argo/exports/csv/");
    assert!(report.path.starts_with(root_canon.join("tenant-a")));
}

/// **F-5 P2 regression** — two tenants using the same relative path
/// land in disjoint tenant subtrees, so there is no in-root collision.
#[test]
fn csv_two_tenants_same_relative_path_get_distinct_output_roots() {
    let shared_root = tempfile::tempdir().expect("tempdir");

    let schema = vec![CsvColumnSchema {
        header: "k".into(),
        kind: CsvColumnKind::Text,
        required: true,
        max_len: None,
    }];

    let req_a = CsvWriteRequest {
        output_root: shared_root.path().to_path_buf(),
        path: "shared-output.csv".into(),
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
        path: "shared-output.csv".into(),
        schema,
        rows: vec![[("k".to_string(), serde_json::json!("B-secret"))]
            .into_iter()
            .collect()],
        include_header: true,
        line_terminator: LineTerminator::Lf,
        quote_minimal: true,
        write_bom: false,
    };

    let report_a = DocumentCsv::for_tenant("tenant-a")
        .write(req_a)
        .expect("tenant A write");
    let report_b = DocumentCsv::for_tenant("tenant-b")
        .write(req_b)
        .expect("tenant B write");

    assert_ne!(report_a.path, report_b.path);
    assert_eq!(report_a.path.file_name().unwrap(), "shared-output.csv");
    assert_eq!(report_b.path.file_name().unwrap(), "shared-output.csv");
    assert_ne!(
        report_a.path.parent().unwrap(),
        report_b.path.parent().unwrap()
    );

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
