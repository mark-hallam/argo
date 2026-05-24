//! LIVE surface-contract pin tests for **F-1** in `argo-executor-document-csv`.
//!
//! Companion to `surface_contract_pin_f1.rs` in `argo-executor-document/`.
//! See the v2 audit-report delta at
//! `docs/skycot/evals/scheduled/2026-05-23/eval-document-executor-cross-tenant-scoping-v2.md`.
//!
//! `CsvWriteRequest` / `CsvWriteReport` do not derive `Serialize`, so
//! the pins here use **exhaustive struct destructure** to lock the
//! field set — any new public field (including a `tenant_id`) will
//! make these tests fail at compile time.

use argo_executor_document_csv::{
    CsvColumnKind, CsvColumnSchema, CsvWriteReport, CsvWriteRequest, DocumentCsv, LineTerminator,
    RowRejection,
};
use argo_types::ArgoResult;
use std::path::PathBuf;

// =========================================================================
// LIVE PIN — DocumentCsv::write signature
// =========================================================================

/// **PIN F-1.a (csv)** — `DocumentCsv::write` takes
/// `(&self, CsvWriteRequest)` only; no tenant axis. Bound to a typed
/// fn-pointer so a tenant arg insertion fails to compile.
#[test]
fn pin_document_csv_write_signature_no_tenant_arg() {
    let _: fn(&DocumentCsv, CsvWriteRequest) -> ArgoResult<CsvWriteReport> = DocumentCsv::write;
}

/// **PIN F-1.b (csv)** — `DocumentCsv::new()` is a no-arg constructor.
/// The executor holds no per-tenant state; the surface advertises no
/// tenant binding at construction time.
#[test]
fn pin_document_csv_new_signature_no_tenant_arg() {
    let _: fn() -> DocumentCsv = DocumentCsv::new;
}

// =========================================================================
// LIVE PIN — CsvWriteRequest field set
// =========================================================================

/// **PIN F-1.c (csv)** — `CsvWriteRequest` field set is exactly
/// `{ output_root, path, schema, rows, include_header,
/// line_terminator, quote_minimal, write_bom }` — no `tenant_id`. The
/// exhaustive destructure is the compile-time guard: a new field would
/// trip Rust's `non_exhaustive_patterns` error and force the next
/// reviewer to think about whether the addition introduces a tenant
/// axis.
#[test]
fn pin_csv_write_request_field_set_has_no_tenant() {
    let req = CsvWriteRequest {
        output_root: PathBuf::from("/tmp/findings/output_root"),
        path: PathBuf::from("x.csv"),
        schema: vec![CsvColumnSchema {
            header: "k".into(),
            kind: CsvColumnKind::Text,
            required: true,
            max_len: None,
        }],
        rows: Vec::new(),
        include_header: true,
        line_terminator: LineTerminator::Lf,
        quote_minimal: true,
        write_bom: false,
    };
    // Exhaustive destructure: every existing field is named once, no
    // `..` rest pattern. A new field will fail to compile here.
    let CsvWriteRequest {
        output_root,
        path,
        schema,
        rows,
        include_header,
        line_terminator,
        quote_minimal,
        write_bom,
    } = req;
    // Silence unused-binding warnings without weakening the
    // structural guard.
    let _ = (
        output_root,
        path,
        schema,
        rows,
        include_header,
        line_terminator,
        quote_minimal,
        write_bom,
    );
}

/// **PIN F-1.d (csv)** — `CsvWriteReport` field set is exactly
/// `{ path, bytes_written, rows_written, rows_rejected, rejections,
/// sha256 }` — no tenant tag on the audit-ledger outcome row.
#[test]
fn pin_csv_write_report_field_set_has_no_tenant() {
    let report = CsvWriteReport {
        path: PathBuf::from("/tmp/findings/x.csv"),
        bytes_written: 0,
        rows_written: 0,
        rows_rejected: 0,
        rejections: Vec::<RowRejection>::new(),
        sha256: String::new(),
    };
    let CsvWriteReport {
        path,
        bytes_written,
        rows_written,
        rows_rejected,
        rejections,
        sha256,
    } = report;
    let _ = (
        path,
        bytes_written,
        rows_written,
        rows_rejected,
        rejections,
        sha256,
    );
}

// =========================================================================
// LIVE PIN — DEFAULT_OUTPUT_ROOT literal has no tenant interpolation
// =========================================================================

/// **PIN F-1.e (csv)** — the canonical CSV output root `~/.argo/exports/csv/`
/// contains no `{tenant_id}` template marker. A change of this literal
/// to introduce per-tenant subdirectories is a real fix — and would
/// flip this test.
#[test]
fn pin_csv_default_output_root_literal_has_no_tenant_marker() {
    let root = argo_executor_document_csv::DEFAULT_OUTPUT_ROOT;
    assert_eq!(root, "~/.argo/exports/csv/");
    assert!(!root.contains("{tenant_id}"));
    assert!(!root.contains("{tenant}"));
    assert!(!root.contains("{TENANT}"));
}

// =========================================================================
// POST-FIX — #[ignore]'d, un-ignore when the fix-brief lands
// =========================================================================

/// **POST-FIX F-1 (csv)** — companion to the pins above. Today
/// `CsvWriteRequest` has no `tenant_id` field; once
/// `mission-document-executor-tenant-stamp` (or
/// `mission-document-csv-output-root-per-tenant`) lands, the request
/// should carry a tenant axis.
///
/// **Un-ignore when**: the fix-brief adds the field. At that point
/// `pin_csv_write_request_field_set_has_no_tenant` will fail (the
/// exhaustive destructure misses the new field) and this test should
/// be promoted to a LIVE positive regression check.
#[test]
#[ignore = "POST-FIX: un-ignore when mission-document-executor-tenant-stamp lands; see surface_contract_pin_f1.rs header"]
fn post_fix_csv_write_request_carries_tenant_axis() {
    // Today there is no tenant_id field — a Debug repr check is the
    // weakest sufficient probe.
    let req = CsvWriteRequest {
        output_root: PathBuf::from("/tmp/findings"),
        path: PathBuf::from("x.csv"),
        schema: Vec::new(),
        rows: Vec::new(),
        include_header: false,
        line_terminator: LineTerminator::Lf,
        quote_minimal: true,
        write_bom: false,
    };
    let debug = format!("{req:?}");
    assert!(
        debug.contains("tenant_id"),
        "POST-FIX: CsvWriteRequest Debug should mention tenant_id once the fix-brief lands"
    );
}
