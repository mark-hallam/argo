//! Unit coverage for the RFC 4180 writer + sandbox primitive.

use crate::sandbox::CsvSandboxError;
use crate::types::{
    CsvColumnKind, CsvColumnSchema, LineTerminator, RejectionReason,
};
use crate::{CsvWriteRequest, DocumentCsv};
use serde_json::json;
use std::collections::HashMap;
use std::path::PathBuf;
use tempfile::TempDir;

fn schema_basic() -> Vec<CsvColumnSchema> {
    vec![
        CsvColumnSchema {
            header: "a".into(),
            kind: CsvColumnKind::Text,
            required: false,
            max_len: None,
        },
        CsvColumnSchema {
            header: "b".into(),
            kind: CsvColumnKind::Text,
            required: false,
            max_len: None,
        },
    ]
}

fn row(pairs: &[(&str, serde_json::Value)]) -> HashMap<String, serde_json::Value> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_string(), v.clone()))
        .collect()
}

fn base_req(
    root: &TempDir,
    name: &str,
    schema: Vec<CsvColumnSchema>,
    rows: Vec<HashMap<String, serde_json::Value>>,
) -> CsvWriteRequest {
    CsvWriteRequest {
        output_root: root.path().to_path_buf(),
        path: PathBuf::from(name),
        schema,
        rows,
        include_header: true,
        line_terminator: LineTerminator::Crlf,
        quote_minimal: true,
        write_bom: false,
    }
}

#[test]
fn rfc_4180_roundtrip_quotes_comma_quote_cr_lf() {
    let root = TempDir::new().unwrap();
    let req = base_req(
        &root,
        "out.csv",
        schema_basic(),
        vec![row(&[
            ("a", json!("comma,inside")),
            ("b", json!("quote\"inside\nand newline")),
        ])],
    );
    let report = DocumentCsv::new().write(req).unwrap();
    let bytes = std::fs::read(&report.path).unwrap();
    // Round-trip via the csv crate's reader.
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(bytes.as_slice());
    let record = reader.records().next().unwrap().unwrap();
    assert_eq!(record.get(0).unwrap(), "comma,inside");
    assert_eq!(record.get(1).unwrap(), "quote\"inside\nand newline");
}

#[test]
fn quote_required_on_special_chars() {
    let root = TempDir::new().unwrap();
    let req = base_req(
        &root,
        "out.csv",
        schema_basic(),
        vec![row(&[("a", json!("has,comma")), ("b", json!("plain"))])],
    );
    let report = DocumentCsv::new().write(req).unwrap();
    let body = std::fs::read_to_string(&report.path).unwrap();
    assert!(body.contains("\"has,comma\""));
    assert!(body.contains(",plain\r\n"));
}

#[test]
fn quote_all_mode_quotes_every_field_including_header() {
    let root = TempDir::new().unwrap();
    let mut req = base_req(
        &root,
        "out.csv",
        schema_basic(),
        vec![row(&[("a", json!("x")), ("b", json!("y"))])],
    );
    req.quote_minimal = false;
    let report = DocumentCsv::new().write(req).unwrap();
    let body = std::fs::read_to_string(&report.path).unwrap();
    assert_eq!(body, "\"a\",\"b\"\r\n\"x\",\"y\"\r\n");
}

#[test]
fn type_coerce_six_kinds() {
    let root = TempDir::new().unwrap();
    let schema = vec![
        CsvColumnSchema { header: "t".into(), kind: CsvColumnKind::Text, required: false, max_len: None },
        CsvColumnSchema { header: "i".into(), kind: CsvColumnKind::Integer, required: false, max_len: None },
        CsvColumnSchema { header: "d".into(), kind: CsvColumnKind::Decimal, required: false, max_len: None },
        CsvColumnSchema { header: "dt".into(), kind: CsvColumnKind::Date, required: false, max_len: None },
        CsvColumnSchema { header: "b".into(), kind: CsvColumnKind::Boolean, required: false, max_len: None },
        CsvColumnSchema { header: "e".into(), kind: CsvColumnKind::EmailAddress, required: false, max_len: None },
    ];
    let rows = vec![row(&[
        ("t", json!("hi")),
        ("i", json!(42)),
        ("d", json!("3.14")),
        ("dt", json!("2026-05-11T12:00:00Z")),
        ("b", json!(true)),
        ("e", json!("user@example.com")),
    ])];
    let report = DocumentCsv::new()
        .write(base_req(&root, "out.csv", schema, rows))
        .unwrap();
    let body = std::fs::read_to_string(&report.path).unwrap();
    let lines: Vec<&str> = body.split("\r\n").collect();
    assert_eq!(lines[1], "hi,42,3.14,2026-05-11,true,user@example.com");
    assert_eq!(report.rows_written, 1);
    assert_eq!(report.rows_rejected, 0);
}

#[test]
fn missing_required_rejects_row_without_aborting_rest() {
    let root = TempDir::new().unwrap();
    let schema = vec![
        CsvColumnSchema { header: "a".into(), kind: CsvColumnKind::Text, required: true, max_len: None },
        CsvColumnSchema { header: "b".into(), kind: CsvColumnKind::Text, required: false, max_len: None },
    ];
    let rows = vec![
        row(&[("a", json!("ok")), ("b", json!("v1"))]),
        row(&[("b", json!("missing-a"))]), // rejected
        row(&[("a", json!("ok2")), ("b", json!("v2"))]),
    ];
    let report = DocumentCsv::new()
        .write(base_req(&root, "out.csv", schema, rows))
        .unwrap();
    assert_eq!(report.rows_written, 2);
    assert_eq!(report.rows_rejected, 1);
    assert_eq!(report.rejections.len(), 1);
    assert_eq!(report.rejections[0].row_index, 1);
    assert!(matches!(report.rejections[0].reason, RejectionReason::MissingRequired));
    let body = std::fs::read_to_string(&report.path).unwrap();
    let lines: Vec<&str> = body.split("\r\n").filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 3); // header + 2 surviving rows
}

#[test]
fn max_len_exceeded_rejects_row() {
    let root = TempDir::new().unwrap();
    let schema = vec![CsvColumnSchema {
        header: "a".into(),
        kind: CsvColumnKind::Text,
        required: false,
        max_len: Some(4),
    }];
    let rows = vec![
        row(&[("a", json!("ok"))]),
        row(&[("a", json!("toolongvalue"))]),
    ];
    let report = DocumentCsv::new()
        .write(base_req(&root, "out.csv", schema, rows))
        .unwrap();
    assert_eq!(report.rows_written, 1);
    assert_eq!(report.rows_rejected, 1);
    assert!(matches!(
        report.rejections[0].reason,
        RejectionReason::MaxLenExceeded
    ));
}

#[test]
fn path_traversal_variants_are_rejected() {
    let root = TempDir::new().unwrap();
    // (a) parent traversal
    let mut req = base_req(&root, "../escape.csv", schema_basic(), vec![]);
    let err = DocumentCsv::new().write(req).unwrap_err();
    assert!(format!("{err}").contains("sandbox"));

    // (b) absolute path outside root
    req = base_req(&root, "/tmp/totally-outside-root.csv", schema_basic(), vec![]);
    req.path = PathBuf::from("/tmp/totally-outside-the-root-xyz.csv");
    let err = DocumentCsv::new().write(req).unwrap_err();
    assert!(format!("{err}").contains("sandbox"));

    // (c) symlink outside root (Unix only).
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        let outside = TempDir::new().unwrap();
        let outside_target = outside.path().join("treasure.csv");
        std::fs::write(&outside_target, b"hi").unwrap();
        let link_in_root = root.path().join("evil-link.csv");
        symlink(&outside_target, &link_in_root).unwrap();
        let req = base_req(&root, "evil-link.csv", schema_basic(), vec![]);
        let err = DocumentCsv::new().write(req).unwrap_err();
        assert!(format!("{err}").contains("sandbox"));
    }
}

#[test]
fn empty_rows_writes_header_only() {
    let root = TempDir::new().unwrap();
    let report = DocumentCsv::new()
        .write(base_req(&root, "out.csv", schema_basic(), vec![]))
        .unwrap();
    let body = std::fs::read_to_string(&report.path).unwrap();
    assert_eq!(body, "a,b\r\n");
    assert_eq!(report.rows_written, 0);
}

#[test]
fn sha256_deterministic_across_runs_same_input() {
    let root1 = TempDir::new().unwrap();
    let root2 = TempDir::new().unwrap();
    let rows = vec![
        row(&[("a", json!("hello, world")), ("b", json!("x"))]),
        row(&[("a", json!("plain")), ("b", json!(""))]),
    ];
    let r1 = DocumentCsv::new()
        .write(base_req(&root1, "a.csv", schema_basic(), rows.clone()))
        .unwrap();
    let r2 = DocumentCsv::new()
        .write(base_req(&root2, "a.csv", schema_basic(), rows))
        .unwrap();
    assert_eq!(r1.sha256, r2.sha256);
    assert_eq!(r1.bytes_written, r2.bytes_written);
}

#[test]
fn sandbox_error_maps_to_schema_argoerror() {
    use argo_types::ArgoError;
    let err = CsvSandboxError::SandboxViolation("../boom".into());
    let mapped: ArgoError = err.into();
    assert!(matches!(mapped, ArgoError::Schema(_)));
}
